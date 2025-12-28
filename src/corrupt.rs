use crate::models::PaymentRecord;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CorruptSummary {
    pub payees_targeted: usize,
    pub payee_name_missing: usize,
    pub payee_country_invalid: usize,
    pub account_type_invalid: usize,
    pub account_value_invalid: usize,
    pub tx_currency_invalid: usize,
    pub tx_payer_country_invalid: usize,
    pub tx_payer_source_invalid: usize,
}

impl CorruptSummary {
    pub fn new() -> Self {
        Self {
            payees_targeted: 0,
            payee_name_missing: 0,
            payee_country_invalid: 0,
            account_type_invalid: 0,
            account_value_invalid: 0,
            tx_currency_invalid: 0,
            tx_payer_country_invalid: 0,
            tx_payer_source_invalid: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PayeeCorruption {
    MissingName,
    InvalidCountry,
    InvalidAccountType,
    InvalidAccountValue,
}

#[derive(Debug, Clone, Copy)]
enum TxCorruption {
    InvalidCurrency,
    InvalidPayerCountry,
    InvalidPayerSource,
}

pub fn corrupt_csv(
    input: &Path,
    output: &Path,
    payee_error_rate: f64,
    tx_error_rate: f64,
    seed: u64,
) -> Result<CorruptSummary, String> {
    if !(0.0..=1.0).contains(&payee_error_rate) {
        return Err("payee_error_rate must be 0..1".to_string());
    }
    if !(0.0..=1.0).contains(&tx_error_rate) {
        return Err("tx_error_rate must be 0..1".to_string());
    }

    let mut reader = csv::Reader::from_path(input).map_err(|err| err.to_string())?;
    let mut records: Vec<PaymentRecord> = Vec::new();
    for result in reader.deserialize() {
        let record: PaymentRecord = result.map_err(|err| err.to_string())?;
        records.push(record);
    }

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut summary = CorruptSummary::new();

    let mut payee_map: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, record) in records.iter().enumerate() {
        payee_map
            .entry(record.payee_id.clone())
            .or_default()
            .push(idx);
    }

    let mut payee_ids: Vec<String> = payee_map.keys().cloned().collect();
    payee_ids.shuffle(&mut rng);
    let target_payees = ((payee_ids.len() as f64) * payee_error_rate).round() as usize;

    for payee_id in payee_ids.into_iter().take(target_payees) {
        let Some(indices) = payee_map.get(&payee_id) else {
            continue;
        };
        summary.payees_targeted += 1;
        let corruption = pick_payee_corruption(&mut rng);
        apply_payee_corruption(&mut records, indices, corruption, &mut summary, &mut rng);
    }

    for record in &mut records {
        if rng.gen_bool(tx_error_rate) {
            let corruption = pick_tx_corruption(&mut rng);
            apply_tx_corruption(record, corruption, &mut summary);
        }
    }

    let mut writer = csv::Writer::from_path(output).map_err(|err| err.to_string())?;
    for record in records {
        writer.serialize(record).map_err(|err| err.to_string())?;
    }
    writer.flush().map_err(|err| err.to_string())?;

    Ok(summary)
}

fn pick_payee_corruption<R: Rng + ?Sized>(rng: &mut R) -> PayeeCorruption {
    let options = [
        PayeeCorruption::MissingName,
        PayeeCorruption::InvalidCountry,
        PayeeCorruption::InvalidAccountType,
        PayeeCorruption::InvalidAccountValue,
    ];
    *options.choose(rng).unwrap_or(&PayeeCorruption::MissingName)
}

fn pick_tx_corruption<R: Rng + ?Sized>(rng: &mut R) -> TxCorruption {
    let options = [
        TxCorruption::InvalidCurrency,
        TxCorruption::InvalidPayerCountry,
        TxCorruption::InvalidPayerSource,
    ];
    *options
        .choose(rng)
        .unwrap_or(&TxCorruption::InvalidCurrency)
}

fn apply_payee_corruption<R: Rng + ?Sized>(
    records: &mut [PaymentRecord],
    indices: &[usize],
    corruption: PayeeCorruption,
    summary: &mut CorruptSummary,
    rng: &mut R,
) {
    match corruption {
        PayeeCorruption::MissingName => {
            for idx in indices {
                records[*idx].payee_name.clear();
            }
            summary.payee_name_missing += 1;
        }
        PayeeCorruption::InvalidCountry => {
            for idx in indices {
                records[*idx].payee_country = "ZZ".to_string();
            }
            summary.payee_country_invalid += 1;
        }
        PayeeCorruption::InvalidAccountType => {
            let account = format!("ACC{}", random_digits(rng, 10));
            for idx in indices {
                records[*idx].payee_account = account.clone();
                records[*idx].payee_account_type = "BADTYPE".to_string();
            }
            summary.account_type_invalid += 1;
        }
        PayeeCorruption::InvalidAccountValue => {
            let account = format!("ZZ00{}", random_digits(rng, 14));
            for idx in indices {
                records[*idx].payee_account = account.clone();
                records[*idx].payee_account_type = "IBAN".to_string();
            }
            summary.account_value_invalid += 1;
        }
    }
}

fn apply_tx_corruption(
    record: &mut PaymentRecord,
    corruption: TxCorruption,
    summary: &mut CorruptSummary,
) {
    match corruption {
        TxCorruption::InvalidCurrency => {
            record.currency = "EURO".to_string();
            summary.tx_currency_invalid += 1;
        }
        TxCorruption::InvalidPayerCountry => {
            record.payer_country = "ZZ".to_string();
            summary.tx_payer_country_invalid += 1;
        }
        TxCorruption::InvalidPayerSource => {
            record.payer_ms_source = "BAD".to_string();
            summary.tx_payer_source_invalid += 1;
        }
    }
}

fn random_digits<R: Rng + ?Sized>(rng: &mut R, len: usize) -> String {
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let digit = rng.gen_range(0..10);
        out.push(char::from(b'0' + digit as u8));
    }
    out
}
