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

    Ok(ValidationResult {
        stdout,
        stderr,
        duration_ms,
    })
}
