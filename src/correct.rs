use crate::location::{account_country_code, bic_country_code, normalize_country_code};
use crate::models::PaymentRecord;
use crate::reference::{iban_length, is_eu_member_state, ACCOUNT_IDENTIFIER_TYPES, EU_MEMBER_STATES};
use crate::util::{iban_check_digits, random_alphanum_upper, random_digits};
use chrono::Utc;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CorrectSummary {
    pub total_records: usize,
    pub corrected_records: usize,
    pub payee_name_fixed: usize,
    pub payee_country_fixed: usize,
    pub payee_account_type_fixed: usize,
    pub payee_account_value_fixed: usize,
    pub payer_country_fixed: usize,
    pub payer_source_fixed: usize,
    pub currency_fixed: usize,
    pub execution_time_fixed: usize,
}

impl CorrectSummary {
    fn new() -> Self {
        Self {
            total_records: 0,
            corrected_records: 0,
            payee_name_fixed: 0,
            payee_country_fixed: 0,
            payee_account_type_fixed: 0,
            payee_account_value_fixed: 0,
            payer_country_fixed: 0,
            payer_source_fixed: 0,
            currency_fixed: 0,
            execution_time_fixed: 0,
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct PayeeKey {
    psp_id: String,
    payee_id: String,
}

#[derive(Debug, Clone)]
struct PayeePlan {
    country: String,
    account_type: Option<String>,
    account_id: Option<String>,
}

pub fn correct_csv(input: &Path, output: &Path, seed: u64) -> Result<CorrectSummary, String> {
    let mut reader = csv::Reader::from_path(input).map_err(|err| err.to_string())?;
    let mut records: Vec<PaymentRecord> = Vec::new();
    for result in reader.deserialize() {
        let record: PaymentRecord = result.map_err(|err| err.to_string())?;
        records.push(record);
    }

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut summary = CorrectSummary::new();
    let payee_plans = build_payee_plans(&records, &mut rng);

    for record in &mut records {
        summary.total_records += 1;
        let mut corrected = false;

        if record.payment_id.trim().is_empty() {
            record.payment_id = Uuid::new_v4().to_string();
            corrected = true;
        }

        if !is_valid_timestamp(&record.execution_time) {
            record.execution_time = Utc::now().to_rfc3339();
            summary.execution_time_fixed += 1;
            corrected = true;
        }

        if record.payee_name.trim().is_empty() {
            let label = if record.payee_id.trim().is_empty() {
                "Unknown Payee".to_string()
            } else {
                format!("Payee {}", record.payee_id.trim())
            };
            record.payee_name = label;
            summary.payee_name_fixed += 1;
            corrected = true;
        }

        let payer_country = normalize_country_code(&record.payer_country);
        let fixed_payer_country = match payer_country {
            Some(code) if is_eu_member_state(&code) => code,
            _ => {
                summary.payer_country_fixed += 1;
                corrected = true;
                fallback_payer_country(&record.psp_id, &mut rng)
            }
        };
        record.payer_country = fixed_payer_country.clone();

        let payer_source_trimmed = record.payer_ms_source.trim();
        if let Some(canonical) = canonical_account_type(payer_source_trimmed) {
            if record.payer_ms_source != canonical {
                record.payer_ms_source = canonical.to_string();
                summary.payer_source_fixed += 1;
                corrected = true;
            }
        } else {
            record.payer_ms_source = "IBAN".to_string();
            summary.payer_source_fixed += 1;
            corrected = true;
        }

        if !is_valid_currency(&record.currency) {
            record.currency = currency_for_country(&fixed_payer_country).to_string();
            summary.currency_fixed += 1;
            corrected = true;
        }

        let key = PayeeKey {
            psp_id: record.psp_id.clone(),
            payee_id: record.payee_id.clone(),
        };
        let plan = payee_plans
            .get(&key)
            .ok_or_else(|| "missing correction plan for payee".to_string())?;

        let (planned_type, planned_id) = match (&plan.account_type, &plan.account_id) {
            (Some(account_type), Some(account_id)) => (account_type.clone(), account_id.clone()),
            _ => (String::new(), String::new()),
        };
        if record.payee_account_type != planned_type {
            record.payee_account_type = planned_type;
            summary.payee_account_type_fixed += 1;
            corrected = true;
        }
        if record.payee_account != planned_id {
            record.payee_account = planned_id;
            summary.payee_account_value_fixed += 1;
            corrected = true;
        }
        if normalize_country_code(&record.payee_country)
            .as_deref()
            != Some(plan.country.as_str())
        {
            record.payee_country = plan.country.clone();
            summary.payee_country_fixed += 1;
            corrected = true;
        }

        if corrected {
            summary.corrected_records += 1;
        }
    }

    let mut writer = csv::Writer::from_path(output).map_err(|err| err.to_string())?;
    for record in records {
        writer.serialize(record).map_err(|err| err.to_string())?;
    }
    writer.flush().map_err(|err| err.to_string())?;

    Ok(summary)
}

fn is_valid_timestamp(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

fn is_valid_currency(code: &str) -> bool {
    code.len() == 3 && code.chars().all(|ch| ch.is_ascii_uppercase())
}

fn canonical_account_type(value: &str) -> Option<&'static str> {
    let trimmed = value.trim();
    for allowed in ACCOUNT_IDENTIFIER_TYPES {
        if allowed.eq_ignore_ascii_case(trimmed) {
            return Some(*allowed);
        }
    }
    None
}

fn build_payee_plans<R: Rng + ?Sized>(
    records: &[PaymentRecord],
    rng: &mut R,
) -> HashMap<PayeeKey, PayeePlan> {
    let mut groups: HashMap<PayeeKey, Vec<&PaymentRecord>> = HashMap::new();
    for record in records {
        let key = PayeeKey {
            psp_id: record.psp_id.clone(),
            payee_id: record.payee_id.clone(),
        };
        groups.entry(key).or_default().push(record);
    }

    let mut plans = HashMap::new();
    for (key, group) in groups {
        let country = derive_target_payee_country_for_group(&group, rng);
        let (account_type, account_id) = select_payee_account(&group, &country, rng);
        plans.insert(
            key,
            PayeePlan {
                country,
                account_type,
                account_id,
            },
        );
    }
    plans
}

fn derive_target_payee_country_for_group<R: Rng + ?Sized>(
    records: &[&PaymentRecord],
    rng: &mut R,
) -> String {
    for record in records {
        if let Some(country) = account_country_for_record(record) {
            return country;
        }
    }
    for record in records {
        if let Some(psp_id) = record.payee_psp_id.as_deref() {
            if let Some(country) = bic_country_code(psp_id) {
                return country;
            }
        }
    }
    for record in records {
        if let Some(country) = normalize_known_country(&record.payee_country) {
            return country;
        }
    }
    if let Some(first) = records.first() {
        if let Some(country) = bic_country_code(&first.psp_id) {
            return country;
        }
    }
    EU_MEMBER_STATES
        .choose(rng)
        .unwrap_or(&"DE")
        .to_string()
}

fn account_country_for_record(record: &PaymentRecord) -> Option<String> {
    let account_type = canonical_account_type(&record.payee_account_type)?;
    let account_id = record.payee_account.trim();
    if account_id.is_empty() {
        return None;
    }
    match account_type {
        "IBAN" => {
            let country = account_country_code("IBAN", account_id)?;
            if is_valid_iban(account_id, &country) {
                Some(country)
            } else {
                None
            }
        }
        "OBAN" | "Other" => account_country_code(account_type, account_id),
        "BIC" => bic_country_code(account_id),
        _ => None,
    }
}

fn select_payee_account<R: Rng + ?Sized>(
    records: &[&PaymentRecord],
    country: &str,
    rng: &mut R,
) -> (Option<String>, Option<String>) {
    let mut ibans: BTreeSet<String> = BTreeSet::new();
    let mut obans: BTreeSet<String> = BTreeSet::new();
    let mut others: BTreeSet<String> = BTreeSet::new();
    let mut saw_account = false;

    for record in records {
        let account_id = record.payee_account.trim();
        if account_id.is_empty() {
            continue;
        }
        saw_account = true;
        let account_type = canonical_account_type(&record.payee_account_type);
        match account_type {
            Some("IBAN") => {
                if is_valid_iban(account_id, country) {
                    ibans.insert(account_id.to_string());
                }
            }
            Some("OBAN") => {
                if account_country_code("OBAN", account_id)
                    .as_deref()
                    .map(|code| code == country)
                    .unwrap_or(false)
                {
                    obans.insert(account_id.to_string());
                }
            }
            Some("Other") => {
                if account_country_code("Other", account_id)
                    .as_deref()
                    .map(|code| code == country)
                    .unwrap_or(false)
                {
                    others.insert(account_id.to_string());
                }
            }
            _ => {}
        }
    }

    if !saw_account && has_valid_payee_psp(records) {
        return (None, None);
    }

    if let Some(account_id) = ibans.iter().next() {
        return (
            Some("IBAN".to_string()),
            Some(account_id.to_string()),
        );
    }
    if let Some(account_id) = obans.iter().next() {
        return (
            Some("OBAN".to_string()),
            Some(account_id.to_string()),
        );
    }
    if let Some(account_id) = others.iter().next() {
        return (
            Some("Other".to_string()),
            Some(account_id.to_string()),
        );
    }

    let (account_type, account_id) = generate_account_for_country(country, rng);
    (Some(account_type), Some(account_id))
}

fn has_valid_payee_psp(records: &[&PaymentRecord]) -> bool {
    records.iter().any(|record| {
        record
            .payee_psp_id
            .as_deref()
            .and_then(bic_country_code)
            .is_some()
    })
}

fn generate_account_for_country<R: Rng + ?Sized>(country: &str, rng: &mut R) -> (String, String) {
    if let Some(len) = iban_length(country) {
        let bban_len = len.saturating_sub(4);
        let bban = random_digits(rng, bban_len);
        let check = iban_check_digits(country, &bban).unwrap_or_else(|_| "00".to_string());
        return ("IBAN".to_string(), format!("{}{}{}", country, check, bban));
    }

    let suffix = random_alphanum_upper(rng, 12);
    ("Other".to_string(), format!("{}{}", country, suffix))
}

fn is_valid_iban(iban: &str, country: &str) -> bool {
    if iban.len() < 4 {
        return false;
    }
    if !iban.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return false;
    }
    if &iban[0..2].to_uppercase() != country {
        return false;
    }
    if let Some(expected) = iban_length(country) {
        if iban.len() != expected {
            return false;
        }
    } else {
        return false;
    }
    let check_digits = &iban[2..4];
    let bban = &iban[4..];
    match iban_check_digits(country, bban) {
        Ok(expected) => expected == check_digits,
        Err(_) => false,
    }
}

fn normalize_known_country(value: &str) -> Option<String> {
    let code = normalize_country_code(value)?;
    if is_known_country(&code) {
        Some(code)
    } else {
        None
    }
}

fn is_known_country(code: &str) -> bool {
    if is_eu_member_state(code) {
        return true;
    }
    if iban_length(code).is_some() {
        return true;
    }
    matches!(code, "US" | "CA")
}

fn fallback_payer_country<R: Rng + ?Sized>(psp_id: &str, rng: &mut R) -> String {
    if let Some(country) = bic_country_code(psp_id) {
        if is_eu_member_state(&country) {
            return country;
        }
    }
    let fallback = EU_MEMBER_STATES
        .choose(rng)
        .unwrap_or(&"DE")
        .to_string();
    fallback
}

fn currency_for_country(country: &str) -> &'static str {
    match country {
        "BG" => "BGN",
        "CZ" => "CZK",
        "DK" => "DKK",
        "HU" => "HUF",
        "PL" => "PLN",
        "RO" => "RON",
        "SE" => "SEK",
        _ => "EUR",
    }
}
