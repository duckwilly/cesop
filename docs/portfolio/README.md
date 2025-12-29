# CESOP Pipeline Portfolio Page Plan

## Goal
Create a scroll-driven, interactive "Pipeline Studio" page that shows the CESOP data pipeline from raw ingest to validated XML. The page should feel premium and be structured so it can later be implemented with FastAPI + HTMX.

## Experience Summary
- A single narrative page with a vertical pipeline timeline on the left.
- A sticky data preview panel on the right that updates per step.
- The story follows one PSP licensed in multiple Member States, narrowing from potential reportable markets to the actual reporting list.
- Each scroll section triggers a short animation (data tokens flowing) and a rule panel that explains a CESOP rule in plain language.
- Mobile layout stacks sections with the preview panel moving below the step content.

## Scroll-Driven Interaction
- Each step is a section with a stable anchor (hash in URL).
- When a section is revealed, update the active pipeline node, the preview panel, and the rule panel content.
- Use smooth transitions (fade + slide) to avoid jarring swaps.
- Optional "Replay pipeline" button that scrolls/animates through steps.

## Pipeline Steps and Artifacts
1) Raw Ingest
- Artifact: CSV/JSON snippet with raw fields.
- UI: "ingest" badge + source metadata chips + licensed MS count.
- Rule panel: reporting period, licensed MS footprint, required fields present.

2) Cross-border Filter
- Artifact: filtered snippet showing excluded domestic rows.
- UI: counts of cross-border vs excluded.
- Rule panel: payer/payee location rules.

3) Threshold Gate (>25)
- Artifact: payee summary list with counts.
- UI: reportable vs below-threshold tally + reporting MS count.
- Rule panel: per payee + Member State aggregation and reporting eligibility.

4) Error Detection
- Artifact: reportable subset with invalid fields highlighted.
- UI: issue list with severity + counts.
- Rule panel: required identifiers, country code format, amount precision.

5) Correction
- Artifact: before/after diff view.
- UI: "applied rules" list (normalization, format fixes).
- Rule panel: field normalization (dates, identifiers), code list alignment.

6) XML Generation
- Artifact: XML snippet (schema-compliant) plus file list.
- UI: reporting member states + XML file counts.
- Rule panel: schema version, namespaces, element ordering.

7) Validation
- Artifact: validation summary with sample errors and pass rate.
- UI: green/red status + counts.
- Rule panel: structural vs. business validation, cross-field checks.

## Rule Panels (Content Style)
- Short, human-readable explanations (2-3 sentences max).
- One concrete example per panel.
- Links to internal docs for the full rule text.
- Tone: confident, technical, but readable for non-experts.

## Layout and Visual Direction
- Left: pipeline timeline with animated nodes.
- Center: step narrative (title, copy, callouts).
- Right: sticky data preview (code panel).
- Background: subtle grid with faint data-flow lines.
- Typography: expressive headline + clean monospace for snippets.
- Palette: neutral base with one strong accent for active states.

## HTMX-Friendly Structure
- Page shell rendered by FastAPI.
- Each step content is a fragment (HTML partial):
  - /pipeline/step/raw
  - /pipeline/step/errors
  - /pipeline/step/corrected
  - /pipeline/step/xml
  - /pipeline/step/validation
- Use hx-trigger="revealed" or an IntersectionObserver to swap the preview panel and rule panel.
- Use hx-push-url to keep the active step in the URL.

## Data Sources (Later)
- Curate 1-2 realistic records with intentional issues.
- Use actual XML output as the final artifact (trimmed).
- Keep example data consistent across all steps.

## Prototype (Current)
- Prototype app lives under `app/` (templates, static, routers, services).
- Scroll-driven layout with preview and rule panels wired to the active step.
- FastAPI entrypoint at `app/main.py`.

## Next Work Items
1) Swap fetch-based updates for HTMX fragment swaps.
2) Expand the correction step with richer before/after artifacts.
3) Stream real preview snippets per step (raw, errors, corrected, xml, validation).
4) Add caching for generated 10k samples to speed up repeat demos.

## Open Questions
- Do we want scroll-triggered auto-advance or manual step buttons only?
- Should the rule panels be dismissible or always visible?
- How technical should the rule explanations be for a portfolio audience?
