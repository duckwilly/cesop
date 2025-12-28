use crate::analysis::{reportable_payee_keys, PayeeKey};
use crate::models::PaymentRecord;

use chrono::{Datelike, SecondsFormat, Utc};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct PeriodKey {
    year: i32,
    quarter: u8,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct ReportKey {
    period: PeriodKey,
    psp_id: String,
    psp_name: String,
}

const REPORTING_THRESHOLD: usize = 25;
const TRANSMITTING_COUNTRY_AUTO: &str = "auto";

#[derive(Debug, Clone)]
pub struct CesopReport {
    period: PeriodKey,
    pub transmitting_country: String,
    pub reporting_psp_id: String,
    pub reporting_psp_name: String,
    pub payees: Vec<PayeeGroup>,
    pub message_type_indic: String,
}

#[derive(Debug, Clone)]
pub struct PayeeAccount {
    pub id: String,
    pub account_type: String,
}

#[derive(Debug, Clone)]
pub struct PayeeGroup {
    pub payee_name: String,
    pub payee_country: String,
    pub payee_accounts: Vec<PayeeAccount>,
    pub payee_tax_id: Option<String>,
    pub payee_vat_id: Option<String>,
    pub payee_email: Option<String>,
    pub payee_web: Option<String>,
    pub payee_address_line: Option<String>,
    pub payee_city: Option<String>,
    pub payee_postcode: Option<String>,
    pub transactions: Vec<PaymentRecord>,
}

pub fn build_reports_from_csv(
    input: &Path,
    transmitting_country: &str,
) -> Result<Vec<CesopReport>, String> {
    let records = read_csv(input)?;
    if records.is_empty() {
        return Err("no records found in input CSV".to_string());
    }
    let mut psp_names: HashMap<String, String> = HashMap::new();
    let mut period_map: BTreeMap<ReportKey, Vec<PaymentRecord>> = BTreeMap::new();

    for record in records.into_iter() {
        let period = period_from_timestamp(&record.execution_time)?;
        if let Some(existing) = psp_names.get(&record.psp_id) {
            if existing != &record.psp_name {
                return Err(format!(
                    "multiple PSP names found for {}: '{}' vs '{}'",
                    record.psp_id, existing, record.psp_name
                ));
            }
        } else {
            psp_names.insert(record.psp_id.clone(), record.psp_name.clone());
        }

        let key = ReportKey {
            period,
            psp_id: record.psp_id.clone(),
            psp_name: record.psp_name.clone(),
        };
        period_map.entry(key).or_default().push(record);
    }

    let mut reports = Vec::new();
    for (key, period_records) in period_map {
        let payees = group_payees(period_records, REPORTING_THRESHOLD)?;
        let message_type_indic = if payees.is_empty() {
            "CESOP102".to_string()
        } else {
            "CESOP100".to_string()
        };
        let tx_country = resolve_transmitting_country(transmitting_country, &key.psp_id)?;

        reports.push(CesopReport {
            period: key.period,
            transmitting_country: tx_country,
            reporting_psp_id: key.psp_id,
            reporting_psp_name: key.psp_name,
            payees,
            message_type_indic,
        });
    }

    Ok(reports)
}

pub fn write_reports(reports: &[CesopReport], output_dir: &Path) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(output_dir).map_err(|err| err.to_string())?;
    let mut outputs = Vec::new();

    for report in reports {
        let filename = format!(
            "cesop_{}_Q{}_{}_{}.xml",
            report.period.year,
            report.period.quarter,
            report.transmitting_country,
            report.reporting_psp_id
        );
        let path = output_dir.join(filename);
        write_report(report, &path)?;
        outputs.push(path);
    }

    Ok(outputs)
}

fn read_csv(path: &Path) -> Result<Vec<PaymentRecord>, String> {
    let mut reader = csv::Reader::from_path(path).map_err(|err| err.to_string())?;
    let mut records = Vec::new();
    for row in reader.deserialize() {
        let record: PaymentRecord = row.map_err(|err| err.to_string())?;
        records.push(record);
    }
    Ok(records)
}

fn resolve_transmitting_country(requested: &str, psp_id: &str) -> Result<String, String> {
    if requested.eq_ignore_ascii_case(TRANSMITTING_COUNTRY_AUTO) {
        return bic_country_code(psp_id).ok_or_else(|| {
            format!(
                "cannot derive transmitting country from PSP identifier {}",
                psp_id
            )
        });
    }

    if requested.trim().is_empty() {
        return Err("transmitting country cannot be empty".to_string());
    }

    Ok(requested.trim().to_uppercase())
}

fn bic_country_code(bic: &str) -> Option<String> {
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

fn period_from_timestamp(ts: &str) -> Result<PeriodKey, String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(ts).map_err(|err| err.to_string())?;
    let month = parsed.month();
    let quarter = ((month - 1) / 3 + 1) as u8;
    Ok(PeriodKey {
        year: parsed.year(),
        quarter,
    })
}

fn group_payees(records: Vec<PaymentRecord>, threshold: usize) -> Result<Vec<PayeeGroup>, String> {
    let reportable_payees = reportable_payee_keys(&records, threshold, false);
    let mut groups: BTreeMap<PayeeKey, Vec<PaymentRecord>> = BTreeMap::new();

    for record in records {
        if record.payer_country == record.payee_country {
            continue;
        }
        let key = PayeeKey {
            payee_id: record.payee_id.clone(),
            payee_country: record.payee_country.clone(),
        };
        groups.entry(key).or_default().push(record);
    }

    let mut payees = Vec::new();
    for (payee_key, transactions) in groups {
        if !reportable_payees.contains(&payee_key) {
            continue;
        }
        let first = transactions
            .first()
            .ok_or_else(|| "missing transactions for payee".to_string())?;
        let payee_accounts = collect_payee_accounts(&transactions)?;

        payees.push(PayeeGroup {
            payee_name: first.payee_name.clone(),
            payee_country: first.payee_country.clone(),
            payee_accounts,
            payee_tax_id: first.payee_tax_id.clone(),
            payee_vat_id: first.payee_vat_id.clone(),
            payee_email: first.payee_email.clone(),
            payee_web: first.payee_web.clone(),
            payee_address_line: first.payee_address_line.clone(),
            payee_city: first.payee_city.clone(),
            payee_postcode: first.payee_postcode.clone(),
            transactions,
        });
    }

    Ok(payees)
}

fn collect_payee_accounts(transactions: &[PaymentRecord]) -> Result<Vec<PayeeAccount>, String> {
    let mut accounts: BTreeMap<String, String> = BTreeMap::new();
    for tx in transactions {
        if tx.payee_account.is_empty() {
            return Err("payee account cannot be empty".to_string());
        }
        let entry = accounts
            .entry(tx.payee_account.clone())
            .or_insert_with(|| tx.payee_account_type.clone());
        if entry != &tx.payee_account_type {
            return Err(format!(
                "payee account {} has multiple account types",
                tx.payee_account
            ));
        }
    }

    Ok(accounts
        .into_iter()
        .map(|(id, account_type)| PayeeAccount { id, account_type })
        .collect())
}

fn write_report(report: &CesopReport, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|err| err.to_string())?;
    let mut writer = Writer::new_with_indent(BufWriter::new(file), b' ', 2);

    let mut root = BytesStart::new("CESOP");
    root.push_attribute(("xmlns", "urn:ec.europa.eu:taxud:fiscalis:cesop:v1"));
    root.push_attribute(("xmlns:cm", "urn:eu:taxud:commontypes:v1"));
    root.push_attribute(("xmlns:iso", "urn:eu:taxud:isotypes:v1"));
    root.push_attribute(("version", "4.03"));
    writer
        .write_event(Event::Start(root))
        .map_err(|err| err.to_string())?;

    write_message_spec(&mut writer, report)?;
    write_payment_body(&mut writer, report)?;

    writer
        .write_event(Event::End(BytesEnd::new("CESOP")))
        .map_err(|err| err.to_string())?;

    Ok(())
}

fn write_message_spec<W: std::io::Write>(
    writer: &mut Writer<W>,
    report: &CesopReport,
) -> Result<(), String> {
    write_start(writer, "MessageSpec", &[])?;
    write_text_element(writer, "TransmittingCountry", &report.transmitting_country)?;
    write_text_element(writer, "MessageType", "PMT")?;
    write_text_element(writer, "MessageTypeIndic", &report.message_type_indic)?;
    write_text_element(writer, "MessageRefId", &uuid::Uuid::new_v4().to_string())?;

    write_start(writer, "ReportingPeriod", &[])?;
    write_text_element(writer, "Quarter", &report.period.quarter.to_string())?;
    write_text_element(writer, "Year", &report.period.year.to_string())?;
    write_end(writer, "ReportingPeriod")?;

    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    write_text_element(writer, "Timestamp", &timestamp)?;
    write_end(writer, "MessageSpec")?;
    Ok(())
}

fn write_payment_body<W: std::io::Write>(
    writer: &mut Writer<W>,
    report: &CesopReport,
) -> Result<(), String> {
    write_start(writer, "PaymentDataBody", &[])?;

    write_start(writer, "ReportingPSP", &[])?;
    write_text_element_with_attrs(
        writer,
        "PSPId",
        &report.reporting_psp_id,
        &[("PSPIdType", "BIC")],
    )?;
    write_text_element_with_attrs(
        writer,
        "Name",
        &report.reporting_psp_name,
        &[("nameType", "BUSINESS")],
    )?;
    write_end(writer, "ReportingPSP")?;

    for payee in &report.payees {
        write_reported_payee(writer, payee)?;
    }

    write_end(writer, "PaymentDataBody")?;
    Ok(())
}

fn write_reported_payee<W: std::io::Write>(
    writer: &mut Writer<W>,
    payee: &PayeeGroup,
) -> Result<(), String> {
    write_start(writer, "ReportedPayee", &[])?;
    write_text_element_with_attrs(writer, "Name", &payee.payee_name, &[("nameType", "BUSINESS")])?;
    write_text_element(writer, "Country", &payee.payee_country)?;

    write_start(writer, "Address", &[])?;
    write_text_element(writer, "cm:CountryCode", &payee.payee_country)?;
    if let Some(address_free) = build_address_free(payee) {
        write_text_element(writer, "cm:AddressFree", &address_free)?;
    }
    write_end(writer, "Address")?;

    if let Some(email) = payee.payee_email.as_deref() {
        write_text_element(writer, "EmailAddress", email)?;
    }
    if let Some(web) = payee.payee_web.as_deref() {
        write_text_element(writer, "WebPage", web)?;
    }

    write_start(writer, "TAXIdentification", &[])?;
    if let Some(vat) = payee.payee_vat_id.as_deref() {
        write_text_element_with_attrs(
            writer,
            "VATId",
            vat,
            &[("issuedBy", payee.payee_country.as_str())],
        )?;
    }
    if let Some(tax) = payee.payee_tax_id.as_deref() {
        write_text_element_with_attrs(
            writer,
            "TAXId",
            tax,
            &[
                ("issuedBy", payee.payee_country.as_str()),
                ("type", "TIN"),
            ],
        )?;
    }
    write_end(writer, "TAXIdentification")?;

    for account in &payee.payee_accounts {
        let mut attrs = vec![
            ("CountryCode", payee.payee_country.as_str()),
            ("type", account.account_type.as_str()),
        ];
        if account.account_type == "Other" {
            attrs.push(("accountIdentifierOther", "OTHER"));
        }
        write_text_element_with_attrs(writer, "AccountIdentifier", &account.id, &attrs)?;
    }

    for tx in &payee.transactions {
        write_reported_transaction(writer, tx)?;
    }

    write_start(writer, "DocSpec", &[])?;
    write_text_element(writer, "cm:DocTypeIndic", "CESOP1")?;
    write_text_element(writer, "cm:DocRefId", &uuid::Uuid::new_v4().to_string())?;
    write_end(writer, "DocSpec")?;

    write_end(writer, "ReportedPayee")?;
    Ok(())
}

fn write_reported_transaction<W: std::io::Write>(
    writer: &mut Writer<W>,
    tx: &PaymentRecord,
) -> Result<(), String> {
    let mut tx_start = BytesStart::new("ReportedTransaction");
    if tx.is_refund {
        tx_start.push_attribute(("IsRefund", "true"));
    }
    writer
        .write_event(Event::Start(tx_start))
        .map_err(|err| err.to_string())?;

    write_text_element(writer, "TransactionIdentifier", &tx.payment_id)?;
    if tx.is_refund {
        if let Some(corr) = tx.corr_payment_id.as_deref() {
            write_text_element(writer, "CorrTransactionIdentifier", corr)?;
        }
    }

    write_text_element_with_attrs(
        writer,
        "DateTime",
        &tx.execution_time,
        &[("transactionDateType", "CESOP701")],
    )?;
    write_text_element_with_attrs(
        writer,
        "Amount",
        &format_amount_for_xml(&tx.amount, tx.is_refund),
        &[("currency", tx.currency.as_str())],
    )?;

    write_start(writer, "PaymentMethod", &[])?;
    write_text_element(writer, "cm:PaymentMethodType", &tx.payment_method)?;
    if tx.payment_method == "Other" {
        write_text_element(writer, "cm:PaymentMethodOther", "Other")?;
    }
    write_end(writer, "PaymentMethod")?;

    write_text_element(
        writer,
        "InitiatedAtPhysicalPremisesOfMerchant",
        if tx.initiated_at_pos { "true" } else { "false" },
    )?;
    write_text_element_with_attrs(
        writer,
        "PayerMS",
        &tx.payer_country,
        &[("PayerMSSource", tx.payer_ms_source.as_str())],
    )?;

    writer
        .write_event(Event::End(BytesEnd::new("ReportedTransaction")))
        .map_err(|err| err.to_string())?;

    Ok(())
}

fn build_address_free(payee: &PayeeGroup) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(line) = payee.payee_address_line.as_deref() {
        if !line.is_empty() {
            parts.push(line.to_string());
        }
    }

    let city_line = match (payee.payee_postcode.as_deref(), payee.payee_city.as_deref()) {
        (Some(post), Some(city)) if !post.is_empty() && !city.is_empty() => {
            Some(format!("{} {}", post, city))
        }
        (Some(post), _) if !post.is_empty() => Some(post.to_string()),
        (_, Some(city)) if !city.is_empty() => Some(city.to_string()),
        _ => None,
    };

    if let Some(city_line) = city_line {
        parts.push(city_line);
    }

    if parts.is_empty() {
        return None;
    }

    Some(parts.join(", "))
}

fn format_amount_for_xml(amount: &str, is_refund: bool) -> String {
    let parsed = amount.parse::<f64>().unwrap_or(0.0);
    let value = parsed.abs();
    let signed = if is_refund { -value } else { value };
    format!("{:.2}", signed)
}

fn write_start<W: std::io::Write>(
    writer: &mut Writer<W>,
    name: &str,
    attrs: &[(&str, &str)],
) -> Result<(), String> {
    let mut elem = BytesStart::new(name);
    for (key, value) in attrs {
        elem.push_attribute((*key, *value));
    }
    writer
        .write_event(Event::Start(elem))
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn write_end<W: std::io::Write>(writer: &mut Writer<W>, name: &str) -> Result<(), String> {
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn write_text_element<W: std::io::Write>(
    writer: &mut Writer<W>,
    name: &str,
    text: &str,
) -> Result<(), String> {
    let elem = BytesStart::new(name);
    writer
        .write_event(Event::Start(elem))
        .map_err(|err| err.to_string())?;
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .map_err(|err| err.to_string())?;
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn write_text_element_with_attrs<W: std::io::Write>(
    writer: &mut Writer<W>,
    name: &str,
    text: &str,
    attrs: &[(&str, &str)],
) -> Result<(), String> {
    let mut elem = BytesStart::new(name);
    for (key, value) in attrs {
        elem.push_attribute((*key, *value));
    }
    writer
        .write_event(Event::Start(elem))
        .map_err(|err| err.to_string())?;
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .map_err(|err| err.to_string())?;
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(|err| err.to_string())?;
    Ok(())
}
