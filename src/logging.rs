use chrono::Local;
use std::path::PathBuf;
use std::sync::Once;

static INIT: Once = Once::new();

pub fn init_logging(app_name: &str) -> Result<(), String> {
    let mut init_result: Result<(), String> = Ok(());
    INIT.call_once(|| {
        if let Err(err) = init_logging_inner(app_name) {
            init_result = Err(err);
        }
    });
    init_result
}

fn init_logging_inner(app_name: &str) -> Result<(), String> {
    let level = std::env::var("CESOP_LOG_LEVEL")
        .or_else(|_| std::env::var("RUST_LOG"))
        .unwrap_or_else(|_| "info".to_string());
    let level = level
        .parse::<log::LevelFilter>()
        .unwrap_or(log::LevelFilter::Info);

    let log_dir = std::env::var("CESOP_LOG_DIR").ok();
    let log_dir = match log_dir.as_deref() {
        Some("off") | Some("none") | Some("") => None,
        Some(path) => Some(PathBuf::from(path)),
        None => Some(PathBuf::from("logs")),
    };

    let mut dispatch = fern::Dispatch::new()
        .level(level)
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} | {:<5} | {} | {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .chain(std::io::stdout());

    if let Some(dir) = log_dir {
        std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let date = Local::now().format("%Y_%m_%d");
        let file_path = dir.join(format!("{app_name}-{date}.log"));
        dispatch = dispatch.chain(fern::log_file(file_path).map_err(|err| err.to_string())?);
    }

    dispatch.apply().map_err(|err| err.to_string())
}
