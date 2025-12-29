# CESOP Business Rules (Directive + Implementing Regulation)

Sources:
- `docs/cesop directive.htm` (Directive 2020/284; Articles 243b-243d)
- `docs/implementing regulation.pdf` (CESOP technical measures)
- `docs/implementing regulation annex.pdf` (XML data elements + checks)

## Scope and definitions
- Applies to payment service providers (PSPs) within scope of PSD2.
- A "payment" is a payment transaction or money remittance (PSD2).
- Cross-border payments are the trigger for record keeping/reporting.

## Cross-border definition (Art. 243b(1))
A payment is cross-border when:
- The payer is located in a Member State, and
- The payee is located in another Member State, or in a third territory/country.

Payer/payee location is determined via identifiers (Art. 243c):
- Use IBAN or another identifier that unambiguously identifies location.
- If no identifier, use the BIC (or equivalent) of the PSP acting for payer/payee.
  - Project behavior: payee country is derived from account identifiers first
    (IBAN/OBAN/BIC/Other) and falls back to the payee PSP BIC only when no
    account identifier is available.

## Threshold and aggregation (Art. 243b(2))
Record keeping/reporting applies when, in a calendar quarter:
- A PSP provides payment services for **more than 25** cross-border payments
  **to the same payee**.

Counting rules:
- Calculate **per Member State** (payee location) and **per payee identifier**
  (as per Art. 243c(2)).
- If the PSP knows the payee has multiple identifiers, **aggregate per payee**
  (not per identifier).

## Which PSP reports? (Art. 243b(3))
- If **at least one payee PSP is located in a Member State**, payer PSPs do
  **not** report those payments, but they **do** include them in the threshold
  calculation.
- If payee PSPs are outside the EU, payer PSPs report.

## Record contents (Art. 243d)
Required payee/PSP info:
- Reporting PSP BIC/identifier.
- Payee name (as recorded by PSP).
- Payee VAT/TIN (if available).
- Payee account identifier (IBAN or other) **when funds go to a payee account**.
- Payee PSP BIC/identifier **when payee has no account** (ReportedPayee/Representative).
- Payee address (if available).

Required payment/refund info:
- Date/time of payment or refund.
- Amount and currency.
- Member State of **origin payment** or **destination refund**.
- Source used to determine payer location (IBAN/BIN/Other; do not send the ID).
- Unique payment reference.
- Physical-premises indicator (when applicable).

## Annex (Implementing Regulation) checks
Mandatory elements and checks at transmission to CESOP (high-level):
- PSP BIC/identifier: presence + BIC syntax.
- Payee name: presence.
- Payee VAT/TIN: if provided, EU VAT syntax check.
- Payee account ID (if payee account used): presence + IBAN syntax.
- Payee PSP BIC (if no payee account): presence + BIC syntax.
- Payee account identifiers: either a single `IBAN`/`OBAN`/`Other`, or an
  account identifier paired with a `BIC`.
- DateTime, Amount, Currency: presence + format checks.
- MS origin payment / destination refund: presence + country code format.
- Payer location source: presence; **do not** transmit payer identifier itself.
- Transaction ID: presence.
- Physical presence: presence if applicable.

## Retention
- PSPs keep records for **3 calendar years** from end of payment year.

## Notes for this project
- We treat refunds as linked to an original payment and report them in the same
  payee group; refunds do **not** count toward the >25 threshold by default.
- Reporting assumes a payee account is present unless explicitly modeled
  otherwise (Representative/Payee PSP case).
- We model the payer-PSP vs payee-PSP rule with `psp_role` and `payee_psp_id`;
  payer PSP records are reportable only when the payee PSP is outside the EU.
  Threshold counts still include payer-PSP payments when the payee PSP is in
  the EU, per Art. 243b(3).
- CESOP validation allows **one account identifier** per payee (IBAN/OBAN/Other)
  or a paired account + BIC. When multiple identifiers exist, the XML output
  chooses a primary account (IBAN > OBAN > Other) and one optional BIC.
