use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub struct ValidationResult {
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
}

pub fn validate_with_vm(
    java_bin: &str,
    jar_path: &Path,
    input: &Path,
) -> Result<ValidationResult, String> {
    if !jar_path.exists() {
        return Err(format!("Validation module jar not found: {}", jar_path.display()));
    }
    if !input.exists() {
        return Err(format!("Input file not found: {}", input.display()));
    }

    let start = Instant::now();
    let output = Command::new(java_bin)
        .arg("-jar")
        .arg(jar_path)
        .arg(input)
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "Java runtime not found (expected `{}`). Install Java or set --java.",
                    java_bin
                )
            } else {
                err.to_string()
            }
        })?;

    let duration_ms = start.elapsed().as_millis();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let details = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        return Err(format!("Validation failed: {}", details));
    }

    if let Some(summary) = read_validation_summary(input)? {
        if summary.has_rejections() {
            return Err(format!(
                "Validation rejected: validated={} partial={} full={}",
                summary.validated, summary.partially_rejected, summary.fully_rejected
            ));
        }
    }

    Ok(ValidationResult {
        stdout,
        stderr,
        duration_ms,
    })
}

#[derive(Default)]
struct ValidationSummary {
    validated: usize,
    partially_rejected: usize,
    fully_rejected: usize,
}

impl ValidationSummary {
    fn has_rejections(&self) -> bool {
        self.partially_rejected > 0 || self.fully_rejected > 0
    }
}

fn read_validation_summary(input: &Path) -> Result<Option<ValidationSummary>, String> {
    let mut candidates = Vec::new();
    if input.is_dir() {
        candidates.push(input.join("validation_output.csv"));
    } else if let Some(parent) = input.parent() {
        candidates.push(parent.join("validation_output.csv"));
    }

    let path = candidates.into_iter().find(|path| path.exists());
    let Some(path) = path else {
        return Ok(None);
    };

    let mut reader = csv::Reader::from_path(&path).map_err(|err| err.to_string())?;
    let mut summary = ValidationSummary::default();

    for result in reader.records() {
        let record = result.map_err(|err| err.to_string())?;
        if let Some(status) = record.get(3) {
            match status.trim() {
                "VALIDATED" => summary.validated += 1,
                "PARTIALLY_REJECTED" => summary.partially_rejected += 1,
                "FULLY_REJECTED" => summary.fully_rejected += 1,
                _ => {}
            }
        }
    }

    Ok(Some(summary))
}
