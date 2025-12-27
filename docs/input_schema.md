# Synthetic Payment Input Schema (v0)

The generator writes a flat, row-based dataset intended to be easy to transform
into CESOP XML. The same fields are used for CSV, JSON, or JSONL output.

## Fields
- `payment_id`: Unique payment identifier (UUID v4).
- `execution_time`: ISO-8601 timestamp with timezone (UTC, RFC3339).
- `amount`: Decimal string with two digits after the decimal point.
- `currency`: ISO-4217 alpha-3 currency code.
- `payer_country`: ISO-3166 alpha-2 Member State code.
- `payer_ms_source`: Source used to infer payer MS (e.g., `IBAN`).
- `payee_country`: ISO-3166 alpha-2 country code.
- `payee_id`: Internal payee identifier (e.g., `MER000123`).
- `payee_name`: Payee legal or trading name.
- `payee_account`: Payee account identifier (IBAN-like string).
- `payee_account_type`: Account identifier type (e.g., `IBAN`).
- `payee_tax_id`: Optional national tax identifier (if available).
- `payee_vat_id`: Optional VAT identifier (if available).
- `payee_email`: Optional payee contact email.
- `payee_web`: Optional payee website.
- `payee_address_line`: Optional street address line.
- `payee_city`: Optional city name.
- `payee_postcode`: Optional postal code.
- `payment_method`: One of the CESOP payment method types (e.g., `Card payment`).
- `initiated_at_pos`: Boolean indicating physical POS initiation.
- `is_refund`: Boolean indicating refund.
- `corr_payment_id`: Optional reference to the original payment for refunds.
- `psp_id`: PSP identifier (BIC-like string).
- `psp_name`: PSP name.

## Notes
- Cross-border logic is determined by `payer_country` != `payee_country`.
- The generator intentionally includes payees above and below the 25-payment
  threshold to showcase CESOP eligibility rules.
- If `data/reference/company_cores.txt` exists, the generator draws a core name
  from that list (one per line) before applying suffixes and legal endings.
