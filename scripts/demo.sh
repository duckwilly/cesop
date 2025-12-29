#!/usr/bin/env bash
set -euo pipefail

rebuild=false
clean=true
sizes=()
usage() {
  echo "Usage: ./scripts/demo.sh [--rebuild] [--clean|--no-clean] [sizes...]"
  echo "Examples:"
  echo "  ./scripts/demo.sh"
  echo "  ./scripts/demo.sh 100000"
  echo "  ./scripts/demo.sh --rebuild --no-clean 25000 100000"
}
for arg in "$@"; do
  case "$arg" in
    --rebuild)
      rebuild=true
      ;;
    --no-clean)
      clean=false
      ;;
    --clean)
      clean=true
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      if [[ "$arg" =~ ^[0-9]+$ ]]; then
        sizes+=("$arg")
      else
        echo "Unknown argument: $arg"
        usage
        exit 1
      fi
      ;;
  esac
done

if [ ${#sizes[@]} -eq 0 ]; then
  sizes=(25000 100000 1000000)
fi

bin="${CESOP_BIN:-}"
if [ -z "$bin" ]; then
  if [ -x "target/release/cesop-demo" ]; then
    bin="target/release/cesop-demo"
  else
    bin="target/debug/cesop-demo"
  fi
fi

needs_build=false
if [ "$rebuild" = true ]; then
  needs_build=true
fi
if [ ! -x "$bin" ]; then
  needs_build=true
else
  if command -v rg >/dev/null 2>&1; then
    if ! "$bin" --help | rg -q "render"; then
      needs_build=true
    fi
  else
    if ! "$bin" --help | grep -q "render"; then
      needs_build=true
    fi
  fi
fi

if [ "$needs_build" = true ]; then
  echo "Building cesop-demo (release)"
  cargo build --release
  bin="target/release/cesop-demo"
fi

EU_MEMBER_STATES=(
  AT BE BG HR CY CZ DK EE FI FR DE GR HU IE IT
  LV LT LU MT NL PL PT RO SK SI ES SE
)

resolve_license_count() {
  local size="$1"
  local override="${CESOP_LICENSED_COUNT:-${CESOP_DEMO_LICENSED_COUNT:-}}"
  if [ -n "$override" ]; then
    local count="$override"
    if [ "$count" -lt 1 ]; then
      count=1
    fi
    if [ "$count" -gt 27 ]; then
      count=27
    fi
    echo "$count"
    return
  fi

  if [ "$size" -le 10000 ]; then
    echo 6
    return
  fi
  if [ "$size" -ge 100000 ]; then
    echo 27
    return
  fi

  local delta=$((size - 10000))
  local count=$((6 + (delta * 21 + 45000) / 90000))
  if [ "$count" -lt 1 ]; then
    count=1
  fi
  if [ "$count" -gt 27 ]; then
    count=27
  fi
  echo "$count"
}

build_license_list() {
  local count="$1"
  local list=()
  local i=0
  for code in "${EU_MEMBER_STATES[@]}"; do
    list+=("$code")
    i=$((i + 1))
    if [ "$i" -ge "$count" ]; then
      break
    fi
  done
  local joined
  joined=$(IFS=,; echo "${list[*]}")
  echo "$joined"
}

java_bin="${JAVA_BIN:-}"
if [ -z "$java_bin" ]; then
  if [ -x "/opt/homebrew/opt/openjdk/bin/java" ]; then
    java_bin="/opt/homebrew/opt/openjdk/bin/java"
  else
    java_bin="java"
  fi
fi

jar_path="${CESOP_VM_JAR:-scripts/CESOP Validation Module/SDEV-CESOP-VM-v1.7.1/cesop-vm-application-1.7.1.jar}"

echo "Using binary: $bin"
echo "Using Java: $java_bin"
echo "Using VM jar: $jar_path"

for size in "${sizes[@]}"; do
  echo ""
  echo "=== Scale ${size} ==="
  out_csv="data/synthetic/payments_scale_${size}.csv"
  out_dir="data/output/scale_${size}"
  val_out="${out_dir}/validation.csv"
  gen_psps="${CESOP_GEN_PSPS:-1}"
  gen_multi="${CESOP_GEN_MULTI_ACCOUNT_RATIO:-0.15}"
  gen_non_eu="${CESOP_GEN_NON_EU_PAYEE_RATIO:-0.10}"
  gen_no_account="${CESOP_GEN_NO_ACCOUNT_RATIO:-0.02}"
  tx_country="${CESOP_TRANSMITTING_COUNTRY:-auto}"
  licensed_csv="${CESOP_LICENSED_COUNTRIES:-}"
  if [ -z "$licensed_csv" ]; then
    licensed_count=$(resolve_license_count "$size")
    licensed_csv=$(build_license_list "$licensed_count")
  fi

  if [ "$clean" = true ] && [ -d "$out_dir" ]; then
    rm -f "$out_dir"/*.xml "$out_dir"/validation_output.csv "$out_dir"/validation.csv
    rm -rf "$out_dir"/validation
  fi
  if [ "$clean" = true ] && [ -d "${out_dir}_corrected" ]; then
    rm -f "${out_dir}_corrected"/*.xml "${out_dir}_corrected"/validation_output.csv "${out_dir}_corrected"/validation.csv
    rm -rf "${out_dir}_corrected"/validation
  fi

  "$bin" generate \
    --scale "$size" \
    --output "$out_csv" \
    --psps "$gen_psps" \
    --multi-account-ratio "$gen_multi" \
    --non-eu-payee-ratio "$gen_non_eu" \
    --no-account-payee-ratio "$gen_no_account"
  "$bin" preflight --input "$out_csv"
  if [ -n "$licensed_csv" ]; then
    echo "Licensed Member States: $licensed_csv"
    "$bin" render --input "$out_csv" --output-dir "$out_dir" --licensed-countries "$licensed_csv"
  else
    "$bin" render --input "$out_csv" --output-dir "$out_dir" --transmitting-country "$tx_country"
  fi
  "$bin" validate --input "$out_dir" --output "$val_out" --java "$java_bin" --jar "$jar_path"

  if [ "${CESOP_DEMO_INVALID:-1}" = "1" ]; then
    invalid_csv="${out_csv%.csv}_invalid.csv"
    corrected_csv="${out_csv%.csv}_corrected.csv"
    corrected_dir="${out_dir}_corrected"
    corrected_val="${corrected_dir}/validation.csv"

    "$bin" corrupt --input "$out_csv" --output "$invalid_csv"
    if ! "$bin" preflight --input "$invalid_csv"; then
      echo "Preflight failed as expected for invalid data"
    fi
    "$bin" correct --input "$invalid_csv" --output "$corrected_csv"
    "$bin" preflight --input "$corrected_csv"
    if [ -n "$licensed_csv" ]; then
      "$bin" render --input "$corrected_csv" --output-dir "$corrected_dir" --licensed-countries "$licensed_csv"
    else
      "$bin" render --input "$corrected_csv" --output-dir "$corrected_dir" --transmitting-country "$tx_country"
    fi
    if "$bin" validate --input "$corrected_dir" --output "$corrected_val" --java "$java_bin" --jar "$jar_path"; then
      echo "Validation succeeded after correction"
    else
      echo "Unexpected: corrected data failed validation"
    fi
  fi

done
