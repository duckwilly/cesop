use crate::models::PaymentRecord;
use std::collections::HashMap;
use std::path::Path;

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
    let mut total_records = 0usize;
    let mut cross_border_records = 0usize;
    let mut payees: HashMap<String, usize> = HashMap::new();

    for result in reader.deserialize() {
        let record: PaymentRecord = result.map_err(|err| err.to_string())?;
        total_records += 1;

        if record.payer_country == record.payee_country {
            continue;
        }
        if record.is_refund && !include_refunds {
            continue;
        }

        cross_border_records += 1;
        let entry = payees.entry(record.payee_id.clone()).or_insert(0);
        *entry += 1;
    }

    let total_payees = payees.len();
    let payees_over_threshold = payees.values().filter(|count| **count > threshold).count();

    Ok(ThresholdReport {
        threshold,
        total_records,
        cross_border_records,
        total_payees,
        payees_over_threshold,
    })
}
