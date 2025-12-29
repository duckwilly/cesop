use crate::location::resolve_payee_country;
use crate::models::PaymentRecord;
use crate::reference::is_eu_member_state;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct PayeeKey {
    pub psp_id: String,
    pub payee_id: String,
    pub payee_country: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct IdentifierKey {
    psp_id: String,
    payee_country: String,
    identifier: String,
}

#[derive(Debug, Clone)]
pub struct ThresholdReport {
    pub threshold: usize,
    pub total_records: usize,
    pub cross_border_records: usize,
    pub total_payees: usize,
    pub payees_over_threshold: usize,
}

pub fn analyze_threshold_csv(
    path: &Path,
    threshold: usize,
    include_refunds: bool,
) -> Result<ThresholdReport, String> {
    let mut reader = csv::Reader::from_path(path).map_err(|err| err.to_string())?;
    let mut records = Vec::new();
    for result in reader.deserialize() {
        let record: PaymentRecord = result.map_err(|err| err.to_string())?;
        records.push(record);
    }

    let (_payees, report) = compute_reportability(&records, threshold, include_refunds)?;
    Ok(report)
}

pub fn reportable_payee_keys(
    records: &[PaymentRecord],
    threshold: usize,
    include_refunds: bool,
) -> Result<HashSet<PayeeKey>, String> {
    let (payees, _report) = compute_reportability(records, threshold, include_refunds)?;
    Ok(payees)
}

pub fn analyze_threshold_records(
    records: &[PaymentRecord],
    threshold: usize,
    include_refunds: bool,
) -> Result<ThresholdReport, String> {
    let (_payees, report) = compute_reportability(records, threshold, include_refunds)?;
    Ok(report)
}

fn compute_reportability(
    records: &[PaymentRecord],
    threshold: usize,
    include_refunds: bool,
) -> Result<(HashSet<PayeeKey>, ThresholdReport), String> {
    let total_records = records.len();
    let multi_identifier_payees = payees_with_multiple_identifiers(records)?;

    let mut cross_border_records = 0usize;
    let mut counts: HashMap<IdentifierKey, usize> = HashMap::new();
    let mut payee_set: HashSet<PayeeKey> = HashSet::new();

    for record in records {
        let payee_country = resolve_payee_country(record)?;
        if !is_cross_border(record.payer_country.as_str(), &payee_country) {
            continue;
        }
        if record.is_refund && !include_refunds {
            continue;
        }

        cross_border_records += 1;
        let payee_key = payee_key(record, &payee_country);
        payee_set.insert(payee_key.clone());

        let key = identifier_key(record, &payee_country, &multi_identifier_payees);
        *counts.entry(key).or_insert(0) += 1;
    }

    let mut reportable_payees: HashSet<PayeeKey> = HashSet::new();
    for record in records {
        let payee_country = resolve_payee_country(record)?;
        if !is_cross_border(record.payer_country.as_str(), &payee_country) {
            continue;
        }
        if record.is_refund && !include_refunds {
            continue;
        }

        let payee_key = payee_key(record, &payee_country);
        let key = identifier_key(record, &payee_country, &multi_identifier_payees);
        if let Some(count) = counts.get(&key) {
            if *count > threshold {
                reportable_payees.insert(payee_key);
            }
        }
    }

    let total_payees = payee_set.len();
    let payees_over_threshold = reportable_payees.len();

    let report = ThresholdReport {
        threshold,
        total_records,
        cross_border_records,
        total_payees,
        payees_over_threshold,
    };

    Ok((reportable_payees, report))
}

fn payees_with_multiple_identifiers(
    records: &[PaymentRecord],
) -> Result<HashSet<PayeeKey>, String> {
    let mut accounts: HashMap<PayeeKey, HashSet<String>> = HashMap::new();
    for record in records {
        let payee_country = resolve_payee_country(record)?;
        let payee_key = payee_key(record, &payee_country);
        let identifier = account_identifier(record);
        accounts
            .entry(payee_key)
            .or_default()
            .insert(identifier);
    }

    Ok(accounts
        .into_iter()
        .filter_map(|(key, ids)| if ids.len() > 1 { Some(key) } else { None })
        .collect())
}

fn is_cross_border(payer_country: &str, payee_country: &str) -> bool {
    is_eu_member_state(payer_country) && payer_country != payee_country
}

fn payee_key(record: &PaymentRecord, payee_country: &str) -> PayeeKey {
    PayeeKey {
        psp_id: record.psp_id.clone(),
        payee_id: record.payee_id.clone(),
        payee_country: payee_country.to_string(),
    }
}

fn account_identifier(record: &PaymentRecord) -> String {
    if record.payee_account.trim().is_empty() {
        let representative = record
            .payee_psp_id
            .as_deref()
            .unwrap_or("")
            .trim();
        return format!("PSP:{}", representative);
    }
    format!("{}:{}", record.payee_account_type, record.payee_account)
}

fn identifier_key(
    record: &PaymentRecord,
    payee_country: &str,
    multi_identifier_payees: &HashSet<PayeeKey>,
) -> IdentifierKey {
    let payee_key = payee_key(record, payee_country);
    let identifier = if multi_identifier_payees.contains(&payee_key) {
        record.payee_id.clone()
    } else {
        account_identifier(record)
    };

    IdentifierKey {
        psp_id: record.psp_id.clone(),
        payee_country: payee_country.to_string(),
        identifier,
    }
}
