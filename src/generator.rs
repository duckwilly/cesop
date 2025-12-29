use crate::location::bic_country_code;
use crate::models::PaymentRecord;
use crate::reference::{currency_for_country, iban_length, is_eu_member_state, EU_MEMBER_STATES};
use crate::util::{
    format_amount, iban_check_digits, random_alphanum_upper, random_digits, random_upper_letters,
    slugify,
};

use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet};

pub struct GeneratorConfig {
    pub records: usize,
    pub payees: usize,
    pub micro_payees: usize,
    pub near_threshold_payees: usize,
    pub large_payees: usize,
    pub psps: usize,
    pub cross_border_ratio: f64,
    pub refund_ratio: f64,
    pub multi_account_ratio: f64,
    pub non_eu_payee_ratio: f64,
    pub no_account_payee_ratio: f64,
    pub year: i32,
    pub quarter: u8,
}

#[derive(Clone)]
struct PayeeSegment {
    label: &'static str,
    min_tx: usize,
    max_tx: usize,
    amount_min: f64,
    amount_max: f64,
}

struct PayeePlan {
    segment: PayeeSegment,
}

#[derive(Clone)]
struct PayeeAccount {
    id: String,
    account_type: String,
}

struct PayeeProfile {
    id: String,
    name: String,
    amount_min: f64,
    amount_max: f64,
    country: String,
    accounts: Vec<PayeeAccount>,
    tax_id: Option<String>,
    vat_id: Option<String>,
    email: Option<String>,
    web: Option<String>,
    address_line: Option<String>,
    city: Option<String>,
    postcode: Option<String>,
    payee_psp_id: String,
    payee_psp_name: String,
    reporting_psp_id: String,
    reporting_psp_name: String,
    psp_role: String,
}

#[derive(Clone)]
struct PspProfile {
    id: String,
    name: String,
}

const NON_EU_PAYEE_COUNTRIES: &[&str] = &["GB", "NO", "CH", "IS", "LI", "US", "CA"];

const COMPANY_PREFIX: &[&str] = &[
    "Silver", "North", "Blue", "Cobalt", "Summit", "Urban", "Prime", "Atlas", "Green", "Nova",
    "Bright", "Vertex", "Golden", "River", "Oak", "Pioneer", "Harbor", "Stone", "Apex", "Cedar",
];

const COMPANY_NOUN: &[&str] = &[
    "Trading", "Supply", "Commerce", "Retail", "Imports", "Exports", "Foods", "Devices",
    "Logistics", "Textiles", "Systems", "Networks", "Studio", "Labs", "Market", "Tools",
];

const COMPANY_SUFFIXES: &[&str] = &[
    "Analytics",
    "Architects",
    "Associates",
    "Capital",
    "Collective",
    "Consulting",
    "Dynamics",
    "Enterprises",
    "Forge",
    "Guild",
    "Holdings",
    "Industries",
    "Innovation",
    "Labs",
    "Logistics",
    "Partners",
    "Solutions",
    "Studios",
    "Systems",
    "Technologies",
    "Ventures",
    "Works",
];

const COMPANY_LEGAL_SUFFIXES: &[&str] = &["BV", "NV", "Ltd", "LLC", "Group"];

const STREET_NAMES: &[&str] = &[
    "Market", "Station", "Oak", "River", "Park", "Hill", "Lake", "Maple", "Cedar", "High",
    "Broad", "King", "Queen", "Mill", "Garden", "Main", "North", "South", "West", "East",
];

const CITIES: &[&str] = &[
    "Dublin", "Berlin", "Paris", "Madrid", "Rome", "Lisbon", "Prague", "Vienna", "Warsaw",
    "Athens", "Helsinki", "Stockholm", "Copenhagen", "Brussels", "Amsterdam", "Luxembourg",
    "Riga", "Vilnius", "Tallinn", "Zagreb", "Sofia", "Bucharest", "Budapest", "Ljubljana",
    "Valletta",
];

const PAYMENT_METHODS: &[&str] = &[
    "Card payment",
    "Bank transfer",
    "Direct debit",
    "E-money",
    "Money Remittance",
    "Marketplace",
    "Intermediary",
];

const PSP_NAMES: &[&str] = &[
    "Northshore Payments",
    "Atlas Pay",
    "BlueBridge PSP",
    "Harborline Processing",
    "Summit Payments",
];

const PSP_ROLE_PAYEE: &str = "PAYEE";
const PSP_ROLE_PAYER: &str = "PAYER";

pub fn generate_records(
    config: &GeneratorConfig,
    seed: u64,
) -> Result<Vec<PaymentRecord>, String> {
    validate_config(config)?;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut plans = build_payee_plans(config)?;
    plans.shuffle(&mut rng);
    let counts = allocate_counts(&mut rng, &plans, config.records)?;

    let mut company_cores = load_company_cores().unwrap_or_default();
    if company_cores.is_empty() {
        company_cores = default_company_cores();
    }

    let psps = build_psps(&mut rng, config.psps)?;
    let non_eu_psps = build_non_eu_psps(&mut rng, (config.psps / 2).max(1))?;
    let payees = build_payees(
        &mut rng,
        &plans,
        &mut company_cores,
        &psps,
        &non_eu_psps,
        config.multi_account_ratio,
        config.non_eu_payee_ratio,
        config.no_account_payee_ratio,
    );

    let (period_start, period_end) = quarter_bounds(config.year, config.quarter)?;
    let mut seen_by_payee: HashMap<String, Vec<String>> = HashMap::new();

    let mut records = Vec::with_capacity(config.records);
    for (idx, payee) in payees.iter().enumerate() {
        let count = counts[idx];
        for _ in 0..count {
            let is_refund = rng.gen_bool(config.refund_ratio);
            let corr_payment_id = if is_refund {
                seen_by_payee
                    .get(&payee.id)
                    .and_then(|ids| ids.choose(&mut rng).cloned())
            } else {
                None
            };
            let is_refund = is_refund && corr_payment_id.is_some();

            let payment_id = uuid::Uuid::new_v4().to_string();
            seen_by_payee
                .entry(payee.id.clone())
                .or_default()
                .push(payment_id.clone());

            let payer_country =
                pick_payer_country(&mut rng, &payee.country, config.cross_border_ratio);
            let amount_value = rng.gen_range(payee.amount_min..payee.amount_max);
            let currency = currency_for_country(&payer_country).to_string();
            let execution_time = random_datetime(&mut rng, period_start, period_end)
                .to_rfc3339_opts(SecondsFormat::Millis, true);

            let payment_method = PAYMENT_METHODS
                .choose(&mut rng)
                .unwrap_or(&"Card payment")
                .to_string();

            let initiated_at_pos = if payment_method == "Card payment" {
                rng.gen_bool(0.7)
            } else {
                rng.gen_bool(0.2)
            };
            let payer_ms_source = pick_payer_ms_source(&mut rng).to_string();
            let (payee_account, payee_account_type) = if let Some(account) =
                payee.accounts.choose(&mut rng)
            {
                (account.id.clone(), account.account_type.clone())
            } else {
                (String::new(), String::new())
            };

            records.push(PaymentRecord {
                payment_id,
                execution_time,
                amount: format_amount(amount_value),
                currency,
                payer_country,
                payer_ms_source,
                payee_country: payee.country.clone(),
                payee_id: payee.id.clone(),
                payee_name: payee.name.clone(),
                payee_account,
                payee_account_type,
                payee_tax_id: payee.tax_id.clone(),
                payee_vat_id: payee.vat_id.clone(),
                payee_email: payee.email.clone(),
                payee_web: payee.web.clone(),
                payee_address_line: payee.address_line.clone(),
                payee_city: payee.city.clone(),
                payee_postcode: payee.postcode.clone(),
                payment_method,
                initiated_at_pos,
                is_refund,
                corr_payment_id,
                psp_role: Some(payee.psp_role.clone()),
                payee_psp_id: Some(payee.payee_psp_id.clone()),
                payee_psp_name: Some(payee.payee_psp_name.clone()),
                psp_id: payee.reporting_psp_id.clone(),
                psp_name: payee.reporting_psp_name.clone(),
            });
        }
    }

    records.shuffle(&mut rng);
    Ok(records)
}

fn validate_config(config: &GeneratorConfig) -> Result<(), String> {
    if config.payees == 0 {
        return Err("payees must be greater than 0".to_string());
    }
    if config.psps == 0 {
        return Err("psps must be greater than 0".to_string());
    }
    if config.micro_payees + config.near_threshold_payees + config.large_payees > config.payees {
        return Err("micro/near/large payees cannot exceed total payees".to_string());
    }
    if !(1..=4).contains(&config.quarter) {
        return Err("quarter must be 1..4".to_string());
    }
    if !(0.0..=1.0).contains(&config.cross_border_ratio) {
        return Err("cross_border_ratio must be 0..1".to_string());
    }
    if !(0.0..=1.0).contains(&config.refund_ratio) {
        return Err("refund_ratio must be 0..1".to_string());
    }
    if !(0.0..=1.0).contains(&config.multi_account_ratio) {
        return Err("multi_account_ratio must be 0..1".to_string());
    }
    if !(0.0..=1.0).contains(&config.non_eu_payee_ratio) {
        return Err("non_eu_payee_ratio must be 0..1".to_string());
    }
    if !(0.0..=1.0).contains(&config.no_account_payee_ratio) {
        return Err("no_account_payee_ratio must be 0..1".to_string());
    }
    Ok(())
}

fn build_payee_plans(config: &GeneratorConfig) -> Result<Vec<PayeePlan>, String> {
    let payees = config.payees;
    let micro = config.micro_payees;
    let near = config.near_threshold_payees;
    let large = config.large_payees;

    if micro + near + large > payees {
        return Err("micro/near/large payees cannot exceed total payees".to_string());
    }

    let remaining = payees - (micro + near + large);
    let small = remaining / 2;
    let mid = remaining - small;

    let near_below = near / 2;
    let near_above = near - near_below;

    let mut plans = Vec::with_capacity(payees);
    for _ in 0..micro {
        plans.push(PayeePlan {
            segment: segment_micro(),
        });
    }
    for _ in 0..small {
        plans.push(PayeePlan {
            segment: segment_small(),
        });
    }
    for _ in 0..mid {
        plans.push(PayeePlan { segment: segment_mid() });
    }
    for _ in 0..near_below {
        plans.push(PayeePlan {
            segment: segment_near_below(),
        });
    }
    for _ in 0..near_above {
        plans.push(PayeePlan {
            segment: segment_near_above(),
        });
    }
    for _ in 0..large {
        plans.push(PayeePlan {
            segment: segment_large(),
        });
    }

    Ok(plans)
}

fn allocate_counts<R: Rng + ?Sized>(
    rng: &mut R,
    plans: &[PayeePlan],
    total_records: usize,
) -> Result<Vec<usize>, String> {
    let min_total: usize = plans.iter().map(|plan| plan.segment.min_tx).sum();
    let max_total: usize = plans.iter().map(|plan| plan.segment.max_tx).sum();

    if total_records < min_total || total_records > max_total {
        return Err(format!(
            "records must be between {} and {} for the chosen parameters",
            min_total, max_total
        ));
    }

    let mut counts: Vec<usize> = plans.iter().map(|plan| plan.segment.min_tx).collect();
    let mut remaining = total_records - min_total;

    while remaining > 0 {
        let idx = rng.gen_range(0..plans.len());
        if counts[idx] < plans[idx].segment.max_tx {
            counts[idx] += 1;
            remaining -= 1;
        }
    }

    Ok(counts)
}

fn build_payees<R: Rng + ?Sized>(
    rng: &mut R,
    plans: &[PayeePlan],
    company_cores: &mut Vec<String>,
    psps: &[PspProfile],
    non_eu_psps: &[PspProfile],
    multi_account_ratio: f64,
    non_eu_payee_ratio: f64,
    no_account_payee_ratio: f64,
) -> Vec<PayeeProfile> {
    let mut payees = Vec::with_capacity(plans.len());
    for (idx, plan) in plans.iter().enumerate() {
        let mut country = pick_payee_country(rng, non_eu_payee_ratio);
        let core = pick_company_core(rng, company_cores);
        let name = build_company_name(rng, &core);
        let slug = slugify(&name);
        let payee_psp_is_eu = is_eu_member_state(&country) || non_eu_psps.is_empty();
        let payee_psp = if payee_psp_is_eu {
            psps.choose(rng).unwrap_or_else(|| &psps[0])
        } else {
            non_eu_psps
                .choose(rng)
                .unwrap_or_else(|| &non_eu_psps[0])
        };
        let payee_psp_country =
            bic_country_code(&payee_psp.id).unwrap_or_else(|| country.clone());
        let has_account = !rng.gen_bool(no_account_payee_ratio);
        let accounts = if has_account {
            build_payee_accounts(rng, &country, multi_account_ratio)
        } else {
            Vec::new()
        };
        if !has_account {
            country = payee_psp_country;
        }
        let (reporting_psp, psp_role) = if payee_psp_is_eu {
            (payee_psp, PSP_ROLE_PAYEE)
        } else {
            let payer_psp = psps.choose(rng).unwrap_or(payee_psp);
            (payer_psp, PSP_ROLE_PAYER)
        };

        let (tax_chance, vat_chance) = match plan.segment.label {
            "micro" => (0.2, 0.4),
            "small" => (0.35, 0.6),
            "mid" => (0.5, 0.75),
            "near_threshold_below" => (0.55, 0.8),
            "near_threshold_above" => (0.6, 0.85),
            "large" => (0.8, 0.95),
            _ => (0.4, 0.6),
        };

        let tax_id = if rng.gen_bool(tax_chance) {
            Some(format!("TAX{}{}", country, random_digits(rng, 8)))
        } else {
            None
        };
        let vat_id = if is_eu_member_state(&country) && rng.gen_bool(vat_chance) {
            Some(format!("{}{}", country, random_digits(rng, 9)))
        } else {
            None
        };

        let street_num = rng.gen_range(1..250);
        let street = STREET_NAMES.choose(rng).unwrap_or(&"Market");
        let address_line = Some(format!("{} {} St", street_num, street));
        let city = CITIES.choose(rng).unwrap_or(&"Berlin").to_string();
        let postcode = random_digits(rng, 5);

        payees.push(PayeeProfile {
            id: format!("MER{:06}", idx + 1),
            name,
            amount_min: plan.segment.amount_min,
            amount_max: plan.segment.amount_max,
            country,
            accounts,
            tax_id,
            vat_id,
            email: Some(format!("billing@{}.example", slug)),
            web: Some(format!("https://{}.example", slug)),
            address_line,
            city: Some(city),
            postcode: Some(postcode),
            payee_psp_id: payee_psp.id.clone(),
            payee_psp_name: payee_psp.name.clone(),
            reporting_psp_id: reporting_psp.id.clone(),
            reporting_psp_name: reporting_psp.name.clone(),
            psp_role: psp_role.to_string(),
        });
    }

    payees
}

fn build_psp<R: Rng + ?Sized>(rng: &mut R) -> PspProfile {
    let name = PSP_NAMES.choose(rng).unwrap_or(&"Atlas Pay");
    let bic = generate_bic(rng);
    PspProfile {
        id: bic,
        name: name.to_string(),
    }
}

fn build_psp_for_country<R: Rng + ?Sized>(rng: &mut R, country: &str) -> PspProfile {
    let name = PSP_NAMES.choose(rng).unwrap_or(&"Atlas Pay");
    let bic = generate_bic_for_country(rng, country);
    PspProfile {
        id: bic,
        name: name.to_string(),
    }
}

fn build_non_eu_psp<R: Rng + ?Sized>(rng: &mut R) -> PspProfile {
    let name = PSP_NAMES.choose(rng).unwrap_or(&"Atlas Pay");
    let country = NON_EU_PAYEE_COUNTRIES
        .choose(rng)
        .unwrap_or(&"GB")
        .to_string();
    let bic = generate_bic_for_country(rng, &country);
    PspProfile {
        id: bic,
        name: name.to_string(),
    }
}

fn build_psps<R: Rng + ?Sized>(rng: &mut R, count: usize) -> Result<Vec<PspProfile>, String> {
    let mut psps = Vec::with_capacity(count);
    let mut seen = HashSet::new();
    let unique_targets = count.min(EU_MEMBER_STATES.len());
    let mut countries: Vec<&str> = EU_MEMBER_STATES.iter().copied().collect();
    countries.shuffle(rng);

    for country in countries.into_iter().take(unique_targets) {
        let psp = build_psp_for_country(rng, country);
        if seen.insert(psp.id.clone()) {
            psps.push(psp);
        }
    }

    while psps.len() < count {
        let psp = build_psp(rng);
        if seen.insert(psp.id.clone()) {
            psps.push(psp);
        }
        if seen.len() > count * 10 {
            return Err("failed to generate unique PSP identifiers".to_string());
        }
    }

    Ok(psps)
}

fn build_non_eu_psps<R: Rng + ?Sized>(
    rng: &mut R,
    count: usize,
) -> Result<Vec<PspProfile>, String> {
    let mut psps = Vec::with_capacity(count);
    let mut seen = HashSet::new();

    while psps.len() < count {
        let psp = build_non_eu_psp(rng);
        if seen.insert(psp.id.clone()) {
            psps.push(psp);
        }
        if seen.len() > count * 10 {
            return Err("failed to generate unique non-eu PSP identifiers".to_string());
        }
    }

    Ok(psps)
}

fn segment_micro() -> PayeeSegment {
    PayeeSegment {
        label: "micro",
        min_tx: 1,
        max_tx: 5,
        amount_min: 5.0,
        amount_max: 60.0,
    }
}

fn segment_small() -> PayeeSegment {
    PayeeSegment {
        label: "small",
        min_tx: 6,
        max_tx: 20,
        amount_min: 10.0,
        amount_max: 160.0,
    }
}

fn segment_mid() -> PayeeSegment {
    PayeeSegment {
        label: "mid",
        min_tx: 16,
        max_tx: 24,
        amount_min: 25.0,
        amount_max: 450.0,
    }
}

fn segment_near_below() -> PayeeSegment {
    PayeeSegment {
        label: "near_threshold_below",
        min_tx: 24,
        max_tx: 25,
        amount_min: 20.0,
        amount_max: 300.0,
    }
}

fn segment_near_above() -> PayeeSegment {
    PayeeSegment {
        label: "near_threshold_above",
        min_tx: 26,
        max_tx: 27,
        amount_min: 20.0,
        amount_max: 300.0,
    }
}

fn segment_large() -> PayeeSegment {
    PayeeSegment {
        label: "large",
        min_tx: 80,
        max_tx: 140,
        amount_min: 120.0,
        amount_max: 2500.0,
    }
}

fn load_company_cores() -> Result<Vec<String>, String> {
    let path = std::path::Path::new("data/reference/company_cores.txt");
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.to_string()),
    };

    let mut names = Vec::new();
    for line in contents.lines() {
        let name = line.trim();
        if name.is_empty() || name.starts_with('#') {
            continue;
        }
        names.push(name.to_string());
    }
    Ok(names)
}

fn default_company_cores() -> Vec<String> {
    let mut cores = Vec::new();
    for prefix in COMPANY_PREFIX {
        for noun in COMPANY_NOUN {
            cores.push(format!("{} {}", prefix, noun));
        }
    }
    cores
}

fn pick_company_core<R: Rng + ?Sized>(rng: &mut R, pool: &mut Vec<String>) -> String {
    if pool.is_empty() {
        return random_company_core(rng);
    }

    let idx = rng.gen_range(0..pool.len());
    pool.swap_remove(idx)
}

fn random_company_core<R: Rng + ?Sized>(rng: &mut R) -> String {
    let prefix = COMPANY_PREFIX.choose(rng).unwrap_or(&"Silver");
    let noun = COMPANY_NOUN.choose(rng).unwrap_or(&"Trading");
    format!("{} {}", prefix, noun)
}

fn build_company_name<R: Rng + ?Sized>(rng: &mut R, core: &str) -> String {
    let suffix = COMPANY_SUFFIXES.choose(rng).unwrap_or(&"Systems");
    let legal = COMPANY_LEGAL_SUFFIXES.choose(rng).unwrap_or(&"Ltd");
    format!("{} {} {}", core, suffix, legal)
}

fn generate_iban<R: Rng + ?Sized>(rng: &mut R, country: &str) -> String {
    let length = iban_length(country).unwrap_or(22);
    let bban_len = length.saturating_sub(4);
    let bban = random_digits(rng, bban_len);
    let check = iban_check_digits(country, &bban).unwrap_or_else(|_| "00".to_string());
    format!("{}{}{}", country, check, bban)
}

fn generate_bic_for_country<R: Rng + ?Sized>(rng: &mut R, country: &str) -> String {
    let bank = random_upper_letters(rng, 4);
    let location = random_alphanum_upper(rng, 2);
    let branch = if rng.gen_bool(0.7) {
        Some(random_alphanum_upper(rng, 3))
    } else {
        None
    };

    match branch {
        Some(branch) => format!("{}{}{}{}", bank, country, location, branch),
        None => format!("{}{}{}", bank, country, location),
    }
}

fn generate_bic<R: Rng + ?Sized>(rng: &mut R) -> String {
    let country = EU_MEMBER_STATES
        .choose(rng)
        .unwrap_or(&"DE")
        .to_string();
    generate_bic_for_country(rng, &country)
}


fn pick_payee_country<R: Rng + ?Sized>(rng: &mut R, non_eu_ratio: f64) -> String {
    if rng.gen_bool(non_eu_ratio) {
        NON_EU_PAYEE_COUNTRIES
            .choose(rng)
            .unwrap_or(&"GB")
            .to_string()
    } else {
        EU_MEMBER_STATES.choose(rng).unwrap_or(&"DE").to_string()
    }
}

fn build_payee_accounts<R: Rng + ?Sized>(
    rng: &mut R,
    country: &str,
    multi_account_ratio: f64,
) -> Vec<PayeeAccount> {
    let mut accounts = Vec::with_capacity(2);
    let (id, account_type) = generate_account_identifier(rng, country);
    accounts.push(PayeeAccount { id, account_type });

    if rng.gen_bool(multi_account_ratio) {
        accounts.push(PayeeAccount {
            id: generate_bic_for_country(rng, country),
            account_type: "BIC".to_string(),
        });
    }

    accounts
}

fn generate_account_identifier<R: Rng + ?Sized>(rng: &mut R, country: &str) -> (String, String) {
    if iban_length(country).is_some() {
        (generate_iban(rng, country), "IBAN".to_string())
    } else {
        (
            format!("{}{}", country, random_alphanum_upper(rng, 12)),
            "Other".to_string(),
        )
    }
}

fn pick_payer_ms_source<R: Rng + ?Sized>(rng: &mut R) -> &'static str {
    let roll = rng.gen::<f64>();
    if roll < 0.8 {
        "IBAN"
    } else if roll < 0.95 {
        "BIC"
    } else {
        "Other"
    }
}

fn pick_payer_country<R: Rng + ?Sized>(
    rng: &mut R,
    payee_country: &str,
    cross_border_ratio: f64,
) -> String {
    if !is_eu_member_state(payee_country) {
        return EU_MEMBER_STATES
            .choose(rng)
            .unwrap_or(&"FR")
            .to_string();
    }
    if rng.gen_bool(cross_border_ratio) {
        loop {
            let candidate = EU_MEMBER_STATES.choose(rng).unwrap_or(&"FR");
            if *candidate != payee_country {
                return candidate.to_string();
            }
        }
    } else {
        payee_country.to_string()
    }
}

fn quarter_bounds(year: i32, quarter: u8) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
    let (start_month, next_year, next_month) = match quarter {
        1 => (1, year, 4),
        2 => (4, year, 7),
        3 => (7, year, 10),
        4 => (10, year + 1, 1),
        _ => return Err("quarter must be 1..4".to_string()),
    };

    let start = Utc
        .with_ymd_and_hms(year, start_month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| "invalid quarter start date".to_string())?;
    let end = Utc
        .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| "invalid quarter end date".to_string())?;

    Ok((start, end))
}

fn random_datetime<R: Rng + ?Sized>(
    rng: &mut R,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> DateTime<Utc> {
    let start_ts = start.timestamp();
    let end_ts = end.timestamp();
    let secs = rng.gen_range(start_ts..end_ts);
    let nanos = rng.gen_range(0..1_000_000_000);
    Utc.timestamp_opt(secs, nanos as u32)
        .single()
        .unwrap_or(start)
}
