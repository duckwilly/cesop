#!/usr/bin/env bash
set -euo pipefail

rebuild=false
clean=false
sizes=()
for arg in "$@"; do
  if [ "$arg" = "--rebuild" ]; then
    rebuild=true
  elif [ "$arg" = "--clean" ]; then
    clean=true
  else
    sizes+=("$arg")
  fi
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
  tx_country="${CESOP_TRANSMITTING_COUNTRY:-auto}"

  if [ "$clean" = true ] && [ -d "$out_dir" ]; then
    rm -f "$out_dir"/*.xml "$out_dir"/validation_output.csv "$out_dir"/validation.csv
    rm -rf "$out_dir"/validation
  fi

  "$bin" generate \
    --scale "$size" \
    --output "$out_csv" \
    --psps "$gen_psps" \
    --multi-account-ratio "$gen_multi" \
    --non-eu-payee-ratio "$gen_non_eu"
  "$bin" preflight --input "$out_csv"
  "$bin" render --input "$out_csv" --output-dir "$out_dir" --transmitting-country "$tx_country"
  "$bin" validate --input "$out_dir" --output "$val_out" --java "$java_bin" --jar "$jar_path"

  if [ "${CESOP_DEMO_INVALID:-1}" = "1" ]; then
    invalid_csv="${out_csv%.csv}_invalid.csv"
    invalid_dir="${out_dir}_invalid"
    invalid_val="${invalid_dir}/validation.csv"

    "$bin" corrupt --input "$out_csv" --output "$invalid_csv"
    if ! "$bin" preflight --input "$invalid_csv"; then
      echo "Preflight failed as expected for invalid data"
    fi
    "$bin" render --input "$invalid_csv" --output-dir "$invalid_dir" --transmitting-country "$tx_country"
    if "$bin" validate --input "$invalid_dir" --output "$invalid_val" --java "$java_bin" --jar "$jar_path"; then
      echo "Unexpected: invalid data validated successfully"
    else
      echo "Validation failed as expected for invalid data"
    fi
  fi

done
