# CESOP Pipeline Studio Prototype

Static prototype for the scroll-driven CESOP pipeline demo page. This version
simulates the live generator for a 10k row sample and updates the data preview
and rule panels as the user scrolls.

## Run the prototype
From the repo root:
```sh
python -m venv .venv
source .venv/bin/activate
pip install -r app/requirements.txt
uvicorn app.main:app --reload
```
Then open `http://127.0.0.1:8000` and use "Generate 10k sample".

## Live pipeline wiring
- The FastAPI entrypoint is `app/main.py`.
- The generator endpoint calls the real CLI to produce:
  - raw CSV (10k rows)
  - corrupted CSV (for error detection)
  - corrected CSV (automatic fixes applied)
  - XML outputs
  - validation output (when Java + CESOP VM are available)
- Frontend fetches `POST /api/generate` and updates previews in-place.
- Generated artifacts are stored under `data/portfolio_demo/<timestamp>/`.

## App structure
- `app/main.py`: FastAPI app + mounts.
- `app/routers/`: page and API routes.
- `app/services/`: pipeline orchestration helpers.
- `app/templates/`: Jinja templates.
- `app/static/`: CSS and JS assets.

## Environment flags
- `CESOP_BIN`: path to a compiled `cesop-demo` binary (skips `cargo run`).
- `CESOP_VM_JAR`: path to the CESOP Validation Module jar.
- `CESOP_SKIP_VALIDATION=1`: skip the validation step if Java is unavailable.
- `CESOP_DEMO_TTL_SECONDS`: auto-delete demo run folders after N seconds (default 300).
- `CESOP_JAVA_BIN` or `JAVA_BIN`: path to a Java runtime for validation.
- `JAVA_HOME`: if set, `JAVA_HOME/bin/java` will be used.
- `CESOP_DEMO_LICENSED_COUNT`: number of licensed Member States to simulate
  (defaults to 6 at 10k, scales to 27 at 100k).

## Notes
- Designed for FastAPI + HTMX fragments later.
- The UI already supports scroll-driven step changes and a replay button.
- Demo runs use a single PSP and distribute reports across licensed Member
  States, prioritizing payees whose country matches a licensed Member State and
  falling back to the PSP home Member State.
