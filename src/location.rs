use crate::models::PaymentRecord;

fn map_country_code(code: &str) -> Option<String> {
    let trimmed = code.trim();
    if trimmed.len() != 2 || !trimmed.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return None;
    }

    let upper = trimmed.to_uppercase();
    match upper.as_str() {
        // Common real-world variants
        "UK" => Some("GB".to_string()),
        "EL" => Some("GR".to_string()),
        _ => Some(upper),
    }
}

pub fn normalize_country_code(code: &str) -> Option<String> {
    map_country_code(code)
}

pub fn bic_country_code(bic: &str) -> Option<String> {
    let bic = bic.trim();
    if !(bic.len() == 8 || bic.len() == 11) {
        return None;
    }
    let code = &bic[4..6];
    map_country_code(code)
}

pub fn account_country_code(account_type: &str, account_id: &str) -> Option<String> {
    let account_id = account_id.trim();
    if account_id.is_empty() {
        return None;
    }
    match account_type.trim().to_uppercase().as_str() {
        "IBAN" | "OBAN" | "OTHER" => account_id.get(0..2).and_then(map_country_code),
        "BIC" => bic_country_code(account_id),
        _ => None,
    }
}

pub fn resolve_payee_country(record: &PaymentRecord) -> Result<String, String> {
    let payee_account = record.payee_account.trim();
    let payee_account_type = record.payee_account_type.trim().to_uppercase();

    if !payee_account.is_empty() {
        if let Some(country) = account_country_code(&payee_account_type, payee_account) {
            return Ok(country);
        }

        // If the account type implies a country prefix, be explicit when it's invalid.
        if matches!(payee_account_type.as_str(), "IBAN" | "OBAN" | "OTHER") && payee_account.len() >= 2 {
            let prefix = &payee_account[0..2];
            if map_country_code(prefix).is_none() {
                return Err(format!(
                    "invalid country code prefix in payee account identifier: '{prefix}'"
                ));
            }
        }

        if let Some(psp_id) = record.payee_psp_id.as_deref() {
            // If a BIC is present but invalid, say so (don't silently fall back).
            let psp_id_trimmed = psp_id.trim();
            if (psp_id_trimmed.len() == 8 || psp_id_trimmed.len() == 11) && bic_country_code(psp_id_trimmed).is_none() {
                return Err(format!("invalid payee PSP BIC: '{psp_id_trimmed}'"));
            }

            if let Some(country) = bic_country_code(psp_id_trimmed) {
                return Ok(country);
            }
        }

        return Err(
            "payee account identifier does not encode country and payee PSP BIC is missing"
                .to_string(),
        );
    }

    if let Some(psp_id) = record.payee_psp_id.as_deref() {
        let psp_id_trimmed = psp_id.trim();
        if (psp_id_trimmed.len() == 8 || psp_id_trimmed.len() == 11) && bic_country_code(psp_id_trimmed).is_none() {
            return Err(format!("invalid payee PSP BIC: '{psp_id_trimmed}'"));
        }
        if let Some(country) = bic_country_code(psp_id_trimmed) {
            return Ok(country);
        }
    }

    Err("payee country cannot be derived from account identifier or payee PSP BIC".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_maps_uk_and_el() {
        assert_eq!(normalize_country_code("uk"), Some("GB".to_string()));
        assert_eq!(normalize_country_code(" EL "), Some("GR".to_string()));
        assert_eq!(normalize_country_code("gb"), Some("GB".to_string()));
        assert_eq!(normalize_country_code("gr"), Some("GR".to_string()));
    }

    #[test]
    fn bic_country_code_maps_variants() {
        // DEUT UK AA (country segment is positions 4..6)
        assert_eq!(bic_country_code("DEUTUKAA"), Some("GB".to_string()));
        assert_eq!(bic_country_code("DEUTELAA"), Some("GR".to_string()));
    }

    #[test]
    fn account_country_code_maps_variants() {
        assert_eq!(account_country_code("iban", "uk12FOO"), Some("GB".to_string()));
        assert_eq!(account_country_code("other", " elXX"), Some("GR".to_string()));
    }

    #[test]
    fn map_country_code_rejects_invalid() {
        assert_eq!(normalize_country_code("U1"), None);
        assert_eq!(normalize_country_code(""), None);
        assert_eq!(normalize_country_code("USA"), None);
    }
}
