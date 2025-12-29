const sampleState = {
  rows: 10000,
  size: "~2.6 MB",
  payees: 420,
  crossBorder: 8200,
  nonCrossBorder: 1800,
  reportable: 3420,
  belowThreshold: 4780,
  reportablePayees: 128,
  memberStates: 6,
  memberStateCodes: ["DK", "SE", "DE", "FR", "IE", "NL"],
  xmlFiles: [
    "cesop_2025_Q4_DK_MLIFDKV9.xml",
    "cesop_2025_Q4_SE_MLIFDKV9.xml",
    "cesop_2025_Q4_DE_MLIFDKV9.xml",
    "cesop_2025_Q4_FR_MLIFDKV9.xml",
    "cesop_2025_Q4_IE_MLIFDKV9.xml",
    "cesop_2025_Q4_NL_MLIFDKV9.xml",
  ],
  errors: 16,
  corrections: 12,
  preflightCorruptErrors: 16,
  preflightCorrectedErrors: 0,
  reports: 6,
  passRate: "98.9%",
  validationTime: "1.6s",
};

const steps = [
  {
    id: "raw",
    title: "Raw Ingest",
    meta: "{rows} rows loaded",
    preview: {
      type: "csv",
      header: [
        "psp_id",
        "payee_id",
        "payee_country",
        "payer_country",
        "amount",
        "currency",
        "tx_date",
        "account_type",
        "account_id",
      ],
      rows: [
        ["MLIFDKV9", "PAYEE-00412", "DK", "SE", "159.98", "EUR", "2025-10-04", "IBAN", "DK5000400440116243"],
        ["MLIFDKV9", "PAYEE-00412", "DK", "SE", "22.14", "EUR", "2025-10-19", "IBAN", "DK5000400440116243"],
        ["MLIFDKV9", "PAYEE-01077", "DK", "NO", "312.00", "EUR", "2025-11-02", "IBAN", "DK9251000190000210"],
        ["MLIFDKV9", "PAYEE-01902", "DK", "SE", "145.50", "EUR", "2025-11-14", "IBAN", "DK2940011001092214"],
        ["MLIFDKV9", "PAYEE-02311", "DK", "DE", "49.95", "EUR", "2025-12-01", "IBAN", "DK7300400440116243"],
        ["MLIFDKV9", "PAYEE-03118", "DK", "FR", "92.40", "EUR", "2025-12-02", "IBAN", "DK2900400440114120"],
        ["MLIFDKV9", "PAYEE-04009", "DK", "IE", "18.75", "EUR", "2025-12-03", "IBAN", "DK9900400440118431"],
      ],
    },
    rule: {
      title: "Data requirements",
      body:
        "CESOP requires payment transactions to include PSP identifiers, payee details, payer location, amount, and timing. This raw data forms the foundation for all downstream processing.",
      list: [
        "PSP identifier (BIC) required",
        "Payee account and location captured",
        "Payer Member State for cross-border determination",
        "Transaction amount and currency",
        "Date within reporting quarter",
      ],
    },
    metrics: [
      { label: "Records", value: "{rows}" },
      { label: "Payees", value: "{payees}" },
      { label: "Cross-border", value: "{crossBorder}" },
    ],
  },
  {
    id: "cross-border",
    title: "Cross-border Filter",
    meta: "{nonCrossBorder} excluded | {crossBorder} remaining",
    preview: {
      type: "csv",
      header: [
        "psp_id",
        "payee_id",
        "payee_country",
        "payer_country",
        "amount",
        "currency",
        "tx_date",
        "account_type",
        "account_id",
      ],
      rows: [
        ["MLIFDKV9", "PAYEE-00412", "DK", "SE", "159.98", "EUR", "2025-10-04", "IBAN", "DK5000400440116243"],
        ["MLIFDKV9", "PAYEE-00918", "DK", "DK", "64.12", "EUR", "2025-11-05", "IBAN", "DK8011000190100101"],
        ["MLIFDKV9", "PAYEE-01077", "DK", "NO", "312.00", "EUR", "2025-11-02", "IBAN", "DK9251000190000210"],
        ["MLIFDKV9", "PAYEE-01902", "DK", "SE", "145.50", "EUR", "2025-11-14", "IBAN", "DK2940011001092214"],
        ["MLIFDKV9", "PAYEE-02311", "DK", "DE", "49.95", "EUR", "2025-12-01", "IBAN", "DK7300400440116243"],
        ["MLIFDKV9", "PAYEE-03118", "DK", "FR", "92.40", "EUR", "2025-12-02", "IBAN", "DK2900400440114120"],
        ["MLIFDKV9", "PAYEE-04009", "DK", "IE", "18.75", "EUR", "2025-12-03", "IBAN", "DK9900400440118431"],
      ],
      highlights: [{ row: 1, cols: [2, 3] }],
    },
    rule: {
      title: "Scope determination",
      body:
        "CESOP reporting applies only to cross-border payments within the EU. Transactions where payer and payee share the same Member State are out of scope.",
      list: [
        "Compare payer MS vs payee MS",
        "Same country = domestic = excluded",
        "Different country = cross-border = in scope",
      ],
    },
    metrics: [
      { label: "Excluded", value: "{nonCrossBorder}" },
      { label: "Remaining", value: "{crossBorder}" },
      { label: "Rule", value: "Payer ≠ Payee" },
    ],
  },
  {
    id: "threshold",
    title: "Threshold Gate",
    meta: "{reportablePayees} payees over | {reportable} reportable rows",
    preview: {
      type: "text",
      value:
        "PAYEE-00412 (SE) -> 34 payments\nPAYEE-01902 (DE) -> 41 payments\nPAYEE-02311 (IE) -> 29 payments\nPAYEE-03118 (FR) -> 27 payments\nPAYEE-04009 (IE) -> 31 payments\nPAYEE-05031 (SE) -> 26 payments\nPAYEE-05302 (NL) -> 33 payments\nPAYEE-06110 (FR) -> 28 payments",
    },
    rule: {
      title: "The >25 rule",
      body:
        "Reporting obligation triggers when a payee receives more than 25 cross-border payments in a calendar quarter. Count is per payee per Member State.",
      list: [
        "Aggregate payments by payee",
        "Group by payee's Member State",
        "Threshold: >25 payments per quarter",
        "Below threshold = no reporting required",
      ],
    },
    metrics: [
      { label: "Reportable", value: "{reportable}" },
      { label: "Below threshold", value: "{belowThreshold}" },
      { label: "Payees over", value: "{reportablePayees}" },
    ],
  },
  {
    id: "errors",
    title: "Error Detection",
    meta: "Reportable scan | {errors} issues flagged",
    preview: {
      type: "csv",
      header: [
        "psp_id",
        "payee_id",
        "payee_country",
        "payer_country",
        "amount",
        "currency",
        "tx_date",
        "account_type",
        "account_id",
      ],
      rows: [
        ["MLIFDKV9", "PAYEE-01902", "DK", "ZZ", "145.50", "EUR", "2025-11-14", "IBAN", "DK2940011001092214"],
        ["MLIFDKV9", "PAYEE-02134", "DK", "FR", "88.10", "EURO", "2025-11-22", "IBAN", "DK1200009882311231"],
        ["MLIFDKV9", "PAYEE-02520", "DK", "SE", "55.00", "EUR", "2025-11-27", "IBAN", "DK4500003111988812"],
        ["MLIFDKV9", "PAYEE-02702", "DK", "DE", "410.20", "EURO", "2025-12-02", "IBAN", "DK0900003111988888"],
        ["MLIFDKV9", "PAYEE-02851", "DK", "XX", "12.40", "EUR", "2025-12-04", "IBAN", "DK1100003111987777"],
        ["MLIFDKV9", "PAYEE-03118", "DK", "FR", "92.40", "EUR", "2025-12-02", "IBAN", "DK2900400440114120"],
        ["MLIFDKV9", "PAYEE-04009", "DK", "IE", "18.75", "EUR", "2025-12-03", "IBAN", "DK9900400440118431"],
      ],
      highlights: [
        { row: 0, cols: [3] },
        { row: 1, cols: [5] },
        { row: 4, cols: [3] },
      ],
    },
    rule: {
      title: "Data quality checks",
      body:
        "CESOP schema enforces strict code lists. Invalid values will fail official validation. Detecting issues early allows for correction before XML generation.",
      list: [
        "Country codes: ISO 3166-1 alpha-2",
        "Currency codes: ISO 4217",
        "Date formats: YYYY-MM-DD",
        "Account identifiers: IBAN or other standard",
      ],
    },
    metrics: [
      { label: "Issues", value: "{errors}" },
      { label: "Scope", value: "Reportable" },
      { label: "Coverage", value: "100%" },
    ],
  },
  {
    id: "corrected",
    title: "Correction",
    meta: "{corrections} fixes applied | audit log saved",
    preview: {
      type: "diff",
      recordId: "PAYEE-01902",
      changes: [
        { field: "payer_country", before: "ZZ", after: "SE" },
        { field: "currency", before: "EURO", after: "EUR" },
        { field: "tx_date", before: "2025-13-15", after: "2025-12-15" },
      ],
    },
    rule: {
      title: "Audit trail",
      body:
        "Every correction is logged with before and after values. Deterministic rules ensure consistent, reproducible fixes. Full transparency for compliance review.",
      list: [
        "Each change recorded as a diff",
        "Original values preserved",
        "Correction rules are deterministic",
        "Complete audit log exportable",
      ],
    },
    metrics: [
      { label: "Fixes", value: "{corrections}" },
      { label: "Preflight errors", value: "{preflightCorrectedErrors}" },
      { label: "Audit", value: "Enabled" },
    ],
  },
  {
    id: "xml",
    title: "XML Generation",
    meta: "{reports} file(s) | {memberStates} reporting MS",
    preview: {
      type: "xml",
      fileList: [
        "cesop_2025_Q4_DK_MLIFDKV9.xml",
        "cesop_2025_Q4_SE_MLIFDKV9.xml",
        "cesop_2025_Q4_DE_MLIFDKV9.xml",
        "cesop_2025_Q4_FR_MLIFDKV9.xml",
        "cesop_2025_Q4_IE_MLIFDKV9.xml",
        "cesop_2025_Q4_NL_MLIFDKV9.xml",
      ],
      value: `<?xml version="1.0" encoding="UTF-8"?>\n<CESOP xmlns="urn:ec.europa.eu:taxud:fiscalis:cesop:v1" xmlns:cm="urn:eu:taxud:commontypes:v1" xmlns:iso="urn:eu:taxud:isotypes:v1" version="4.03">\n  <MessageSpec>\n    <TransmittingCountry>DK</TransmittingCountry>\n    <MessageType>PMT</MessageType>\n    <MessageTypeIndic>CESOP100</MessageTypeIndic>\n    <MessageRefId>67f2a1c9-3ab9-4f2c-9f1b-1d4a8bcd27e6</MessageRefId>\n    <ReportingPeriod>\n      <Quarter>4</Quarter>\n      <Year>2025</Year>\n    </ReportingPeriod>\n    <Timestamp>2025-12-31T23:59:59Z</Timestamp>\n  </MessageSpec>\n  <PaymentDataBody>\n    <ReportingPSP>\n      <PSPId PSPIdType="BIC">MLIFDKV9</PSPId>\n      <Name nameType="BUSINESS">Northshore Payments</Name>\n    </ReportingPSP>\n    <ReportedPayee>\n      <Name nameType="BUSINESS">Silver Trading BV</Name>\n      <Country>DK</Country>\n      <Address>\n        <cm:CountryCode>DK</cm:CountryCode>\n        <cm:AddressFree>Market St 12, 2100 Copenhagen</cm:AddressFree>\n      </Address>\n      <TAXIdentification/>\n      <AccountIdentifier CountryCode="DK" type="IBAN">DK5000400440116243</AccountIdentifier>\n      <ReportedTransaction>\n        <TransactionIdentifier>PAY-00412-01</TransactionIdentifier>\n        <DateTime transactionDateType="CESOP701">2025-10-04T10:04:22Z</DateTime>\n        <Amount currency="EUR">159.98</Amount>\n        <PaymentMethod>\n          <cm:PaymentMethodType>Card payment</cm:PaymentMethodType>\n        </PaymentMethod>\n        <InitiatedAtPhysicalPremisesOfMerchant>true</InitiatedAtPhysicalPremisesOfMerchant>\n        <PayerMS PayerMSSource="IBAN">SE</PayerMS>\n      </ReportedTransaction>\n      <DocSpec>\n        <cm:DocTypeIndic>CESOP1</cm:DocTypeIndic>\n        <cm:DocRefId>9e0d9d12-5e76-4f1a-92d7-3f2e5a4c8d3f</cm:DocRefId>\n      </DocSpec>\n    </ReportedPayee>\n  </PaymentDataBody>\n</CESOP>`,
    },
    rule: {
      title: "Schema compliance",
      body:
        "Generated XML follows the official CESOP PaymentData schema. Correct element ordering, namespace declarations, and data types are enforced.",
      list: [
        "CESOP PaymentData v4.03 schema",
        "Proper XML namespace declarations",
        "Elements in required order",
        "One file per reporting Member State",
      ],
    },
    metrics: [
      { label: "Files", value: "{reports}" },
      { label: "Reporting MS", value: "{memberStates}" },
      { label: "Output", value: "XML" },
    ],
  },
  {
    id: "validation",
    title: "Validation",
    meta: "Pass rate {passRate} | {validationTime}",
    preview: {
      type: "text",
      value:
        "Validation: PASS\nSchema checks: 134\nBusiness rules: 27\nWarnings: 2 (precision normalized)\nValidated files: 6\nErrors: 0\nOutput: validation.xml\nDuration: 1.6s",
    },
    rule: {
      title: "Official validation",
      body:
        "The EU-provided CESOP Validation Module performs comprehensive checks. Passing validation confirms files are ready for submission to tax authorities.",
      list: [
        "XML schema validation",
        "Business rule checks",
        "Cross-field consistency",
        "Validation report generated",
      ],
    },
    metrics: [
      { label: "Pass rate", value: "{passRate}" },
      { label: "Warnings", value: "2" },
      { label: "Duration", value: "{validationTime}" },
    ],
  },
  {
    id: "outro",
    title: "Complete",
    meta: "Pipeline finished | Files ready",
    preview: {
      type: "text",
      value:
        "Pipeline complete\nDeliverables:\n  - 6 validated XML files\n  - Correction audit log\n  - Validation reports\nReady for CESOP portal submission",
    },
    rule: {
      title: "Production ready",
      body:
        "This pipeline transforms raw PSP data into validated CESOP reports. The same process scales to production volumes with real transaction data.",
      list: [
        "Connect to live PSP exports",
        "Schedule quarterly runs",
        "Archive audit trails",
        "Submit via CESOP portal",
      ],
    },
    metrics: [
      { label: "Status", value: "Complete" },
      { label: "Artifacts", value: "{reports} XMLs" },
      { label: "Validation", value: "Pass" },
    ],
  },
];

const numberFormat = new Intl.NumberFormat("en-US");
const csvPalette = [
  "csv-col-0",
  "csv-col-1",
  "csv-col-2",
  "csv-col-3",
  "csv-col-4",
  "csv-col-5",
  "csv-col-6",
  "csv-col-7",
];
const MAX_PREVIEW_LINES = 8;
const flowStageOrder = ["raw", "cross-border", "threshold", "xml"];
const flowStageMap = {
  raw: "raw",
  "cross-border": "cross-border",
  threshold: "threshold",
  errors: "threshold",
  corrected: "threshold",
  xml: "xml",
  validation: "xml",
  outro: "xml",
};

let activeStepId = steps[0].id;
let scrollTicking = false;

const stepElements = Array.from(document.querySelectorAll(".step"));
const timelineItems = Array.from(document.querySelectorAll(".timeline-item"));
const previewTitle = document.getElementById("previewTitle");
const previewMeta = document.getElementById("previewMeta");
const previewCode = document.getElementById("previewCode");
const previewMetrics = document.getElementById("previewMetrics");
const ruleTitle = document.getElementById("ruleTitle");
const ruleBody = document.getElementById("ruleBody");
const ruleList = document.getElementById("ruleList");
const previewPanel = document.querySelector(".preview-panel");

const generateBtn = document.getElementById("generateSample");
const sampleStatus = document.getElementById("sampleStatus");
const sampleRows = document.getElementById("sampleRows");
const sampleSize = document.getElementById("sampleSize");
const generatorStatus = document.getElementById("generatorStatus");
const generatorProgress = document.getElementById("generatorProgress");
const flowTotal = document.getElementById("flowTotal");
const flowCrossBorder = document.getElementById("flowCrossBorder");
const flowExcluded = document.getElementById("flowExcluded");
const flowReportable = document.getElementById("flowReportable");
const flowBelowThreshold = document.getElementById("flowBelowThreshold");
const flowXmlFiles = document.getElementById("flowXmlFiles");
const flowMemberStates = document.getElementById("flowMemberStates");
const flowSteps = Array.from(document.querySelectorAll(".flow-step"));
const flowTotalBar = document.getElementById("flowTotalBar");
const flowCrossBorderBar = document.getElementById("flowCrossBorderBar");
const flowReportableBar = document.getElementById("flowReportableBar");
const stepsContainer = document.querySelector(".steps");
const detailGrid = document.querySelector(".detail-grid");

function initStepOrder() {
  stepElements.forEach((el, index) => {
    el.style.setProperty("--step-order", index + 1);
  });
}

function getPreviewOffset() {
  const raw = getComputedStyle(document.documentElement)
    .getPropertyValue("--preview-offset")
    .trim();
  const value = Number.parseFloat(raw);
  if (Number.isNaN(value)) {
    return 320;
  }
  return value;
}

function updateStackPadding() {
  if (!stepsContainer || stepElements.length === 0) {
    return;
  }
  const stepHeight = stepElements[0].getBoundingClientRect().height;
  if (!stepHeight) {
    return;
  }
  const page = document.querySelector(".page");
  const pagePadding = page
    ? Number.parseFloat(getComputedStyle(page).paddingBottom) || 0
    : 0;
  const topOffset = getPreviewOffset() + 20;
  const padding = Math.max(
    0,
    Math.floor(window.innerHeight - topOffset - stepHeight - pagePadding)
  );
  const paddingValue = `${padding}px`;
  stepsContainer.style.setProperty("--stack-padding", paddingValue);
}

function updateActiveStepFromScroll() {
  if (stepElements.length === 0) {
    return;
  }
  const topOffset = getPreviewOffset() + 20;
  let candidate = stepElements[0];
  let candidateIndex = 0;
  stepElements.forEach((el, index) => {
    const rect = el.getBoundingClientRect();
    if (rect.top - topOffset <= 1) {
      candidate = el;
      candidateIndex = index;
    }
  });
  const candidateId = candidate.dataset.step;
  if (candidateId && candidateId !== activeStepId) {
    setActiveStep(candidateId);
  }
  const stepHeight = stepElements[0].getBoundingClientRect().height || 0;
  const threshold = stepHeight * 0.35;
  stepElements.forEach((el, index) => {
    let isForeground = index === candidateIndex;
    if (index === candidateIndex + 1) {
      const rect = el.getBoundingClientRect();
      const distance = rect.top - topOffset;
      if (distance <= threshold) {
        isForeground = true;
      }
    }
    el.classList.toggle("is-foreground", isForeground);
  });
}

function handleScroll() {
  if (scrollTicking) {
    return;
  }
  scrollTicking = true;
  requestAnimationFrame(() => {
    scrollTicking = false;
    updateActiveStepFromScroll();
  });
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

function clampPreviewLines(lines) {
  if (!Array.isArray(lines)) {
    return [];
  }
  if (lines.length <= MAX_PREVIEW_LINES) {
    return lines;
  }
  return [...lines.slice(0, MAX_PREVIEW_LINES - 1), "..."];
}

function clampPreviewText(value) {
  const lines = String(value ?? "").split("\n");
  return clampPreviewLines(lines).join("\n");
}

function formatValue(value) {
  if (typeof value === "number") {
    return numberFormat.format(value);
  }
  return String(value);
}

function formatBytes(bytes) {
  if (bytes === undefined || bytes === null || Number.isNaN(bytes)) {
    return sampleState.size;
  }
  const units = ["B", "KB", "MB", "GB"];
  let value = Number(bytes);
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  const rounded = value >= 10 || unitIndex === 0 ? Math.round(value) : Math.round(value * 10) / 10;
  return `~${rounded} ${units[unitIndex]}`;
}

function fillTemplate(template) {
  return template.replace(/\{(\w+)\}/g, (_, key) => {
    const value = sampleState[key];
    if (value === undefined) {
      return "";
    }
    return formatValue(value);
  });
}

function renderCsvSnippet(preview) {
  if (!preview || !preview.rows) {
    return "";
  }
  const header = Array.isArray(preview.header) ? preview.header : [];
  const rows = Array.isArray(preview.rows) ? preview.rows : [];
  const highlights = Array.isArray(preview.highlights) ? preview.highlights : [];

  const markMap = new Set();
  const headerOffset = header.length > 0 ? 1 : 0;
  const maxRows = Math.max(0, MAX_PREVIEW_LINES - headerOffset);
  const visibleRows = rows.slice(0, maxRows);
  highlights.forEach((highlight) => {
    if (!highlight || !Array.isArray(highlight.cols)) {
      return;
    }
    const rowIndex = (highlight.row || 0) + headerOffset;
    highlight.cols.forEach((col) => {
      markMap.add(`${rowIndex}:${col}`);
    });
  });

  const allRows = header.length > 0 ? [header, ...visibleRows] : visibleRows;
  const lines = allRows.map((row, rowIndex) => {
    return row
      .map((value, colIndex) => {
        const classes = ["csv-cell", csvPalette[colIndex % csvPalette.length]];
        if (rowIndex === 0 && header.length > 0) {
          classes.push("csv-header");
        }
        if (markMap.has(`${rowIndex}:${colIndex}`)) {
          classes.push("csv-mark");
        }
        const text = escapeHtml(value ?? "");
        const suffix = colIndex < row.length - 1 ? "," : "";
        return `<span class="${classes.join(" ")}">${text}${suffix}</span>`;
      })
      .join("");
  });

  return clampPreviewLines(lines).join("\n");
}

function renderDiffSnippet(preview) {
  if (!preview || !Array.isArray(preview.changes) || preview.changes.length === 0) {
    return "No corrections applied.";
  }

  const lines = [];
  if (preview.recordId) {
    lines.push(`<span class="diff-header">${escapeHtml(preview.recordId)}</span>`);
  }

  let shown = 0;
  for (const change of preview.changes) {
    if (lines.length + 2 > MAX_PREVIEW_LINES) {
      break;
    }
    lines.push(
      `<span class="remove">- ${escapeHtml(change.field)}: ${escapeHtml(change.before)}</span>`
    );
    lines.push(
      `<span class="add">+ ${escapeHtml(change.field)}: ${escapeHtml(change.after)}</span>`
    );
    shown += 1;
  }

  if (lines.length < MAX_PREVIEW_LINES) {
    const summary = `Showing ${shown} of ${preview.changes.length} fixes`;
    if (lines.length + 1 <= MAX_PREVIEW_LINES) {
      lines.push(`<span class="diff-header">${escapeHtml(summary)}</span>`);
    }
  }

  return lines.slice(0, MAX_PREVIEW_LINES).join("\n");
}

function renderXmlSnippet(preview) {
  const lines = [];
  const files = Array.isArray(preview.fileList) ? preview.fileList : sampleState.xmlFiles;
  const memberStates = Array.isArray(sampleState.memberStateCodes)
    ? sampleState.memberStateCodes
    : [];

  if (memberStates.length > 0) {
    lines.push(`Reporting MS: ${memberStates.join(", ")}`);
  }

  lines.push(`Generated XML files (${files.length}):`);

  const rawXmlLines = String(preview.value ?? "")
    .split("\n")
    .map((line) => line.trimEnd())
    .filter((line) => line.length > 0);
  const availableForExcerpt = Math.max(0, MAX_PREVIEW_LINES - lines.length - 1);
  const excerptLines = rawXmlLines.slice(0, Math.min(availableForExcerpt, 2));
  const reservedForExcerpt = excerptLines.length > 0 ? 1 + excerptLines.length : 0;
  const availableForFiles = Math.max(0, MAX_PREVIEW_LINES - lines.length - reservedForExcerpt);
  const fileLines = [];

  if (files && files.length > 0 && availableForFiles > 0) {
    let maxFiles = Math.min(files.length, availableForFiles);
    if (files.length > maxFiles && availableForFiles > 1) {
      maxFiles = availableForFiles - 1;
    }
    for (let i = 0; i < maxFiles; i += 1) {
      fileLines.push(`- ${files[i]}`);
    }
    if (files.length > maxFiles && fileLines.length < availableForFiles) {
      fileLines.push(`- ...and ${files.length - maxFiles} more`);
    }
  }

  lines.push(...fileLines);

  if (excerptLines.length > 0 && lines.length + 1 + excerptLines.length <= MAX_PREVIEW_LINES) {
    lines.push("XML excerpt:");
    lines.push(...excerptLines);
  }

  return lines.slice(0, MAX_PREVIEW_LINES).join("\n");
}

function setPreview(step) {
  previewTitle.textContent = step.title;
  previewMeta.textContent = fillTemplate(step.meta);

  previewCode.classList.toggle("csv-snippet", step.preview.type === "csv");
  previewCode.classList.toggle("diff-snippet", step.preview.type === "diff");

  if (step.preview.type === "csv") {
    previewCode.innerHTML = renderCsvSnippet(step.preview);
  } else if (step.preview.type === "xml") {
    previewCode.textContent = renderXmlSnippet(step.preview);
  } else if (step.preview.type === "diff") {
    previewCode.innerHTML = renderDiffSnippet(step.preview);
  } else if (step.preview.type === "html") {
    previewCode.innerHTML = step.preview.value;
  } else {
    previewCode.textContent = clampPreviewText(step.preview.value);
  }

  previewMetrics.innerHTML = "";
  step.metrics.forEach((metric) => {
    const metricEl = document.createElement("div");
    metricEl.className = "metric";

    const label = document.createElement("div");
    label.className = "label";
    label.textContent = metric.label;

    const value = document.createElement("div");
    value.className = "value";
    value.textContent = fillTemplate(metric.value);

    metricEl.appendChild(label);
    metricEl.appendChild(value);
    previewMetrics.appendChild(metricEl);
  });

  ruleTitle.textContent = step.rule.title;
  ruleBody.textContent = step.rule.body;
  ruleList.innerHTML = "";
  step.rule.list.forEach((item) => {
    const li = document.createElement("li");
    li.textContent = item;
    ruleList.appendChild(li);
  });

  requestAnimationFrame(updatePreviewOffset);
}

function setActiveStep(id) {
  const step = steps.find((item) => item.id === id);
  if (!step) {
    return;
  }
  activeStepId = id;
  setPreview(step);
  setFlowStage(id);

  const timelineId = id === "outro" ? "validation" : id;

  stepElements.forEach((el) => {
    el.classList.toggle("is-active", el.dataset.step === id);
  });
  timelineItems.forEach((el) => {
    el.classList.toggle("is-active", el.dataset.step === timelineId);
  });
  updateFlowDisplay();
}

function updatePreviewOffset() {
  if (!previewPanel) {
    return;
  }
  const height = Math.ceil(previewPanel.getBoundingClientRect().height);
  if (height > 0) {
    document.documentElement.style.setProperty(
      "--preview-offset",
      `${height + 24}px`
    );
  }
  updateStackPadding();
  updateActiveStepFromScroll();
}

function setFlowStage(stepId) {
  const stageKey = flowStageMap[stepId] || flowStageOrder[0];
  const activeIndex = flowStageOrder.indexOf(stageKey);
  flowSteps.forEach((el) => {
    const stepKey = el.dataset.flow;
    const stepIndex = flowStageOrder.indexOf(stepKey);
    const isActive = stepIndex !== -1 && stepIndex <= activeIndex;
    el.classList.toggle("is-active", isActive);
  });
}

function initScrollTracking() {
  window.addEventListener("scroll", handleScroll, { passive: true });
  window.addEventListener("resize", handleScroll);
  handleScroll();
}

function initTimelineNav() {
  timelineItems.forEach((item) => {
    const button = item.querySelector("button");
    if (!button) {
      return;
    }
    button.addEventListener("click", () => {
      const stepId = item.dataset.step;
      const target = document.getElementById(`step-${stepId}`);
      if (target) {
        target.scrollIntoView({ behavior: "smooth", block: "center" });
      }
    });
  });
}

function initPreviewOffset() {
  updatePreviewOffset();
  window.addEventListener("resize", () => {
    updatePreviewOffset();
  });
}

function updateSampleDisplay() {
  sampleRows.textContent = formatValue(sampleState.rows);
  sampleSize.textContent = sampleState.size;
  updateFlowDisplay();
  updatePreviewOffset();
}

function updateFlowDisplay() {
  const stageKey = flowStageMap[activeStepId] || flowStageOrder[0];
  const activeIndex = flowStageOrder.indexOf(stageKey);
  const canReveal = (stage) => activeIndex >= flowStageOrder.indexOf(stage);

  if (flowTotal) {
    flowTotal.textContent = formatValue(sampleState.rows);
  }
  if (flowTotalBar) {
    flowTotalBar.style.width = "100%";
  }
  if (flowCrossBorder) {
    flowCrossBorder.textContent = canReveal("cross-border")
      ? formatValue(sampleState.crossBorder)
      : "--";
  }
  if (flowCrossBorderBar) {
    const pct = canReveal("cross-border")
      ? Math.round((sampleState.crossBorder / sampleState.rows) * 100)
      : 0;
    flowCrossBorderBar.style.width = `${pct}%`;
  }
  if (flowExcluded) {
    flowExcluded.textContent = canReveal("cross-border")
      ? `-${formatValue(sampleState.nonCrossBorder)} excluded`
      : "";
  }
  if (flowReportable) {
    flowReportable.textContent = canReveal("threshold")
      ? formatValue(sampleState.reportable)
      : "--";
  }
  if (flowReportableBar) {
    const pct = canReveal("threshold")
      ? Math.round((sampleState.reportable / sampleState.rows) * 100)
      : 0;
    flowReportableBar.style.width = `${pct}%`;
  }
  if (flowBelowThreshold) {
    flowBelowThreshold.textContent = canReveal("threshold")
      ? `-${formatValue(sampleState.belowThreshold)} below threshold`
      : "";
  }
  if (flowXmlFiles) {
    const fileCount = Array.isArray(sampleState.xmlFiles)
      ? sampleState.xmlFiles.length
      : sampleState.reports;
    flowXmlFiles.textContent = canReveal("xml")
      ? `${formatValue(fileCount)} file${fileCount === 1 ? "" : "s"}`
      : "--";
  }
  if (flowMemberStates) {
    flowMemberStates.textContent = canReveal("xml")
      ? `${formatValue(sampleState.memberStates)} reporting MS`
      : "";
  }
}

function updateStepPreview(stepId, preview) {
  const step = steps.find((item) => item.id === stepId);
  if (!step) {
    return;
  }
  step.preview = { ...step.preview, ...preview };
  requestAnimationFrame(updatePreviewOffset);
}

function applyPipelineData(data) {
  if (!data || typeof data !== "object") {
    return;
  }

  if (data.rows !== undefined) {
    sampleState.rows = data.rows;
  }
  if (data.sizeBytes !== undefined) {
    sampleState.size = formatBytes(data.sizeBytes);
  }
  if (data.payees !== undefined) {
    sampleState.payees = data.payees;
  }
  if (data.crossBorder !== undefined) {
    sampleState.crossBorder = data.crossBorder;
  }
  if (data.nonCrossBorder !== undefined) {
    sampleState.nonCrossBorder = data.nonCrossBorder;
  }
  if (data.reportable !== undefined) {
    sampleState.reportable = data.reportable;
  }
  if (data.belowThreshold !== undefined) {
    sampleState.belowThreshold = data.belowThreshold;
  }
  if (data.reportablePayees !== undefined) {
    sampleState.reportablePayees = data.reportablePayees;
  }
  if (data.memberStates !== undefined) {
    sampleState.memberStates = data.memberStates;
  }
  if (Array.isArray(data.memberStateCodes)) {
    sampleState.memberStateCodes = data.memberStateCodes;
  }
  if (data.errors !== undefined) {
    sampleState.errors = data.errors;
  }
  if (data.corrections !== undefined) {
    sampleState.corrections = data.corrections;
  }
  if (data.preflight) {
    const corrupt = data.preflight.corrupt;
    if (corrupt && corrupt.errors !== undefined) {
      sampleState.preflightCorruptErrors = corrupt.errors;
    }
    const corrected = data.preflight.corrected;
    if (corrected && corrected.errors !== undefined) {
      sampleState.preflightCorrectedErrors = corrected.errors;
    }
  }
  if (data.reports !== undefined) {
    sampleState.reports = data.reports;
  }
  if (Array.isArray(data.xmlFiles)) {
    sampleState.xmlFiles = data.xmlFiles;
  }

  if (data.validation) {
    if (data.validation.passRate) {
      sampleState.passRate = data.validation.passRate;
    }
    if (data.validation.duration) {
      sampleState.validationTime = data.validation.duration;
    }
    if (data.validation.snippet) {
      updateStepPreview("validation", { type: "text", value: data.validation.snippet });
    }
  }

  if (data.snippets) {
    if (data.snippets.raw) {
      updateStepPreview("raw", { type: "csv", ...data.snippets.raw });
    }
    if (data.snippets.crossBorder) {
      updateStepPreview("cross-border", { type: "csv", ...data.snippets.crossBorder });
    }
    if (data.snippets.threshold) {
      updateStepPreview("threshold", {
        type: "text",
        value: data.snippets.threshold.summary,
      });
    }
    if (data.snippets.error) {
      updateStepPreview("errors", { type: "csv", ...data.snippets.error });
    }
    if (data.snippets.corrected) {
      updateStepPreview("corrected", { type: "diff", ...data.snippets.corrected });
    }
    if (data.snippets.xml) {
      updateStepPreview("xml", {
        type: "xml",
        value: data.snippets.xml,
        fileList: sampleState.xmlFiles,
      });
    }
  }

  updateSampleDisplay();
  setActiveStep(activeStepId);
}

function beginGeneration() {
  if (generateBtn.disabled) {
    return false;
  }
  generateBtn.disabled = true;
  generateBtn.classList.add("is-loading");
  generatorStatus.textContent = "Generating 10,000 rows...";
  sampleStatus.textContent = "Generating...";
  generatorProgress.value = 0;
  return true;
}

function completeGeneration(message, statusLabel) {
  generatorStatus.textContent = message;
  sampleStatus.textContent = statusLabel || "Ready";
  generatorProgress.value = 100;
  generateBtn.disabled = false;
  generateBtn.classList.remove("is-loading");
  updateSampleDisplay();
  setActiveStep(activeStepId);
}

function simulateGeneration(readyMessage, statusLabel) {
  let progress = 0;
  const interval = setInterval(() => {
    progress += Math.random() * 18 + 6;
    if (progress >= 100) {
      progress = 100;
      generatorProgress.value = progress;
      clearInterval(interval);
      completeGeneration(readyMessage || "Sample ready for the demo.", statusLabel);
      return;
    }
    generatorProgress.value = progress;
  }, 140);
}

async function generateSample() {
  if (!beginGeneration()) {
    return;
  }

  const useApi = window.location.protocol !== "file:";
  if (!useApi) {
    simulateGeneration("Sample ready for the demo.");
    return;
  }

  try {
    generatorProgress.value = 18;
    const response = await fetch("/api/generate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ scale: sampleState.rows }),
    });
    if (!response.ok) {
      throw new Error(`API error: ${response.status}`);
    }
    const data = await response.json();
    applyPipelineData(data);
    completeGeneration("Sample ready for the demo.");
  } catch (error) {
    console.warn("Live generator unavailable", error);
    generatorStatus.textContent = "Live generator unavailable, using simulated data.";
    sampleStatus.textContent = "Simulated";
    simulateGeneration("Simulated sample ready for the demo.", "Simulated");
  }
}

generateBtn.addEventListener("click", generateSample);

updateSampleDisplay();
setActiveStep(activeStepId);
initStepOrder();
initScrollTracking();
initTimelineNav();
initPreviewOffset();
updateStackPadding();

const cesopInfo = document.getElementById("cesopInfo");
const toggleCesopInfo = document.getElementById("toggleCesopInfo");
const closeCesopInfo = document.getElementById("closeCesopInfo");

function toggleCesopPanel() {
  if (cesopInfo) {
    cesopInfo.classList.toggle("is-open");
  }
}

if (toggleCesopInfo) {
  toggleCesopInfo.addEventListener("click", toggleCesopPanel);
}

if (closeCesopInfo) {
  closeCesopInfo.addEventListener("click", toggleCesopPanel);
}
