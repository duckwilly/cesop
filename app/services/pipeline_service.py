import csv
import os
import random
import re
import shutil
import subprocess
import time
from threading import Timer
from datetime import datetime
from pathlib import Path

from app.core.config import CESOP_JAR, DATA_DIR, REPO_ROOT

EU_MEMBER_STATES = [
    "AT",
    "BE",
    "BG",
    "HR",
    "CY",
    "CZ",
    "DK",
    "EE",
    "FI",
    "FR",
    "DE",
    "GR",
    "HU",
    "IE",
    "IT",
    "LV",
    "LT",
    "LU",
    "MT",
    "NL",
    "PL",
    "PT",
    "RO",
    "SK",
    "SI",
    "ES",
    "SE",
]
EU_MEMBER_STATE_SET = set(EU_MEMBER_STATES)


def resolve_cesop_command() -> list[str]:
    env_bin = os.environ.get("CESOP_BIN")
    if env_bin:
        return [env_bin]

    release_bin = REPO_ROOT / "target" / "release" / "cesop-demo"
    debug_bin = REPO_ROOT / "target" / "debug" / "cesop-demo"
    if release_bin.exists():
        return [str(release_bin)]
    if debug_bin.exists():
        return [str(debug_bin)]

    return ["cargo", "run", "--quiet", "--"]


def resolve_cleanup_ttl_seconds() -> int:
    value = os.environ.get("CESOP_DEMO_TTL_SECONDS", "300")
    try:
        return max(0, int(value))
    except ValueError:
        return 300


def resolve_license_count(scale: int) -> int:
    value = os.environ.get("CESOP_DEMO_LICENSED_COUNT")
    if value:
        try:
            parsed = int(value)
        except ValueError:
            parsed = 6
        return max(1, min(parsed, 27))

    if scale <= 10_000:
        return 6
    if scale >= 100_000:
        return 27
    ratio = (scale - 10_000) / 90_000
    interpolated = 6 + ratio * (27 - 6)
    return max(1, min(int(round(interpolated)), 27))


def pick_licensed_countries(countries: list[str], desired: int) -> list[str]:
    if desired <= 0:
        return []

    selected: list[str] = []
    for code in countries:
        if not code:
            continue
        normalized = code.strip().upper()
        if normalized in EU_MEMBER_STATE_SET and normalized not in selected:
            selected.append(normalized)

    if len(selected) < desired:
        for code in EU_MEMBER_STATES:
            if len(selected) >= desired:
                break
            if code not in selected:
                selected.append(code)

    return selected[:desired]


def schedule_cleanup(target: Path, ttl_seconds: int) -> None:
    if ttl_seconds <= 0:
        return

    def _cleanup() -> None:
        try:
            resolved = target.resolve()
            root = DATA_DIR.resolve()
            if root in resolved.parents and resolved.exists():
                shutil.rmtree(resolved)
        except Exception:
            return

    timer = Timer(ttl_seconds, _cleanup)
    timer.daemon = True
    timer.start()


def run_cesop(args: list[str], allow_fail: bool = False) -> subprocess.CompletedProcess:
    cmd = resolve_cesop_command()
    env = os.environ.copy()
    env.setdefault("CESOP_LOG_DIR", "none")
    env.setdefault("CESOP_LOG_LEVEL", "info")

    result = subprocess.run(
        cmd + args,
        cwd=REPO_ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0 and not allow_fail:
        message = result.stderr.strip() or result.stdout.strip() or "unknown error"
        raise RuntimeError(f"Command failed: {' '.join(cmd + args)}\n{message}")
    return result


PREFLIGHT_RE = re.compile(r"Preflight issues: errors=(\d+) warnings=(\d+)")
CORRECTED_RE = re.compile(r"Corrected records: (\d+) / (\d+)")


def parse_preflight_summary(output: str) -> dict | None:
    match = PREFLIGHT_RE.search(output)
    if not match:
        return None
    return {
        "errors": int(match.group(1)),
        "warnings": int(match.group(2)),
    }


def parse_correct_summary(output: str) -> dict | None:
    match = CORRECTED_RE.search(output)
    if not match:
        return None
    return {
        "corrected": int(match.group(1)),
        "total": int(match.group(2)),
    }


def resolve_vm_jar() -> Path | None:
    override = os.environ.get("CESOP_VM_JAR")
    if override:
        jar_path = Path(override)
        return jar_path if jar_path.exists() else None
    return CESOP_JAR if CESOP_JAR.exists() else None


def resolve_java_bin() -> str | None:
    override = os.environ.get("CESOP_JAVA_BIN") or os.environ.get("JAVA_BIN")
    if override:
        return validate_java_bin(override)

    java_home = os.environ.get("JAVA_HOME")
    if java_home:
        candidate = Path(java_home) / "bin" / "java"
        resolved = validate_java_bin(str(candidate))
        if resolved:
            return resolved

    java_home_exec = Path("/usr/libexec/java_home")
    if java_home_exec.exists():
        try:
            result = subprocess.run(
                [str(java_home_exec)],
                capture_output=True,
                text=True,
                check=False,
            )
            if result.returncode == 0:
                candidate = Path(result.stdout.strip()) / "bin" / "java"
                resolved = validate_java_bin(str(candidate))
                if resolved:
                    return resolved
        except Exception:
            pass

    java_cmd = shutil.which("java")
    if java_cmd:
        resolved = validate_java_bin(java_cmd)
        if resolved:
            return resolved
    return None


def validate_java_bin(java_bin: str) -> str | None:
    if not java_bin:
        return None
    if not (Path(java_bin).exists() or shutil.which(java_bin)):
        return None
    try:
        result = subprocess.run(
            [java_bin, "-version"],
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode == 0:
            return java_bin
    except Exception:
        return None
    return None


def read_csv_all(path: Path) -> tuple[list[str], list[list[str]]]:
    with path.open(newline="") as handle:
        reader = csv.reader(handle)
        header = next(reader, [])
        rows = list(reader)
    return header, rows


def read_text_snippet(path: Path, max_lines: int = 14) -> str:
    if not path.exists():
        return ""
    lines: list[str] = []
    with path.open(encoding="utf-8") as handle:
        for _, line in zip(range(max_lines), handle):
            lines.append(line.rstrip("\n"))
    return "\n".join(lines)


def extract_member_state(filename: str) -> str | None:
    parts = filename.split("_")
    if len(parts) < 4:
        return None
    code = parts[3]
    if len(code) == 2 and code.isalpha():
        return code.upper()
    return None


def safe_get(row: list[str], index: int | None) -> str:
    if index is None or index < 0 or index >= len(row):
        return ""
    return row[index]


def normalize_country_code(value: str) -> str | None:
    trimmed = value.strip()
    if len(trimmed) == 2 and trimmed.isalpha():
        return trimmed.upper()
    return None


def bic_country_code(value: str) -> str | None:
    trimmed = value.strip()
    if len(trimmed) not in (8, 11):
        return None
    if not trimmed.isalnum():
        return None
    code = trimmed[4:6]
    if code.isalpha():
        return code.upper()
    return None


def account_country_code(account_type: str, account_id: str) -> str | None:
    account_id = account_id.strip()
    if not account_id:
        return None
    kind = account_type.strip().upper()
    if kind in {"IBAN", "OBAN", "OTHER"}:
        return normalize_country_code(account_id[:2])
    if kind == "BIC":
        return bic_country_code(account_id)
    return None


def resolve_payee_country(
    row: list[str],
    payee_account_idx: int | None,
    payee_account_type_idx: int | None,
    payee_psp_id_idx: int | None,
) -> str:
    account_type = safe_get(row, payee_account_type_idx)
    account_id = safe_get(row, payee_account_idx)
    if account_id.strip():
        derived = account_country_code(account_type, account_id)
        return derived or ""
    payee_psp_id = safe_get(row, payee_psp_id_idx)
    derived = bic_country_code(payee_psp_id)
    if derived:
        return derived
    return ""


def reportable_for_psp(
    row: list[str],
    psp_role_idx: int | None,
    payee_psp_id_idx: int | None,
) -> bool:
    role = safe_get(row, psp_role_idx).strip().upper() or "PAYEE"
    if role != "PAYER":
        return True
    payee_psp_id = safe_get(row, payee_psp_id_idx)
    country = bic_country_code(payee_psp_id)
    if not country:
        return True
    return country not in EU_MEMBER_STATE_SET


def build_column_index(header: list[str]) -> dict[str, int]:
    return {name: idx for idx, name in enumerate(header)}


def compute_payee_count(rows: list[list[str]], payee_idx: int | None) -> int:
    if payee_idx is None:
        return 0
    return len({row[payee_idx] for row in rows if len(row) > payee_idx})


def split_cross_border_indices(
    rows: list[list[str]],
    payer_country_idx: int | None,
    payee_countries: list[str],
) -> tuple[list[int], list[int]]:
    cross_border = []
    non_cross_border = []
    if payer_country_idx is None:
        return cross_border, list(range(len(rows)))

    for idx, row in enumerate(rows):
        payer_raw = safe_get(row, payer_country_idx)
        payer = normalize_country_code(payer_raw) or ""
        payee = payee_countries[idx] if idx < len(payee_countries) else ""
        if payer in EU_MEMBER_STATE_SET and payee and payer != payee:
            cross_border.append(idx)
        else:
            non_cross_border.append(idx)
    return cross_border, non_cross_border


def compute_threshold_groups(
    rows: list[list[str]],
    cross_border_indices: list[int],
    payee_id_idx: int | None,
    payee_countries: list[str],
) -> tuple[dict[tuple[str, str], int], list[int]]:
    if payee_id_idx is None:
        return {}, []

    counts: dict[tuple[str, str], int] = {}
    for idx in cross_border_indices:
        row = rows[idx]
        payee_id = safe_get(row, payee_id_idx)
        payee_country = payee_countries[idx] if idx < len(payee_countries) else ""
        key = (payee_id, payee_country)
        counts[key] = counts.get(key, 0) + 1

    eligible_keys = {key for key, count in counts.items() if count > 25}
    eligible_indices = [
        idx
        for idx in cross_border_indices
        if (safe_get(rows[idx], payee_id_idx), payee_countries[idx])
        in eligible_keys
    ]
    return counts, eligible_indices


def diff_rows_for_indices(
    clean_rows: list[list[str]],
    corrupt_rows: list[list[str]],
    indices: list[int],
) -> tuple[int, int, tuple[int, list[int], list[str], list[str]] | None]:
    diff_cells = 0
    diff_row_count = 0
    first_diff = None

    for idx in indices:
        if idx >= len(clean_rows) or idx >= len(corrupt_rows):
            continue
        clean = clean_rows[idx]
        corrupt = corrupt_rows[idx]
        max_len = max(len(clean), len(corrupt))
        row_diff_cols = []
        for col in range(max_len):
            left = clean[col] if col < len(clean) else ""
            right = corrupt[col] if col < len(corrupt) else ""
            if left != right:
                row_diff_cols.append(col)
        if row_diff_cols:
            diff_row_count += 1
            diff_cells += len(row_diff_cols)
            if first_diff is None:
                first_diff = (idx, row_diff_cols, clean, corrupt)

    return diff_row_count, diff_cells, first_diff


def run_validation(output_dir: Path) -> dict:
    if os.environ.get("CESOP_SKIP_VALIDATION", "").lower() in {"1", "true", "yes"}:
        return {
            "status": "skipped",
            "passRate": "n/a",
            "duration": "0s",
            "snippet": "Validation skipped (CESOP_SKIP_VALIDATION=1).",
        }
    jar_path = resolve_vm_jar()
    if jar_path is None:
        return {
            "status": "skipped",
            "passRate": "n/a",
            "duration": "0s",
            "snippet": "Validation skipped (validation jar not found).",
        }
    java_bin = resolve_java_bin()
    if java_bin is None:
        return {
            "status": "skipped",
            "passRate": "n/a",
            "duration": "0s",
            "snippet": "Validation skipped (Java runtime not found). Install a JDK or set JAVA_HOME/CESOP_JAVA_BIN.",
        }

    validation_path = output_dir / "validation.xml"
    start = time.monotonic()
    result = run_cesop(
        [
            "validate",
            "--input",
            str(output_dir),
            "--output",
            str(validation_path),
            "--jar",
            str(jar_path),
            "--java",
            java_bin,
        ],
        allow_fail=True,
    )
    duration = time.monotonic() - start
    duration_label = f"{duration:.1f}s"

    if result.returncode != 0:
        message = result.stderr.strip() or result.stdout.strip() or "Validation failed."
        return {
            "status": "failed",
            "passRate": "n/a",
            "duration": duration_label,
            "snippet": message,
        }

    snippet = read_text_snippet(validation_path, max_lines=12)
    if not snippet:
        snippet = "Validation complete. Output written to validation.xml"

    return {
        "status": "pass",
        "passRate": "100%",
        "duration": duration_label,
        "snippet": snippet,
    }


def run_pipeline(scale: int) -> dict:
    ttl_seconds = resolve_cleanup_ttl_seconds()
    psps = 1
    run_id = datetime.utcnow().strftime("%Y%m%d_%H%M%S")
    run_dir = DATA_DIR / run_id
    run_dir.mkdir(parents=True, exist_ok=True)

    csv_path = run_dir / "payments.csv"
    corrupt_path = run_dir / "payments_invalid.csv"
    corrected_path = run_dir / "payments_corrected.csv"
    output_dir = run_dir / "output"
    output_dir.mkdir(parents=True, exist_ok=True)

    seed = random.randint(100000, 999999)

    run_cesop(
        [
            "generate",
            "--scale",
            str(scale),
            "--psps",
            str(psps),
            "--output",
            str(csv_path),
            "--seed",
            str(seed),
        ]
    )

    clean_header, clean_rows = read_csv_all(csv_path)
    header = clean_header

    column_index = build_column_index(header)
    payee_id_idx = column_index.get("payee_id")
    payer_country_idx = column_index.get("payer_country")
    payee_country_idx = column_index.get("payee_country")
    payee_account_idx = column_index.get("payee_account")
    payee_account_type_idx = column_index.get("payee_account_type")
    payee_psp_id_idx = column_index.get("payee_psp_id")
    psp_role_idx = column_index.get("psp_role")

    payee_countries = [
        resolve_payee_country(row, payee_account_idx, payee_account_type_idx, payee_psp_id_idx)
        for row in clean_rows
    ]

    payees = compute_payee_count(clean_rows, payee_id_idx)
    cross_border_indices, non_cross_border_indices = split_cross_border_indices(
        clean_rows, payer_country_idx, payee_countries
    )
    cross_border_count = len(cross_border_indices)
    non_cross_border_count = len(non_cross_border_indices)

    threshold_counts, reportable_indices = compute_threshold_groups(
        clean_rows, cross_border_indices, payee_id_idx, payee_countries
    )
    eligible_indices = sorted(reportable_indices)
    reportable_indices = [
        idx
        for idx in eligible_indices
        if reportable_for_psp(clean_rows[idx], psp_role_idx, payee_psp_id_idx)
    ]
    reportable_count = len(reportable_indices)
    below_threshold_count = max(0, cross_border_count - len(eligible_indices))

    eligible_keys = {key for key, count in threshold_counts.items() if count > 25}
    reportable_payees = len(eligible_keys)
    reportable_member_states = sorted(
        {
            payee_countries[idx]
            for idx in reportable_indices
            if payee_countries[idx]
        }
    )
    licensed_countries = pick_licensed_countries(
        reportable_member_states, resolve_license_count(scale)
    )

    run_cesop(
        [
            "corrupt",
            "--input",
            str(csv_path),
            "--output",
            str(corrupt_path),
            "--seed",
            str(seed),
        ]
    )
    corrupt_preflight_result = run_cesop(
        [
            "preflight",
            "--input",
            str(corrupt_path),
        ],
        allow_fail=True,
    )
    corrupt_preflight = parse_preflight_summary(
        f"{corrupt_preflight_result.stdout}\n{corrupt_preflight_result.stderr}"
    )
    correct_result = run_cesop(
        [
            "correct",
            "--input",
            str(corrupt_path),
            "--output",
            str(corrected_path),
            "--seed",
            str(seed),
        ]
    )
    correct_summary = parse_correct_summary(
        f"{correct_result.stdout}\n{correct_result.stderr}"
    )
    corrected_preflight_result = run_cesop(
        [
            "preflight",
            "--input",
            str(corrected_path),
        ],
        allow_fail=True,
    )
    corrected_preflight = parse_preflight_summary(
        f"{corrected_preflight_result.stdout}\n{corrected_preflight_result.stderr}"
    )
    render_args = ["render", "--input", str(corrected_path), "--output-dir", str(output_dir)]
    if licensed_countries:
        render_args += ["--licensed-countries", ",".join(licensed_countries)]
    run_cesop(render_args)

    corrupt_header, corrupt_rows = read_csv_all(corrupt_path)
    corrected_header, corrected_rows = read_csv_all(corrected_path)
    if not header:
        header = corrupt_header
    if not header:
        header = corrected_header

    diff_row_count, diff_cells, first_diff = diff_rows_for_indices(
        corrupt_rows, corrected_rows, reportable_indices
    )
    errors_count = (
        corrupt_preflight["errors"]
        if corrupt_preflight and "errors" in corrupt_preflight
        else diff_cells or diff_row_count
    )
    corrections_count = (
        correct_summary["corrected"]
        if correct_summary and "corrected" in correct_summary
        else diff_cells or diff_row_count
    )

    raw_rows = clean_rows[:5]

    cross_border_rows: list[list[str]] = []
    cross_border_highlights: list[dict] = []
    if cross_border_indices:
        cross_border_rows.append(clean_rows[cross_border_indices[0]])
    if non_cross_border_indices:
        cross_border_rows.append(clean_rows[non_cross_border_indices[0]])
        if payer_country_idx is not None and payee_country_idx is not None:
            cross_border_highlights.append(
                {
                    "row": len(cross_border_rows) - 1,
                    "cols": [payer_country_idx, payee_country_idx],
                }
            )

    threshold_summary_lines: list[str] = []
    if eligible_keys:
        ranked = sorted(
            ((key, count) for key, count in threshold_counts.items() if count > 25),
            key=lambda item: item[1],
            reverse=True,
        )
        for (payee_id, payee_country), count in ranked[:4]:
            label = payee_id or "Unknown payee"
            country = payee_country or "--"
            threshold_summary_lines.append(f"{label} ({country}) -> {count} payments")
    else:
        threshold_summary_lines.append("No payees over the >25 threshold in this sample.")

    error_rows: list[list[str]] = []
    error_highlights: list[dict] = []
    corrected_changes: list[dict] = []
    record_id = None
    if first_diff:
        diff_index, diff_cols, corrupted_row, corrected_row = first_diff
        error_rows.append(corrupted_row)
        for idx in reportable_indices:
            if idx != diff_index:
                error_rows.append(corrupt_rows[idx])
                break
        error_highlights.append({"row": 0, "cols": diff_cols})

        payment_idx = header.index("payment_id") if "payment_id" in header else None
        payee_idx = header.index("payee_id") if "payee_id" in header else None
        record_id = safe_get(corrected_row, payment_idx) or safe_get(corrected_row, payee_idx)

        for col in diff_cols[:4]:
            field = header[col] if col < len(header) else f"col_{col}"
            corrected_changes.append(
                {
                    "field": field,
                    "before": safe_get(corrupted_row, col),
                    "after": safe_get(corrected_row, col),
                }
            )

    xml_files = [path for path in sorted(output_dir.glob("*.xml")) if path.name != "validation.xml"]
    xml_snippet = read_text_snippet(xml_files[0], max_lines=14) if xml_files else ""
    xml_member_states = sorted(
        {
            code
            for code in (extract_member_state(path.name) for path in xml_files)
            if code
        }
    )
    member_state_codes = xml_member_states or licensed_countries or reportable_member_states

    validation = run_validation(output_dir)
    schedule_cleanup(run_dir, ttl_seconds)

    preflight_payload: dict = {}
    if corrupt_preflight:
        preflight_payload["corrupt"] = corrupt_preflight
    if corrected_preflight:
        preflight_payload["corrected"] = corrected_preflight

    return {
        "seed": seed,
        "rows": len(clean_rows),
        "sizeBytes": csv_path.stat().st_size if csv_path.exists() else 0,
        "payees": payees,
        "crossBorder": cross_border_count,
        "nonCrossBorder": non_cross_border_count,
        "reportable": reportable_count,
        "belowThreshold": below_threshold_count,
        "reportablePayees": reportable_payees,
        "memberStates": len(member_state_codes),
        "memberStateCodes": member_state_codes,
        "errors": errors_count,
        "corrections": corrections_count,
        "reports": len(xml_files),
        "xmlFiles": [path.name for path in xml_files],
        "validation": validation,
        "preflight": preflight_payload,
        "snippets": {
            "raw": {"header": header, "rows": raw_rows},
            "crossBorder": {
                "header": header,
                "rows": cross_border_rows,
                "highlights": cross_border_highlights,
            },
            "threshold": {"summary": "\n".join(threshold_summary_lines)},
            "error": {"header": header, "rows": error_rows, "highlights": error_highlights},
            "corrected": {"recordId": record_id, "changes": corrected_changes},
            "xml": xml_snippet,
        },
    }
