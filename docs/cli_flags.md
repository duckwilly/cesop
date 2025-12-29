# CESOP Demo CLI Flags

Use `--help` on any command to see the current defaults and options.

## `cesop-demo generate`
Generate synthetic payment data (auto-analyzes CSV output).

- `--scale <N>`: Target number of payment records. Default `1200`.
- `--seed <N>`: RNG seed for repeatable output. Default: random.
- `--psps <N>`: Number of PSPs to simulate. Default `1`.
- `--multi-account-ratio <F>`: Share of payees with account identifier + BIC pairs. Default `0.15`.
- `--non-eu-payee-ratio <F>`: Share of payees outside the EU. Default `0.10`.
- `--no-account-payee-ratio <F>`: Share of payees with no account (Representative PSP flow). Default `0.02`.
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
- `--transmitting-country <MS|auto>`: Tax administration Member State. Default `auto`
  (derive from reporting PSP BIC).
- `--licensed-countries <CSV>`: Comma-separated list of transmitting Member States.
  When set, generate one report per country and assign payees to a transmitting
  Member State that matches the payee country when possible; otherwise fall
  back to the PSP home Member State (from the PSP BIC) or round-robin if needed.
  Overrides `--transmitting-country`.

Example:
```sh
cesop-demo render --input data/synthetic/payments.csv --output-dir data/output
```

Output files are named:
`cesop_<YEAR>_Q<QUARTER>_<MS>_<PSP_ID>.xml`

## `cesop-demo preflight`
Validate CSV input against mandatory field + syntax rules and reportability stats.

- `--input <PATH>`: Input CSV file. Default `data/synthetic/payments.csv`.
- `--threshold <N>`: Threshold for "over". Default `25`.
- `--include-refunds`: Include refunds in threshold counting. Default `false`.

## `cesop-demo corrupt`
Create an intentionally invalid CSV by injecting payee- and transaction-level
errors for demo purposes.

- `--input <PATH>`: Input CSV file. Default `data/synthetic/payments.csv`.
- `--output <PATH>`: Output CSV file. Default `data/synthetic/payments_invalid.csv`.
- `--payee-error-rate <F>`: Share of payees to corrupt. Default `0.02`.
- `--tx-error-rate <F>`: Share of transactions to corrupt. Default `0.01`.
- `--seed <N>`: RNG seed for repeatable output.

## `cesop-demo correct`
Apply deterministic corrections to an invalid CSV so it can be re-rendered and
validated.

- `--input <PATH>`: Input CSV file. Default `data/synthetic/payments_invalid.csv`.
- `--output <PATH>`: Output CSV file. Default `data/synthetic/payments_corrected.csv`.
- `--seed <N>`: RNG seed for repeatable output.

## `cesop-demo validate`
Run the CESOP Validation Module against an XML file.

- `--input <PATH>`: XML file or folder to validate. Default `data/output`.
- `--output <PATH>`: Optional file path to write the validation result XML.
- `--jar <PATH>`: Path to `cesop-vm-application-1.7.1.jar`.
- `--java <BIN>`: Java binary to use. Default `java`.

Example:
```sh
cesop-demo validate --input data/output --output data/output/validation.xml
```

## Logging environment variables
- `CESOP_LOG_LEVEL`: Log level (`trace`, `debug`, `info`, `warn`, `error`).
- `RUST_LOG`: Fallback log level if `CESOP_LOG_LEVEL` is not set.
- `CESOP_LOG_DIR`: Directory for log files. Set to `off` or `none` to disable file logs.

Example:
```sh
CESOP_LOG_LEVEL=info cesop-demo generate --scale 900
```
