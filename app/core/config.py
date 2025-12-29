from pathlib import Path

APP_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = APP_DIR.parent
STATIC_DIR = APP_DIR / "static"
TEMPLATE_DIR = APP_DIR / "templates"
DOCS_DIR = REPO_ROOT / "docs"
DATA_DIR = REPO_ROOT / "data" / "portfolio_demo"
CESOP_JAR = (
    REPO_ROOT
    / "scripts"
    / "CESOP Validation Module"
    / "SDEV-CESOP-VM-v1.7.1"
    / "cesop-vm-application-1.7.1.jar"
)
