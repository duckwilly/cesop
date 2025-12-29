use crate::models::PaymentRecord;

pub fn normalize_country_code(code: &str) -> Option<String> {
    let trimmed = code.trim();
    if trimmed.len() == 2 && trimmed.chars().all(|ch| ch.is_ascii_alphabetic()) {
        Some(trimmed.to_uppercase())
    } else {
        None
    }
}

pub fn bic_country_code(bic: &str) -> Option<String> {
    let bic = bic.trim();
    if !(bic.len() == 8 || bic.len() == 11) {
        return None;
    }
    let code = &bic[4..6];
    if code.chars().all(|ch| ch.is_ascii_alphabetic()) {
        Some(code.to_uppercase())
    } else {
        None
    }
}

pub fn account_country_code(account_type: &str, account_id: &str) -> Option<String> {
    let account_id = account_id.trim();
    if account_id.is_empty() {
        return None;
    }
    match account_type.trim().to_uppercase().as_str() {
        "IBAN" | "OBAN" => account_id.get(0..2).and_then(normalize_country_code),
        "BIC" => bic_country_code(account_id),
        "OTHER" => account_id.get(0..2).and_then(normalize_country_code),
        _ => None,
    }
}

pub fn resolve_payee_country(record: &PaymentRecord) -> Result<String, String> {
    if !record.payee_account.trim().is_empty() {
        if let Some(country) =
            account_country_code(&record.payee_account_type, &record.payee_account)
        {
            return Ok(country);
        }
        if let Some(psp_id) = record.payee_psp_id.as_deref() {
            if let Some(country) = bic_country_code(psp_id) {
                return Ok(country);
            }
        }
        return Err(
            "payee account identifier does not encode country and payee PSP BIC is missing"
                .to_string(),
        );
    }
    if let Some(psp_id) = record.payee_psp_id.as_deref() {
        if let Some(country) = bic_country_code(psp_id) {
            return Ok(country);
        }
    }
    Err("payee country cannot be derived from account identifier or payee PSP BIC".to_string())
}
