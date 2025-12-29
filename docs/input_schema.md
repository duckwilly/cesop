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
- `payee_country`: ISO-3166 alpha-2 country code (optional input; derived from
  the payee identifier/BIC when missing or validated when present).
- `payee_id`: Internal payee identifier (e.g., `MER000123`).
- `payee_name`: Payee legal or trading name.
- `payee_account`: Payee account identifier (IBAN-like string). Optional when
  the payee receives funds without a payment account and `payee_psp_id` is set.
- `payee_account_type`: Account identifier type (e.g., `IBAN`). Optional when
  `payee_account` is empty.
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
- `psp_role`: Optional role of the reporting PSP (`PAYEE` or `PAYER`).
- `payee_psp_id`: Optional PSP identifier acting for the payee (BIC-like).
- `payee_psp_name`: Optional PSP name acting for the payee.
- `psp_id`: PSP identifier (BIC-like string).
- `psp_name`: PSP name.

## Notes
- Cross-border logic uses payer-in-EU and derived payee location
  (`payer_country` != derived payee country).
- Threshold logic is per Member State (payee location) and per payee identifier;
  if multiple identifiers are known for a payee, counts aggregate per payee.
- A single `payee_id` may appear with multiple `payee_account` values to model
  multiple identifiers; all of them are reported under the same payee.
- `payee_country` may be non-EU (third country/territory) to model cross-border
  reporting outside the Union.
- If `payee_country` is missing or mismatched, the pipeline derives it from
  `payee_account` (IBAN/OBAN/BIC/Other). When no account identifier is present,
  `payee_psp_id` is used per Art. 243c.
- When `payee_account` is empty and a valid `payee_psp_id` is present, the XML
  output uses the `Representative` element (payee PSP) and emits an empty
  `AccountIdentifier`.
- The generator intentionally includes payees above and below the 25-payment
  threshold to showcase CESOP eligibility rules.
- CSV inputs may contain multiple PSPs; rendering groups reports by PSP and
  quarter automatically.
- If `psp_role=PAYER` and the payee PSP is in the EU, those records are not
  reportable by the payer PSP and are skipped during rendering.
- If `data/reference/company_cores.txt` exists, the generator draws a core name
  from that list (one per line) before applying suffixes and legal endings.
