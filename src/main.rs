mod analysis;
mod cesop_xml;
mod correct;
mod corrupt;
mod generator;
mod location;
mod logging;
mod models;
mod preflight;
mod reference;
mod util;
mod validation;

use analysis::{analyze_threshold_csv, ThresholdReport};
use clap::{Parser, Subcommand};
use chrono::Datelike;
use cesop_xml::{build_reports_from_csv, write_reports};
use generator::{generate_records, GeneratorConfig};
use models::PaymentRecord;
use rand::Rng;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::time::Instant;
use validation::validate_with_vm;

#[derive(Parser)]
#[command(name = "cesop-demo")]
#[command(about = "CESOP synthetic data generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Generate(GenerateArgs),
    Analyze(AnalyzeArgs),
    Render(RenderArgs),
    Correct(CorrectArgs),
    Corrupt(CorruptArgs),
    Preflight(PreflightArgs),
    Validate(ValidateArgs),
}

#[derive(Parser)]
struct GenerateArgs {
    #[arg(long, default_value_t = 1200)]
    scale: usize,
    #[arg(long)]
    seed: Option<u64>,
    #[arg(long, default_value_t = 1)]
    psps: usize,
    #[arg(long, default_value_t = 0.15)]
    multi_account_ratio: f64,
    #[arg(long, default_value_t = 0.10)]
    non_eu_payee_ratio: f64,
    #[arg(long, default_value_t = 0.02)]
    no_account_payee_ratio: f64,
    #[arg(long, default_value = "data/synthetic/payments.csv")]
    output: PathBuf,
}

#[derive(Parser)]
struct AnalyzeArgs {
    #[arg(long, default_value = "data/synthetic/payments.csv")]
    input: PathBuf,
    #[arg(long, default_value_t = 25)]
    threshold: usize,
    #[arg(long, default_value_t = false)]
    include_refunds: bool,
}

#[derive(Parser)]
struct RenderArgs {
    #[arg(long, default_value = "data/synthetic/payments.csv")]
    input: PathBuf,
    #[arg(long, default_value = "data/output")]
    output_dir: PathBuf,
    #[arg(long, default_value = "auto")]
    transmitting_country: String,
    #[arg(long)]
    licensed_countries: Option<String>,
}

#[derive(Parser)]
struct CorrectArgs {
    #[arg(long, default_value = "data/synthetic/payments_invalid.csv")]
    input: PathBuf,
    #[arg(long, default_value = "data/synthetic/payments_corrected.csv")]
    output: PathBuf,
    #[arg(long)]
    seed: Option<u64>,
}

#[derive(Parser)]
struct CorruptArgs {
    #[arg(long, default_value = "data/synthetic/payments.csv")]
    input: PathBuf,
    #[arg(long, default_value = "data/synthetic/payments_invalid.csv")]
    output: PathBuf,
    #[arg(long, default_value_t = 0.02)]
    payee_error_rate: f64,
    #[arg(long, default_value_t = 0.01)]
    tx_error_rate: f64,
    #[arg(long)]
    seed: Option<u64>,
}

#[derive(Parser)]
struct PreflightArgs {
    #[arg(long, default_value = "data/synthetic/payments.csv")]
    input: PathBuf,
    #[arg(long, default_value_t = 25)]
    threshold: usize,
    #[arg(long, default_value_t = false)]
    include_refunds: bool,
}

#[derive(Parser)]
struct ValidateArgs {
    #[arg(long, default_value = "data/output")]
    input: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(
        long,
        default_value = "scripts/CESOP Validation Module/SDEV-CESOP-VM-v1.7.1/cesop-vm-application-1.7.1.jar"
    )]
    jar: PathBuf,
    #[arg(long, default_value = "java")]
    java: String,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    logging::init_logging("cesop-demo")?;
    let cli = Cli::parse();
    match cli.command {
        Command::Generate(args) => run_generate(args),
        Command::Analyze(args) => run_analyze(args),
        Command::Render(args) => run_render(args),
        Command::Correct(args) => run_correct(args),
        Command::Corrupt(args) => run_corrupt(args),
        Command::Preflight(args) => run_preflight(args),
        Command::Validate(args) => run_validate(args),
    }
}

fn run_generate(args: GenerateArgs) -> Result<(), String> {
    let (year, quarter) = resolve_year_quarter()?;
    let seed = args.seed.unwrap_or_else(random_seed);

    let derived = derive_scaled_generation(args.scale)?;
    let config = GeneratorConfig {
        records: derived.records,
        payees: derived.payees,
        micro_payees: derived.micro_payees,
        near_threshold_payees: derived.near_threshold_payees,
        large_payees: derived.large_payees,
        psps: args.psps,
        cross_border_ratio: 0.8,
        refund_ratio: 0.02,
        multi_account_ratio: args.multi_account_ratio,
        non_eu_payee_ratio: args.non_eu_payee_ratio,
        no_account_payee_ratio: args.no_account_payee_ratio,
        year,
        quarter,
    };

    log::info!(
        "Using scale {} -> payees={}, micro={}, near={}, large={}",
        args.scale,
        config.payees,
        config.micro_payees,
        config.near_threshold_payees,
        config.large_payees
    );
    log::info!(
        "Generator options: psps={}, multi_account_ratio(account+BIC)={}, non_eu_payee_ratio={}, no_account_payee_ratio={}",
        config.psps,
        config.multi_account_ratio,
        config.non_eu_payee_ratio,
        config.no_account_payee_ratio
    );

    log::info!(
        "Generating {} records across {} payees (seed {})",
        config.records,
        config.payees,
        seed
    );
    let gen_start = Instant::now();
    let records = generate_records(&config, seed)?;
    let gen_elapsed = gen_start.elapsed();
    write_csv(&args.output, &records)?;

    log::info!(
        "generated {} records for Q{} {}, seed {}, output {}",
        records.len(),
        quarter,
        year,
        seed,
        args.output.display()
    );
    emit_info_line(&format!(
        "Generation time: {} ms",
        gen_elapsed.as_millis()
    ));

    let analysis_start = Instant::now();
    let report = analyze_threshold_csv(&args.output, 25, false)?;
    let analysis_elapsed = analysis_start.elapsed();
    log_threshold_report(&report);
    emit_info_line(&format!(
        "Analysis time: {} ms",
        analysis_elapsed.as_millis()
    ));

    Ok(())
}

fn run_analyze(args: AnalyzeArgs) -> Result<(), String> {
    let analysis_start = Instant::now();
    let report = analyze_threshold_csv(&args.input, args.threshold, args.include_refunds)?;
    let analysis_elapsed = analysis_start.elapsed();
    log_threshold_report(&report);
    emit_info_line(&format!(
        "Analysis time: {} ms",
        analysis_elapsed.as_millis()
    ));
    Ok(())
}

fn run_render(args: RenderArgs) -> Result<(), String> {
    let licensed_list = match args.licensed_countries {
        Some(value) => {
            let parsed = parse_country_list(&value)?;
            if parsed.is_empty() {
                None
            } else {
                Some(parsed)
            }
        }
        None => None,
    };
    let reports = build_reports_from_csv(
        &args.input,
        &args.transmitting_country,
        licensed_list.as_deref(),
    )?;
    if reports.is_empty() {
        return Err("no reports generated (no cross-border data)".to_string());
    }

    let outputs = write_reports(&reports, &args.output_dir)?;
    emit_info_line(&format!(
        "Rendered {} report(s) to {}",
        outputs.len(),
        args.output_dir.display()
    ));
    for path in outputs {
        emit_info_line(&format!("XML output: {}", path.display()));
    }
    Ok(())
}

fn run_correct(args: CorrectArgs) -> Result<(), String> {
    let seed = args.seed.unwrap_or_else(random_seed);
    let summary = correct::correct_csv(&args.input, &args.output, seed)?;

    emit_info_line(&format!(
        "Correct: input={} output={} seed={}",
        args.input.display(),
        args.output.display(),
        seed
    ));
    emit_info_line(&format!(
        "Corrected records: {} / {}",
        summary.corrected_records, summary.total_records
    ));
    emit_info_line(&format!(
        "Corrections: payee_name={} payee_country={} account_type={} account_value={} payer_country={} payer_source={} currency={} execution_time={}",
        summary.payee_name_fixed,
        summary.payee_country_fixed,
        summary.payee_account_type_fixed,
        summary.payee_account_value_fixed,
        summary.payer_country_fixed,
        summary.payer_source_fixed,
        summary.currency_fixed,
        summary.execution_time_fixed
    ));
    Ok(())
}

fn parse_country_list(input: &str) -> Result<Vec<String>, String> {
    let mut countries: Vec<String> = Vec::new();
    for raw in input.split(',') {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let code = trimmed.to_uppercase();
        if code.len() != 2 || !code.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return Err(format!(
                "invalid country code in --licensed-countries: {}",
                trimmed
            ));
        }
        if !countries.contains(&code) {
            countries.push(code);
        }
    }
    Ok(countries)
}

fn run_corrupt(args: CorruptArgs) -> Result<(), String> {
    let seed = args.seed.unwrap_or_else(random_seed);
    let summary = corrupt::corrupt_csv(
        &args.input,
        &args.output,
        args.payee_error_rate,
        args.tx_error_rate,
        seed,
    )?;

    emit_info_line(&format!(
        "Corrupt: input={} output={} seed={}",
        args.input.display(),
        args.output.display(),
        seed
    ));
    emit_info_line(&format!(
        "Corrupt payee errors: targeted={} name_missing={} country_invalid={} account_type_invalid={} account_value_invalid={}",
        summary.payees_targeted,
        summary.payee_name_missing,
        summary.payee_country_invalid,
        summary.account_type_invalid,
        summary.account_value_invalid
    ));
    emit_info_line(&format!(
        "Corrupt tx errors: currency_invalid={} payer_country_invalid={} payer_source_invalid={}",
        summary.tx_currency_invalid,
        summary.tx_payer_country_invalid,
        summary.tx_payer_source_invalid
    ));

    Ok(())
}

fn run_preflight(args: PreflightArgs) -> Result<(), String> {
    let report = preflight::preflight_csv(&args.input, args.threshold, args.include_refunds)?;

    emit_info_line(&format!(
        "Preflight (threshold >{}): records={} cross_border={} payees={} payees_over_threshold={}",
        report.threshold,
        report.total_records,
        report.cross_border_records,
        report.total_payees,
        report.payees_over_threshold
    ));
    emit_info_line(&format!(
        "Preflight issues: errors={} warnings={}",
        report.error_count(),
        report.warning_count()
    ));

    emit_issue_summary("error", &report.issues, preflight::IssueLevel::Error);
    emit_issue_summary("warning", &report.issues, preflight::IssueLevel::Warning);

    if report.error_count() > 0 {
        return Err(format!(
            "preflight failed with {} error(s)",
            report.error_count()
        ));
    }

    Ok(())
}

fn emit_issue_summary(
    label: &str,
    issues: &[preflight::PreflightIssue],
    level: preflight::IssueLevel,
) {
    let mut counts = std::collections::HashMap::new();
    for issue in issues.iter().filter(|issue| issue.level == level) {
        *counts.entry(issue.message.as_str()).or_insert(0usize) += 1;
    }
    if counts.is_empty() {
        return;
    }

    let mut items: Vec<(&str, usize)> = counts.into_iter().collect();
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));

    let max_items = 5usize;
    for (message, count) in items.iter().take(max_items) {
        emit_info_line(&format!("Preflight {}s: {} = {}", label, message, count));
    }
    if items.len() > max_items {
        emit_info_line(&format!(
            "Preflight {}s: {} additional issue types not shown",
            label,
            items.len() - max_items
        ));
    }
}

fn run_validate(args: ValidateArgs) -> Result<(), String> {
    let result = match validate_with_vm(&args.java, &args.jar, &args.input) {
        Ok(result) => result,
        Err(err) => {
            emit_info_line("Validation failed");
            return Err(err);
        }
    };

    if let Some(output_path) = args.output {
        if let Some(parent) = output_path.parent() {
            create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        std::fs::write(&output_path, result.stdout).map_err(|err| err.to_string())?;
        emit_info_line(&format!(
            "Validation output written to {}",
            output_path.display()
        ));
    } else {
        println!("{}", result.stdout);
    }

    if !result.stderr.trim().is_empty() {
        emit_info_line(&format!("Validation warnings: {}", result.stderr.trim()));
    }

    emit_info_line("Validation successful");
    emit_info_line(&format!("Validation time: {} ms", result.duration_ms));
    Ok(())
}

fn resolve_year_quarter() -> Result<(i32, u8), String> {
    let now = chrono::Utc::now();
    let current_quarter = ((now.month() - 1) / 3 + 1) as u8;
    let year = now.year();
    let quarter = current_quarter;
    if !(1..=4).contains(&quarter) {
        return Err("quarter must be 1..4".to_string());
    }
    Ok((year, quarter))
}

fn random_seed() -> u64 {
    let mut rng = rand::rngs::OsRng;
    rng.gen()
}

fn write_csv(output: &Path, records: &[PaymentRecord]) -> Result<(), String> {
    let mut writer = csv::Writer::from_path(output).map_err(|err| err.to_string())?;
    for record in records {
        writer.serialize(record).map_err(|err| err.to_string())?;
    }
    writer.flush().map_err(|err| err.to_string())
}

#[derive(Debug, Clone)]
struct DerivedGeneration {
    records: usize,
    payees: usize,
    micro_payees: usize,
    near_threshold_payees: usize,
    large_payees: usize,
}

fn derive_scaled_generation(records: usize) -> Result<DerivedGeneration, String> {
    // Aim for ~24 transactions per payee to keep most payees below threshold.
    const TARGET_AVG: f64 = 24.0;
    let mut payees = ((records as f64) / TARGET_AVG).ceil() as usize;
    if payees == 0 {
        payees = 1;
    }

    for _ in 0..10_000 {
        let (micro, near, large) = ratio_counts(payees, records);
        let (min_total, max_total) = record_bounds(payees, micro, near, large);

        if records < min_total {
            if payees == 1 {
                break;
            }
            payees = payees.saturating_sub(1);
            continue;
        }
        if records > max_total {
            payees = payees.saturating_add(1);
            continue;
        }

        return Ok(DerivedGeneration {
            records,
            payees,
            micro_payees: micro,
            near_threshold_payees: near,
            large_payees: large,
        });
    }

    Err("could not derive a valid payee mix for the requested scale".to_string())
}

fn ratio_counts(payees: usize, records: usize) -> (usize, usize, usize) {
    const MICRO_RATIO: f64 = 0.25;
    const NEAR_RATIO: f64 = 0.10;
    const OVER_THRESHOLD_PER_RECORD: f64 = 0.0025; // 250 per 100k records

    let mut micro = ((payees as f64) * MICRO_RATIO).round() as usize;
    let mut near = ((payees as f64) * NEAR_RATIO).round() as usize;
    let mut large = ((records as f64) * OVER_THRESHOLD_PER_RECORD).round() as usize;
    if large > payees {
        large = payees;
    }

    while micro + near + large > payees {
        if micro > 0 {
            micro -= 1;
        } else if near > 0 {
            near -= 1;
        } else if large > 0 {
            large -= 1;
        } else {
            break;
        }
    }

    (micro, near, large)
}

fn record_bounds(
    payees: usize,
    micro: usize,
    near: usize,
    large: usize,
) -> (usize, usize) {
    let remaining = payees.saturating_sub(micro + near + large);
    let small = remaining / 2;
    let mid = remaining - small;
    let near_below = near / 2;
    let near_above = near - near_below;

    let min_total = micro * 1
        + small * 6
        + mid * 16
        + near_below * 24
        + near_above * 26
        + large * 80;
    let max_total = micro * 5
        + small * 20
        + mid * 24
        + near_below * 25
        + near_above * 27
        + large * 140;

    (min_total, max_total)
}

fn log_threshold_report(report: &ThresholdReport) {
    emit_info_line(&format!(
        "Threshold check (>{}) cross-border={} total_records={} payees={}",
        report.threshold, report.cross_border_records, report.total_records, report.total_payees
    ));
    emit_info_line(&format!(
        "Payees over threshold: {}",
        report.payees_over_threshold
    ));
}

fn emit_info_line(message: &str) {
    if log::log_enabled!(log::Level::Info) {
        log::info!("{}", message);
    } else {
        println!("{message}");
    }
}
