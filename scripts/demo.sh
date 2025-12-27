#!/usr/bin/env bash
set -euo pipefail

sizes=("$@")
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
if [ ! -x "$bin" ]; then
  needs_build=true
elif ! "$bin" --help | rg -q "render"; then
  needs_build=true
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

  "$bin" generate --scale "$size" --output "$out_csv"
  "$bin" render --input "$out_csv" --output-dir "$out_dir" --transmitting-country DE
  "$bin" validate --input "$out_dir" --output "$val_out" --java "$java_bin" --jar "$jar_path"

done
