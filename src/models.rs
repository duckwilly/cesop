use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRecord {
    pub payment_id: String,
    pub execution_time: String,
    pub amount: String,
    pub currency: String,
    pub payer_country: String,
    pub payer_ms_source: String,
    pub payee_country: String,
    pub payee_id: String,
    pub payee_name: String,
    pub payee_account: String,
    pub payee_account_type: String,
    #[serde(default)]
    pub payee_tax_id: Option<String>,
    #[serde(default)]
    pub payee_vat_id: Option<String>,
    #[serde(default)]
    pub payee_email: Option<String>,
    #[serde(default)]
    pub payee_web: Option<String>,
    #[serde(default)]
    pub payee_address_line: Option<String>,
    #[serde(default)]
    pub payee_city: Option<String>,
    #[serde(default)]
    pub payee_postcode: Option<String>,
    pub payment_method: String,
    pub initiated_at_pos: bool,
    pub is_refund: bool,
    #[serde(default)]
    pub corr_payment_id: Option<String>,
    #[serde(default)]
    pub psp_role: Option<String>,
    #[serde(default)]
    pub payee_psp_id: Option<String>,
    #[serde(default)]
    pub payee_psp_name: Option<String>,
    pub psp_id: String,
    pub psp_name: String,
}
