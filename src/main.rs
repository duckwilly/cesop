mod analysis;
mod cesop_xml;
mod generator;
mod logging;
mod models;
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
    Validate(ValidateArgs),
}

#[derive(Parser)]
struct GenerateArgs {
    #[arg(long, default_value_t = 1200)]
    scale: usize,
    #[arg(long)]
    seed: Option<u64>,
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
    #[arg(long, default_value = "DE")]
    transmitting_country: String,
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
        cross_border_ratio: 0.8,
        refund_ratio: 0.02,
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
    let reports = build_reports_from_csv(&args.input, &args.transmitting_country)?;
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
    // Aim for ~30 transactions per payee to keep a mix around the 25 threshold.
    const TARGET_AVG: f64 = 30.0;
    let mut payees = ((records as f64) / TARGET_AVG).ceil() as usize;
    if payees == 0 {
        payees = 1;
    }

    for _ in 0..10_000 {
        let (micro, near, large) = ratio_counts(payees);
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

fn ratio_counts(payees: usize) -> (usize, usize, usize) {
    let mut micro = ((payees as f64) * 0.20).round() as usize;
    let mut near = ((payees as f64) * 0.15).round() as usize;
    let mut large = ((payees as f64) * 0.10).round() as usize;

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
        + mid * 30
        + near_below * 24
        + near_above * 26
        + large * 80;
    let max_total = micro * 5
        + small * 20
        + mid * 50
        + near_below * 25
        + near_above * 30
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
