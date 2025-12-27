# CESOP Demo CLI Flags

Use `--help` on any command to see the current defaults and options.

## `cesop-demo generate`
Generate synthetic payment data (auto-analyzes CSV output).

- `--scale <N>`: Target number of payment records. Default `1200`.
- `--seed <N>`: RNG seed for repeatable output. Default: random.
- `--output <PATH>`: Output file path. Default `data/synthetic/payments.csv`.

Example:
```sh
cesop-demo generate --scale 1200 --output data/synthetic/payments.csv
```
Scale example:
```sh
cesop-demo generate --scale 100000 --output data/synthetic/payments_100k.csv
```

## `cesop-demo analyze`
Analyze a generated CSV for cross-border payees above the threshold.

- `--input <PATH>`: Input CSV file. Default `data/synthetic/payments.csv`.
- `--threshold <N>`: Threshold for "over". Default `25`.
- `--include-refunds`: Include refunds in the count. Default `false`.

Example:
```sh
cesop-demo analyze --input data/synthetic/payments_2000.csv
```

## `cesop-demo render`
Render CESOP PaymentData XML from CSV input.

- `--input <PATH>`: Input CSV file. Default `data/synthetic/payments.csv`.
- `--output-dir <PATH>`: Output directory for XML files. Default `data/output`.
- `--transmitting-country <MS>`: Tax administration Member State. Default `DE`.

Example:
```sh
cesop-demo render --input data/synthetic/payments.csv --output-dir data/output
```

## `cesop-demo validate`
Run the CESOP Validation Module against an XML file.

- `--input <PATH>`: XML file or folder to validate. Default `data/output`.
- `--output <PATH>`: Optional file path to write the validation result XML.
- `--jar <PATH>`: Path to `cesop-vm-application-1.7.1.jar`.
- `--java <BIN>`: Java binary to use. Default `java`.

Example:
```sh
cesop-demo validate --input data/output/cesop_2025_Q4.xml --output data/output/validation.xml
```

## Logging environment variables
- `CESOP_LOG_LEVEL`: Log level (`trace`, `debug`, `info`, `warn`, `error`).
- `RUST_LOG`: Fallback log level if `CESOP_LOG_LEVEL` is not set.
- `CESOP_LOG_DIR`: Directory for log files. Set to `off` or `none` to disable file logs.

Example:
```sh
CESOP_LOG_LEVEL=info cesop-demo generate --scale 900
```
