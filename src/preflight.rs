use crate::analysis::analyze_threshold_records;
use crate::models::PaymentRecord;
use crate::reference::{iban_length, is_eu_member_state, ACCOUNT_IDENTIFIER_TYPES};
use crate::util::iban_check_digits;
use chrono::DateTime;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct PreflightIssue {
    pub level: IssueLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct PreflightReport {
    pub threshold: usize,
    pub total_records: usize,
    pub cross_border_records: usize,
    pub total_payees: usize,
    pub payees_over_threshold: usize,
    pub issues: Vec<PreflightIssue>,
}

impl PreflightReport {
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.level == IssueLevel::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.level == IssueLevel::Warning)
            .count()
    }
}

pub fn preflight_csv(
    path: &Path,
    threshold: usize,
    include_refunds: bool,
) -> Result<PreflightReport, String> {
    let mut reader = csv::Reader::from_path(path).map_err(|err| err.to_string())?;
    let mut records = Vec::new();
    let mut issues = Vec::new();
    let mut payment_ids: HashSet<String> = HashSet::new();
    let mut psp_names: HashMap<String, String> = HashMap::new();

    for result in reader.deserialize() {
        let record: PaymentRecord = result.map_err(|err| err.to_string())?;
        validate_record(&record, &mut issues);

        if !payment_ids.insert(record.payment_id.clone()) {
            issues.push(PreflightIssue {
                level: IssueLevel::Error,
                message: "duplicate payment_id detected".to_string(),
            });
        }

        if let Some(existing) = psp_names.get(&record.psp_id) {
            if existing != &record.psp_name {
                issues.push(PreflightIssue {
                    level: IssueLevel::Error,
                    message: format!(
                        "multiple PSP names found for {}: '{}' vs '{}'",
                        record.psp_id, existing, record.psp_name
                    ),
                });
            }
        } else {
            psp_names.insert(record.psp_id.clone(), record.psp_name.clone());
        }

        records.push(record);
    }

    let payment_id_set: HashSet<String> = payment_ids;
    for record in &records {
        if record.is_refund {
            match record.corr_payment_id.as_deref() {
                Some(corr) if payment_id_set.contains(corr) => {}
                Some(corr) => issues.push(PreflightIssue {
                    level: IssueLevel::Warning,
                    message: format!("refund references missing payment_id {}", corr),
                }),
                None => {}
            }
        }
    }

    let report = analyze_threshold_records(&records, threshold, include_refunds);

    Ok(PreflightReport {
        threshold,
        total_records: report.total_records,
        cross_border_records: report.cross_border_records,
        total_payees: report.total_payees,
        payees_over_threshold: report.payees_over_threshold,
        issues,
    })
}

fn validate_record(record: &PaymentRecord, issues: &mut Vec<PreflightIssue>) {
    if record.payment_id.trim().is_empty() {
        issues.push(issue(IssueLevel::Error, "payment_id is required"));
    }
    if record.execution_time.trim().is_empty() {
        issues.push(issue(IssueLevel::Error, "execution_time is required"));
    } else if DateTime::parse_from_rfc3339(&record.execution_time).is_err() {
        issues.push(issue(
            IssueLevel::Error,
            "execution_time must be RFC3339 with timezone",
        ));
    }
    if !is_valid_amount(&record.amount) {
        issues.push(issue(
            IssueLevel::Error,
            "amount must be a decimal with two digits",
        ));
    }
    if !is_valid_currency(&record.currency) {
        issues.push(issue(
            IssueLevel::Error,
            "currency must be ISO-4217 alpha-3",
        ));
    }
    if !is_valid_country(&record.payer_country) {
        issues.push(issue(
            IssueLevel::Error,
            "payer_country must be ISO-3166 alpha-2",
        ));
    } else if !is_eu_member_state(&record.payer_country) {
        issues.push(issue(
            IssueLevel::Error,
            "payer_country must be an EU Member State",
        ));
    }
    if !is_valid_country(&record.payee_country) {
        issues.push(issue(
            IssueLevel::Error,
            "payee_country must be ISO-3166 alpha-2",
        ));
    }
    if record.payer_country == record.payee_country {
        issues.push(issue(
            IssueLevel::Warning,
            "payment is not cross-border (not reportable)",
        ));
    }
    if record.payee_id.trim().is_empty() {
        issues.push(issue(IssueLevel::Error, "payee_id is required"));
    }
    if record.payee_name.trim().is_empty() {
        issues.push(issue(IssueLevel::Error, "payee_name is required"));
    }
    if record.payee_account.trim().is_empty() {
        issues.push(issue(IssueLevel::Error, "payee_account is required"));
    }
    if !ACCOUNT_IDENTIFIER_TYPES
        .iter()
        .any(|value| *value == record.payee_account_type)
    {
        issues.push(issue(
            IssueLevel::Error,
            "payee_account_type must be IBAN/OBAN/BIC/Other",
        ));
    } else if record.payee_account_type == "IBAN" {
        validate_iban(&record.payee_account, &record.payee_country, issues);
    }
    if !ACCOUNT_IDENTIFIER_TYPES
        .iter()
        .any(|value| *value == record.payer_ms_source)
    {
        issues.push(issue(
            IssueLevel::Error,
            "payer_ms_source must be IBAN/OBAN/BIC/Other",
        ));
    }
    if record.payment_method.trim().is_empty() {
        issues.push(issue(IssueLevel::Error, "payment_method is required"));
    }
    if record.is_refund && record.corr_payment_id.is_none() {
        issues.push(issue(
            IssueLevel::Error,
            "refunds must include corr_payment_id",
        ));
    }
    if !record.is_refund && record.corr_payment_id.is_some() {
        issues.push(issue(
            IssueLevel::Warning,
            "corr_payment_id set on non-refund",
        ));
    }
    if record.psp_id.trim().is_empty() {
        issues.push(issue(IssueLevel::Error, "psp_id is required"));
    } else if !is_valid_bic(&record.psp_id) {
        issues.push(issue(
            IssueLevel::Warning,
            "psp_id is not a valid BIC format",
        ));
    }
    if record.psp_name.trim().is_empty() {
        issues.push(issue(IssueLevel::Error, "psp_name is required"));
    }
}

fn issue(level: IssueLevel, message: &str) -> PreflightIssue {
    PreflightIssue {
        level,
        message: message.to_string(),
    }
}

fn is_valid_amount(amount: &str) -> bool {
    let mut parts = amount.split('.');
    let whole = match parts.next() {
        Some(part) if !part.is_empty() => part,
        _ => return false,
    };
    let frac = match parts.next() {
        Some(part) if part.len() == 2 => part,
        _ => return false,
    };
    if parts.next().is_some() {
        return false;
    }
    whole.chars().all(|ch| ch.is_ascii_digit()) && frac.chars().all(|ch| ch.is_ascii_digit())
}

fn is_valid_currency(code: &str) -> bool {
    code.len() == 3 && code.chars().all(|ch| ch.is_ascii_uppercase())
}

fn is_valid_country(code: &str) -> bool {
    code.len() == 2 && code.chars().all(|ch| ch.is_ascii_uppercase())
}

fn is_valid_bic(bic: &str) -> bool {
    let bic = bic.trim();
    if !(bic.len() == 8 || bic.len() == 11) {
        return false;
    }
    if !bic.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return false;
    }
    let country = &bic[4..6];
    country.chars().all(|ch| ch.is_ascii_alphabetic())
}

fn validate_iban(
    iban: &str,
    country: &str,
    issues: &mut Vec<PreflightIssue>,
) {
    if iban.len() < 4 {
        issues.push(issue(IssueLevel::Error, "IBAN is too short"));
        return;
    }
    if !iban.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        issues.push(issue(IssueLevel::Error, "IBAN must be alphanumeric"));
    }
    let iban_country = &iban[0..2];
    if iban_country != country {
        issues.push(issue(
            IssueLevel::Error,
            "IBAN country code does not match payee_country",
        ));
    }
    if let Some(expected) = iban_length(country) {
        if iban.len() != expected {
            issues.push(issue(
                IssueLevel::Error,
                "IBAN length does not match country specification",
            ));
        }
    } else {
        issues.push(issue(
            IssueLevel::Warning,
            "IBAN length not known for country",
        ));
    }
    let check_digits = &iban[2..4];
    let bban = &iban[4..];
    if let Ok(expected) = iban_check_digits(iban_country, bban) {
        if expected != check_digits {
            issues.push(issue(
                IssueLevel::Error,
                "IBAN check digits are invalid",
            ));
        }
    }
}
