pub const EU_MEMBER_STATES: &[&str] = &[
    "AT", "BE", "BG", "HR", "CY", "CZ", "DK", "EE", "FI", "FR", "DE", "GR", "HU", "IE", "IT",
    "LV", "LT", "LU", "MT", "NL", "PL", "PT", "RO", "SK", "SI", "ES", "SE",
];

pub const IBAN_LENGTHS: &[(&str, usize)] = &[
    ("AT", 20),
    ("BE", 16),
    ("BG", 22),
    ("HR", 21),
    ("CY", 28),
    ("CZ", 24),
    ("DK", 18),
    ("EE", 20),
    ("FI", 18),
    ("FR", 27),
    ("DE", 22),
    ("GR", 27),
    ("HU", 28),
    ("IE", 22),
    ("IT", 27),
    ("LV", 21),
    ("LT", 20),
    ("LU", 20),
    ("MT", 31),
    ("NL", 18),
    ("PL", 28),
    ("PT", 25),
    ("RO", 24),
    ("SK", 24),
    ("SI", 19),
    ("ES", 24),
    ("SE", 24),
    ("CH", 21),
    ("GB", 22),
    ("IS", 26),
    ("LI", 21),
    ("NO", 15),
];

pub const ACCOUNT_IDENTIFIER_TYPES: &[&str] = &["IBAN", "OBAN", "BIC", "Other"];

pub fn iban_length(country: &str) -> Option<usize> {
    IBAN_LENGTHS
        .iter()
        .find(|(code, _)| *code == country)
        .map(|(_, len)| *len)
}

pub fn is_eu_member_state(code: &str) -> bool {
    EU_MEMBER_STATES.iter().any(|ms| *ms == code)
}

pub fn currency_for_country(country: &str) -> &'static str {
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
