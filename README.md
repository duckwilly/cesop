# CESOP Reporting Workbench

A focused demo pipeline that generates synthetic payment data, applies the CESOP
eligibility logic (>25 cross-border payments per payee per Member State per
quarter), renders PaymentData XML, and validates it using the official CESOP
Validation Module.

## Highlights
- End-to-end flow: synthetic CSV -> CESOP XML -> validation result.
- Cross-border and threshold logic aligned to CESOP (per Member State and payee
  identifier).
- XML output shaped to XSD v6.00 (PaymentData v4.03).
- Validation wired to the official CESOP VM CLI.

## Features
- Scale-friendly generator (`--scale`) with reproducible seeds.
- Grouping by payee and quarter with the >25 rule per Member State.
- Deterministic XML writer with correct namespaces and ordering.
- Reports split by PSP and quarter with auto transmitting country support.
- Preflight validator for mandatory fields + syntax checks.
- Validation CLI integration with clear success/failure output.

## Tech Stack
- Rust (clap, csv, quick-xml, rand, chrono)
- Java (OpenJDK) for CESOP Validation Module

## Architecture Overview
- `src/generator.rs`: synthetic payment generation.
- `src/analysis.rs`: threshold analysis for cross-border payees.
- `src/cesop_xml.rs`: CSV -> XML transform and writer.
- `src/preflight.rs`: CSV preflight validation.
- `src/validation.rs`: CESOP VM CLI wrapper.

## Quickstart
Generate CSV, preflight, render XML, validate:
```sh
cargo run -- generate --scale 1200 --output data/synthetic/payments.csv
cargo run -- preflight --input data/synthetic/payments.csv
cargo run -- render --input data/synthetic/payments.csv --output-dir data/output
cargo run -- validate --input data/output --output data/output/validation.xml
```

Demo invalid data flow:
```sh
cargo run -- corrupt --input data/synthetic/payments.csv --output data/synthetic/payments_invalid.csv
cargo run -- preflight --input data/synthetic/payments_invalid.csv
cargo run -- correct --input data/synthetic/payments_invalid.csv --output data/synthetic/payments_corrected.csv
cargo run -- preflight --input data/synthetic/payments_corrected.csv
cargo run -- render --input data/synthetic/payments_corrected.csv --output-dir data/output_corrected
cargo run -- validate --input data/output_corrected --output data/output_corrected/validation.xml
```

Run the demo script (default sizes 25k/100k/1m):
```sh
./scripts/demo.sh
```

## Validation Module
The CESOP Validation Module jar is included under
`scripts/CESOP Validation Module/SDEV-CESOP-VM-v1.7.1/`. 

Only Java is required; `cesop-demo validate` uses the bundled jar by default:
```sh
cargo run -- validate \
  --input data/output \
  --output data/output/validation.xml
```

If Java is not on your PATH or you store the jar elsewhere, override it:
```sh
cargo run -- validate \
  --input data/output \
  --output data/output/validation.xml \
  --jar /path/to/cesop-vm-application.jar \
  --java /path/to/java
```

Environment overrides supported by `scripts/demo.sh`:
- `CESOP_BIN`: path to the compiled binary.
- `JAVA_BIN`: path to the Java runtime.
- `CESOP_VM_JAR`: path to the validation jar.

## Project Structure
```
src/
  analysis.rs      # threshold checks
  cesop_xml.rs     # CSV -> XML mapping and writer
  generator.rs     # synthetic data generator
  preflight.rs     # CSV preflight validator
  reference.rs     # shared country/identifier tables
  validation.rs    # CESOP VM CLI wrapper
scripts/
  demo.sh          # end-to-end demo runner
schemas/
  v6_00/           # CESOP XSDs (reference)
data/
  synthetic/       # generated CSVs
  output/          # rendered XML and validation results
```
