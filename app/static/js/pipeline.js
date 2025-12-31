// =============================================================================
// ANIMATION MODULE - Reusable preview transition animations
// =============================================================================

const PreviewAnimations = (() => {
  // Animation state
  let currentAnimation = null;
  let animationQueue = [];

  // Configuration
  const config = {
    lineDelay: 250,        // Delay between staggered lines (ms)
    scrollDuration: 600,   // Horizontal scroll duration (ms)
    fadeOutDuration: 400,  // Fade out duration (ms)
    fadeInDuration: 300,   // Fade in duration (ms)
    typewriterDelay: 80,   // Delay between typewriter characters (ms)
  };

  // Cancel any running animation
  function cancelAnimation() {
    if (currentAnimation) {
      currentAnimation.cancelled = true;
      currentAnimation = null;
    }
    animationQueue = [];
  }

  // Create animation context for tracking cancellation
  function createContext() {
    const ctx = { cancelled: false };
    currentAnimation = ctx;
    return ctx;
  }

  // Sleep utility that respects cancellation
  function sleep(ms, ctx) {
    return new Promise((resolve) => {
      const timeout = setTimeout(() => {
        resolve(!ctx.cancelled);
      }, ms);
      if (ctx.cancelled) {
        clearTimeout(timeout);
        resolve(false);
      }
    });
  }

  function getScrollTarget(preEl, targetCols) {
    if (!preEl || !Array.isArray(targetCols) || targetCols.length === 0) return null;

    // Find the first data row's cells to measure column positions (skip header)
    const rows = preEl.querySelectorAll('.csv-row');
    const firstDataRow = rows.length > 1 ? rows[1] : rows[0];
    if (!firstDataRow) return null;

    const cells = firstDataRow.querySelectorAll('.csv-cell');
    if (cells.length === 0) return null;

    // Calculate scroll position to center on the target columns
    const targetCells = targetCols
      .map((col) => cells[Math.min(Math.max(col, 0), cells.length - 1)])
      .filter(Boolean);
    if (targetCells.length === 0) return null;

    // Get the offset of the target cells relative to the pre element
    const preRect = preEl.getBoundingClientRect();
    const currentScroll = preEl.scrollLeft;

    const offsets = targetCells.map((cell) => {
      const cellRect = cell.getBoundingClientRect();
      const left = (cellRect.left - preRect.left) + currentScroll;
      return { left, right: left + cellRect.width };
    });
    const minOffset = Math.min(...offsets.map((offset) => offset.left));
    const maxOffset = Math.max(...offsets.map((offset) => offset.right));

    // Calculate where we need to scroll to show the target columns
    // Center the entire column range in the visible area
    const visibleWidth = preEl.clientWidth;
    const targetCenter = (minOffset + maxOffset) / 2;
    const maxScroll = Math.max(0, preEl.scrollWidth - visibleWidth);
    return Math.max(0, Math.min(targetCenter - (visibleWidth * 0.5), maxScroll));
  }

  // Animate horizontal scroll to highlight specific columns
  async function horizontalScrollToColumns(preEl, targetCols, ctx) {
    if (ctx.cancelled || !preEl) return false;

    const scrollTarget = getScrollTarget(preEl, targetCols);
    if (scrollTarget === null) return false;

    // Smooth scroll animation with longer duration
    const startScroll = preEl.scrollLeft;
    const distance = scrollTarget - startScroll;
    const startTime = performance.now();
    const duration = 800; // Slower, smoother scroll

    return new Promise((resolve) => {
      function animate(time) {
        if (ctx.cancelled) {
          resolve(false);
          return;
        }

        const elapsed = time - startTime;
        const progress = Math.min(elapsed / duration, 1);
        // Ease in-out quad for smoother feel
        const eased = progress < 0.5
          ? 2 * progress * progress
          : 1 - Math.pow(-2 * progress + 2, 2) / 2;

        preEl.scrollLeft = startScroll + (distance * eased);

        if (progress < 1) {
          requestAnimationFrame(animate);
        } else {
          resolve(true);
        }
      }
      requestAnimationFrame(animate);
    });
  }

  function jumpToColumns(preEl, targetCols) {
    if (!preEl) return false;
    const scrollTarget = getScrollTarget(preEl, targetCols);
    if (scrollTarget === null) return false;
    preEl.scrollLeft = scrollTarget;
    return true;
  }

  // Cross-border filter animation sequence
  // 1. Scroll to country columns
  // 2. Highlight non-cross-border (excluded) rows
  // Note: Scroll-out happens on transition to next step, not here
  async function crossBorderFilterSequence(preEl, ctx) {
    if (ctx.cancelled || !preEl) return false;

    // Step 1: Smooth scroll to country columns (payer_country=4, payee_country=6)
    await horizontalScrollToColumns(preEl, [4, 6], ctx);
    if (ctx.cancelled) return false;

    await sleep(300, ctx);
    if (ctx.cancelled) return false;

    // Step 2: Highlight excluded rows (domestic ones that fail the filter)
    const excludedRows = preEl.querySelectorAll('.csv-row[data-excluded="true"]');

    // Stagger the highlighting for visual effect
    for (let i = 0; i < excludedRows.length; i++) {
      if (ctx.cancelled) return false;
      excludedRows[i].classList.add('csv-row-highlight-excluded');
      await sleep(150, ctx);
    }

    return true;
  }

  // Animate content scrolling up and out of view
  async function scrollLinesOut(contentEl, direction = 'up', ctx) {
    if (ctx.cancelled || !contentEl) return false;

    // Add scroll-out animation to the pre element content
    const translateY = direction === 'up' ? '-30px' : '30px';

    // Apply transition
    contentEl.style.transition = 'transform 0.35s ease-in, opacity 0.35s ease-in';
    contentEl.style.transform = `translateY(${translateY})`;
    contentEl.style.opacity = '0';

    const completed = await sleep(350, ctx);

    // Reset styles (content will be replaced anyway)
    contentEl.style.transition = '';
    contentEl.style.transform = '';
    contentEl.style.opacity = '';

    return completed;
  }

  // Animate lines appearing one by one with stagger effect
  async function staggeredLinesIn(preEl, content, ctx, options = {}) {
    if (ctx.cancelled || !preEl) return false;

    const {
      delay = config.lineDelay,
      fromDirection = 'below',
      preserveFormatting = true,
    } = options;

    const lines = content.split('\n');
    preEl.innerHTML = '';
    preEl.classList.add('anim-stagger-container');

    for (let i = 0; i < lines.length; i++) {
      if (ctx.cancelled) return false;

      const lineEl = document.createElement('span');
      lineEl.className = `anim-line anim-line-hidden anim-from-${fromDirection}`;
      lineEl.innerHTML = preserveFormatting ? lines[i] : escapeHtml(lines[i]);
      preEl.appendChild(lineEl);

      // Trigger reflow then animate in
      void lineEl.offsetHeight;

      await sleep(delay, ctx);
      if (ctx.cancelled) return false;

      lineEl.classList.remove('anim-line-hidden');
      lineEl.classList.add('anim-line-visible');
    }

    return true;
  }

  // Typewriter effect - characters appear one by one per line
  async function typewriterLines(preEl, content, ctx, options = {}) {
    if (ctx.cancelled || !preEl) return false;

    const {
      charDelay = config.typewriterDelay,
      lineDelay = config.lineDelay,
    } = options;

    const lines = content.split('\n');
    preEl.innerHTML = '';
    preEl.classList.add('anim-typewriter-container');

    for (let i = 0; i < lines.length; i++) {
      if (ctx.cancelled) return false;

      const lineEl = document.createElement('span');
      lineEl.className = 'anim-typewriter-line';
      preEl.appendChild(lineEl);

      const line = lines[i];
      for (let j = 0; j < line.length; j++) {
        if (ctx.cancelled) return false;

        lineEl.textContent += line[j];
        await sleep(charDelay, ctx);
      }

      if (i < lines.length - 1) {
        await sleep(lineDelay - charDelay * line.length, ctx);
      }
    }

    return true;
  }

  // Scroll lines in from a direction
  async function scrollLinesIn(contentEl, direction = 'below', ctx, durationMs = null) {
    if (ctx.cancelled || !contentEl) return false;

    const duration = durationMs ?? config.fadeInDuration;
    if (durationMs !== null) {
      contentEl.style.setProperty('--scroll-in-duration', `${duration}ms`);
    } else {
      contentEl.style.removeProperty('--scroll-in-duration');
    }

    contentEl.classList.add('anim-scroll-in', `anim-from-${direction}`);

    // Trigger reflow
    void contentEl.offsetHeight;

    contentEl.classList.add('anim-scroll-in-active');

    const completed = await sleep(duration, ctx);

    contentEl.classList.remove('anim-scroll-in', `anim-from-${direction}`, 'anim-scroll-in-active');
    if (durationMs !== null) {
      contentEl.style.removeProperty('--scroll-in-duration');
    }
    return completed;
  }

  // Highlight flash animation for specific elements
  async function flashHighlight(preEl, selector, ctx) {
    if (ctx.cancelled || !preEl) return false;

    const elements = preEl.querySelectorAll(selector);
    elements.forEach(el => el.classList.add('anim-flash'));

    if (!await sleep(800, ctx)) return false;

    elements.forEach(el => el.classList.remove('anim-flash'));
    return true;
  }

  // Cross-fade between two content states
  async function crossFade(preEl, newContent, ctx, isHtml = false) {
    if (ctx.cancelled || !preEl) return false;

    preEl.classList.add('anim-fade-out');

    if (!await sleep(config.fadeOutDuration / 2, ctx)) return false;

    if (isHtml) {
      preEl.innerHTML = newContent;
    } else {
      preEl.textContent = newContent;
    }

    preEl.classList.remove('anim-fade-out');
    preEl.classList.add('anim-fade-in');

    if (!await sleep(config.fadeInDuration, ctx)) return false;

    preEl.classList.remove('anim-fade-in');
    return true;
  }

  // Combined animation: scroll out current, then stagger in new content
  async function transitionWithStagger(preEl, newContent, ctx, options = {}) {
    if (ctx.cancelled || !preEl) return false;

    // Scroll current content out
    if (!await scrollLinesOut(preEl, options.outDirection || 'up', ctx)) return false;

    // Stagger new content in
    if (!await staggeredLinesIn(preEl, newContent, ctx, options)) return false;

    return true;
  }

  // Public API
  return {
    config,
    cancelAnimation,
    createContext,
    sleep,
    horizontalScrollToColumns,
    jumpToColumns,
    scrollLinesOut,
    scrollLinesIn,
    staggeredLinesIn,
    typewriterLines,
    flashHighlight,
    crossFade,
    transitionWithStagger,
    crossBorderFilterSequence,
  };
})();

// =============================================================================
// TOOLTIP MODULE - Interactive tooltips for highlighted elements
// =============================================================================

const Tooltips = (() => {
  let tooltipEl = null;
  let activeTarget = null;

  // Tooltip content definitions for different contexts
  const tooltipContent = {
    'cross-border-same': {
      title: 'Domestic Transaction',
      body: 'Payer and payee are in the same country. This transaction is excluded from CESOP reporting.',
      type: 'excluded',
    },
    'cross-border-diff': {
      title: 'Cross-border Transaction',
      body: 'Payer and payee are in different EU Member States. This transaction is in scope for CESOP.',
      type: 'included',
    },
    'error-invalid-country': {
      title: 'Invalid Country Code',
      body: 'Country code must be a valid ISO 3166-1 alpha-2 code (e.g., "DE", "FR"). "ZZ" is not valid.',
      type: 'error',
    },
    'error-invalid-currency': {
      title: 'Invalid Currency Code',
      body: 'Currency must be a valid ISO 4217 code. "EURO" should be "EUR".',
      type: 'error',
    },
    'error-missing-payee-name': {
      title: 'Missing Payee Name',
      body: 'Payee name is required for reporting. This field is blank.',
      type: 'error',
    },
    'error-invalid-account-type': {
      title: 'Invalid Account Type',
      body: 'Account type must be a supported identifier (e.g., IBAN/OBAN/BIC/Other). "BADTYPE" is not valid.',
      type: 'error',
    },
    'error-invalid-account': {
      title: 'Invalid Account Identifier',
      body: 'Account value does not match the expected format for its type (e.g., IBAN checksum).',
      type: 'error',
    },
    'error-invalid-payer-source': {
      title: 'Invalid Payer Source',
      body: 'Payer MS source must be a valid identifier source (e.g., IBAN/OBAN/BIC/Other).',
      type: 'error',
    },
    'error-payer-role': {
      title: 'Payer PSP Role',
      body: 'This PSP is the payer\'s PSP. Only report if payee PSP is outside the EU.',
      type: 'warning',
    },
    'threshold-above': {
      title: 'Above Threshold',
      body: 'This payee received >25 cross-border payments this quarter. Reporting is required.',
      type: 'included',
    },
    'threshold-below': {
      title: 'Below Threshold',
      body: 'This payee received ≤25 cross-border payments. No reporting required for this payee.',
      type: 'excluded',
    },
    'correction-applied': {
      title: 'Correction Applied',
      body: 'This field was automatically corrected based on deterministic rules.',
      type: 'success',
    },
  };

  function init() {
    // Create tooltip element if it doesn't exist
    if (!tooltipEl) {
      tooltipEl = document.createElement('div');
      tooltipEl.className = 'preview-tooltip';
      tooltipEl.innerHTML = `
        <div class="tooltip-header">
          <span class="tooltip-icon"></span>
          <span class="tooltip-title"></span>
        </div>
        <div class="tooltip-body"></div>
      `;
      document.body.appendChild(tooltipEl);
    }
  }

  function show(target, contentKey, customContent = null) {
    if (!tooltipEl) init();

    const content = customContent || tooltipContent[contentKey];
    if (!content) return;

    activeTarget = target;

    tooltipEl.querySelector('.tooltip-title').textContent = content.title;
    tooltipEl.querySelector('.tooltip-body').textContent = content.body;
    tooltipEl.className = `preview-tooltip is-visible tooltip-${content.type || 'info'}`;

    positionTooltip(target);
  }

  function hide() {
    if (tooltipEl) {
      tooltipEl.classList.remove('is-visible');
    }
    activeTarget = null;
  }

  function positionTooltip(target) {
    if (!tooltipEl || !target) return;

    const rect = target.getBoundingClientRect();
    const tooltipRect = tooltipEl.getBoundingClientRect();

    let top = rect.bottom + 8;
    let left = rect.left + (rect.width / 2) - (tooltipRect.width / 2);

    // Keep within viewport
    if (left < 8) left = 8;
    if (left + tooltipRect.width > window.innerWidth - 8) {
      left = window.innerWidth - tooltipRect.width - 8;
    }

    // Flip above if not enough space below
    if (top + tooltipRect.height > window.innerHeight - 8) {
      top = rect.top - tooltipRect.height - 8;
      tooltipEl.classList.add('tooltip-above');
    } else {
      tooltipEl.classList.remove('tooltip-above');
    }

    tooltipEl.style.top = `${top}px`;
    tooltipEl.style.left = `${left}px`;
  }

  function attachToPreview(preEl) {
    if (!preEl) return;

    // Use mouseover/mouseout instead of mouseenter/mouseleave for proper event delegation
    // mouseover/mouseout bubble up, while mouseenter/mouseleave don't
    preEl.addEventListener('mouseover', handleMouseOver);
    preEl.addEventListener('mouseout', handleMouseOut);
    preEl.addEventListener('click', handleClick);
  }

  let hoverTarget = null;

  function resolveTooltipTarget(rawTarget) {
    if (!rawTarget) {
      return null;
    }
    const element = rawTarget.nodeType === 1 ? rawTarget : rawTarget.parentElement;
    return element ? element.closest('[data-tooltip]') : null;
  }

  function handleMouseOver(e) {
    const target = resolveTooltipTarget(e.target);
    if (target && target !== hoverTarget) {
      hoverTarget = target;
      const key = target.dataset.tooltip;
      show(target, key);
    }
  }

  function handleMouseOut(e) {
    const target = resolveTooltipTarget(e.target);
    if (target) {
      // Check if we're moving to another element inside the same tooltip target
      const relatedTarget = e.relatedTarget;
      if (!target.contains(relatedTarget)) {
        hoverTarget = null;
        hide();
      }
    }
  }

  function handleClick(e) {
    const target = resolveTooltipTarget(e.target);
    if (target) {
      e.stopPropagation();
      const key = target.dataset.tooltip;
      if (activeTarget === target) {
        hide();
      } else {
        show(target, key);
      }
    }
  }

  // Close tooltip when clicking outside
  document.addEventListener('click', (e) => {
    if (tooltipEl && !tooltipEl.contains(e.target) && !resolveTooltipTarget(e.target)) {
      hide();
    }
  });

  return {
    init,
    show,
    hide,
    attachToPreview,
    tooltipContent,
  };
})();

// =============================================================================
// APPLICATION STATE
// =============================================================================

const sampleState = {
  rows: 10000,
  size: "~3.3 MB",
  payees: 417,
  crossBorder: 8260,
  nonCrossBorder: 1740,
  reportable: 2540,
  belowThreshold: 5720,
  reportablePayees: 29,
  memberStates: 6,
  memberStateCodes: ["AT", "BG", "CZ", "DK", "ES", "FI"],
  xmlFiles: [
    "cesop_2025_Q4_AT_AFBQBGKT.xml",
    "cesop_2025_Q4_BG_AFBQBGKT.xml",
    "cesop_2025_Q4_CZ_AFBQBGKT.xml",
    "cesop_2025_Q4_DK_AFBQBGKT.xml",
    "cesop_2025_Q4_ES_AFBQBGKT.xml",
    "cesop_2025_Q4_FI_AFBQBGKT.xml",
  ],
  errors: 458,
  corrections: 458,
  preflightCorruptErrors: 458,
  preflightCorrectedErrors: 0,
  reports: 6,
  passRate: "100%",
  validationTime: "0.7s",
};

const steps = [
  {
    id: "raw",
    title: "Raw Ingest",
    meta: "{rows} rows loaded",
    preview: {
      type: "csv",
      header: [
        "payment_id",
        "execution_time",
        "amount",
        "currency",
        "payer_country",
        "payer_ms_source",
        "payee_country",
        "payee_id",
        "payee_name",
        "payee_account",
        "payee_account_type",
        "payee_tax_id",
        "payee_vat_id",
        "payee_email",
        "payee_web",
        "payee_address_line",
        "payee_city",
        "payee_postcode",
        "payment_method",
        "initiated_at_pos",
        "is_refund",
        "corr_payment_id",
        "psp_role",
        "payee_psp_id",
        "payee_psp_name",
        "psp_id",
        "psp_name",
      ],
      rows: [
        // Cross-border: HU -> FI
        ["fea76c07-fe56-4f95-9b1a-9cf62ec62a9f", "2025-10-27T06:23:08.996Z", "82.45", "HUF", "HU", "IBAN", "FI", "MER000216", "Victorious Industries LLC", "FI9257441723360727", "IBAN", "", "", "billing@victorious-industries-llc.example", "https://victorious-industries-llc.example", "207 Garden St", "Prague", "09331", "Marketplace", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // DOMESTIC: FR -> FR (will be excluded)
        ["f2a92871-3afe-45c8-adbe-804cf08fcb71", "2025-11-19T20:01:29.814Z", "381.63", "EUR", "FR", "IBAN", "FR", "MER000366", "Ramillies Consulting Group", "FR7302301496564619556692558", "IBAN", "", "FR599011876", "billing@ramillies-consulting-group.example", "https://ramillies-consulting-group.example", "18 Mill St", "Dublin", "99458", "Money Remittance", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // Cross-border: DK -> ES
        ["391488b3-08e0-44d5-9f5b-9fbdcbaf3d5b", "2025-12-09T13:16:52.328Z", "1159.19", "DKK", "DK", "IBAN", "ES", "MER000288", "Faramir Innovation NV", "ES9727918921959377644152", "IBAN", "TAXES85136604", "", "billing@faramir-innovation-nv.example", "https://faramir-innovation-nv.example", "180 Oak St", "Amsterdam", "59450", "Bank transfer", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // DOMESTIC: DE -> DE (will be excluded)
        ["a8b3c2d1-e4f5-6789-abcd-ef0123456789", "2025-11-05T14:32:11.442Z", "892.50", "EUR", "DE", "IBAN", "DE", "MER000192", "Berlin Tech Solutions GmbH", "DE89370400440532013000", "IBAN", "TAXDE12345678", "DE123456789", "billing@berlin-tech.example", "https://berlin-tech.example", "45 Unter den Linden", "Berlin", "10117", "Card payment", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // Cross-border: SI -> PT
        ["561b87fe-b0ae-46e0-8e00-6bfbd2ccd24c", "2025-10-22T18:28:06.814Z", "1545.23", "EUR", "SI", "IBAN", "PT", "MER000075", "Hermes Logistics LLC", "PT87150602618998901532246", "IBAN", "TAXPT75841280", "PT429556609", "billing@hermes-logistics-llc.example", "https://hermes-logistics-llc.example", "110 South St", "Warsaw", "96231", "Money Remittance", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // Cross-border: RO -> AT
        ["f068e199-0be8-4866-9df2-9e07b846a0e0", "2025-10-18T23:18:27.076Z", "1156.99", "RON", "RO", "IBAN", "AT", "MER000364", "Malaya Collective NV", "AT471597898563119790", "IBAN", "TAXAT68443464", "AT756956672", "billing@malaya-collective-nv.example", "https://malaya-collective-nv.example", "30 Lake St", "Ljubljana", "01753", "E-money", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // Cross-border: RO -> NL
        ["16d32802-58d4-4308-853a-23a32c3e3dc5", "2025-12-20T08:05:25.039Z", "228.83", "RON", "RO", "IBAN", "NL", "MER000127", "Caledonia Partners NV", "NL4296581176785813", "IBAN", "TAXNL86159362", "NL901866680", "billing@caledonia-partners-nv.example", "https://caledonia-partners-nv.example", "42 Station St", "Copenhagen", "71627", "Marketplace", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
      ],
    },
    // Animation config for this step
    animation: {
      onEnter: null,
      onExit: 'scrollOut',
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
        "payment_id",
        "execution_time",
        "amount",
        "currency",
        "payer_country",
        "payer_ms_source",
        "payee_country",
        "payee_id",
        "payee_name",
        "payee_account",
        "payee_account_type",
        "payee_tax_id",
        "payee_vat_id",
        "payee_email",
        "payee_web",
        "payee_address_line",
        "payee_city",
        "payee_postcode",
        "payment_method",
        "initiated_at_pos",
        "is_refund",
        "corr_payment_id",
        "psp_role",
        "payee_psp_id",
        "payee_psp_name",
        "psp_id",
        "psp_name",
      ],
      rows: [
        // Cross-border: HU -> FI (included)
        ["fea76c07-fe56-4f95-9b1a-9cf62ec62a9f", "2025-10-27T06:23:08.996Z", "82.45", "HUF", "HU", "IBAN", "FI", "MER000216", "Victorious Industries LLC", "FI9257441723360727", "IBAN", "", "", "billing@victorious-industries-llc.example", "https://victorious-industries-llc.example", "207 Garden St", "Prague", "09331", "Marketplace", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // DOMESTIC: FR -> FR (excluded - highlighted)
        ["f2a92871-3afe-45c8-adbe-804cf08fcb71", "2025-11-19T20:01:29.814Z", "381.63", "EUR", "FR", "IBAN", "FR", "MER000366", "Ramillies Consulting Group", "FR7302301496564619556692558", "IBAN", "", "FR599011876", "billing@ramillies-consulting-group.example", "https://ramillies-consulting-group.example", "18 Mill St", "Dublin", "99458", "Money Remittance", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // Cross-border: DK -> ES (included)
        ["391488b3-08e0-44d5-9f5b-9fbdcbaf3d5b", "2025-12-09T13:16:52.328Z", "1159.19", "DKK", "DK", "IBAN", "ES", "MER000288", "Faramir Innovation NV", "ES9727918921959377644152", "IBAN", "TAXES85136604", "", "billing@faramir-innovation-nv.example", "https://faramir-innovation-nv.example", "180 Oak St", "Amsterdam", "59450", "Bank transfer", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // DOMESTIC: DE -> DE (excluded - highlighted)
        ["a8b3c2d1-e4f5-6789-abcd-ef0123456789", "2025-11-05T14:32:11.442Z", "892.50", "EUR", "DE", "IBAN", "DE", "MER000192", "Berlin Tech Solutions GmbH", "DE89370400440532013000", "IBAN", "TAXDE12345678", "DE123456789", "billing@berlin-tech.example", "https://berlin-tech.example", "45 Unter den Linden", "Berlin", "10117", "Card payment", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // Cross-border: SI -> PT (included)
        ["561b87fe-b0ae-46e0-8e00-6bfbd2ccd24c", "2025-10-22T18:28:06.814Z", "1545.23", "EUR", "SI", "IBAN", "PT", "MER000075", "Hermes Logistics LLC", "PT87150602618998901532246", "IBAN", "TAXPT75841280", "PT429556609", "billing@hermes-logistics-llc.example", "https://hermes-logistics-llc.example", "110 South St", "Warsaw", "96231", "Money Remittance", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // Cross-border: RO -> AT (included)
        ["f068e199-0be8-4866-9df2-9e07b846a0e0", "2025-10-18T23:18:27.076Z", "1156.99", "RON", "RO", "IBAN", "AT", "MER000364", "Malaya Collective NV", "AT471597898563119790", "IBAN", "TAXAT68443464", "AT756956672", "billing@malaya-collective-nv.example", "https://malaya-collective-nv.example", "30 Lake St", "Ljubljana", "01753", "E-money", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        // Cross-border: RO -> NL (included)
        ["16d32802-58d4-4308-853a-23a32c3e3dc5", "2025-12-20T08:05:25.039Z", "228.83", "RON", "RO", "IBAN", "NL", "MER000127", "Caledonia Partners NV", "NL4296581176785813", "IBAN", "TAXNL86159362", "NL901866680", "billing@caledonia-partners-nv.example", "https://caledonia-partners-nv.example", "42 Station St", "Copenhagen", "71627", "Marketplace", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
      ],
      // Highlight domestic transactions (rows 1 and 3, columns 4 and 6 are payer_country and payee_country)
      highlights: [
        { row: 1, cols: [4, 6], tooltip: 'cross-border-same', excluded: true },
        { row: 3, cols: [4, 6], tooltip: 'cross-border-same', excluded: true },
      ],
    },
    animation: {
      onEnter: 'crossBorderFilter', // Full animation: scroll to countries, highlight, strikethrough
      onExit: 'scrollOut', // Scroll out when transitioning to next step
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
        "MER000240 (CH) -> 123 payments\nMER000037 (NO) -> 123 payments\nMER000230 (NL) -> 112 payments\nMER000075 (PT) -> 105 payments\nMER000056 (DK) -> 103 payments\nMER000003 (BG) -> 103 payments\nMER000338 (CZ) -> 101 payments\nMER000345 (IT) -> 97 payments",
    },
    animation: {
      onEnter: 'staggerLines', // Lines appear one by one
      onExit: 'scrollOut',
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
        "payment_id",
        "execution_time",
        "amount",
        "currency",
        "payer_country",
        "payer_ms_source",
        "payee_country",
        "payee_id",
        "payee_name",
        "payee_account",
        "payee_account_type",
        "payee_tax_id",
        "payee_vat_id",
        "payee_email",
        "payee_web",
        "payee_address_line",
        "payee_city",
        "payee_postcode",
        "payment_method",
        "initiated_at_pos",
        "is_refund",
        "corr_payment_id",
        "psp_role",
        "payee_psp_id",
        "payee_psp_name",
        "psp_id",
        "psp_name",
      ],
      rows: [
        ["f3f457c9-fd49-4e24-9a8b-c6536711fb66", "2025-10-10T13:28:28.682Z", "2231.79", "EUR", "LU", "IBAN", "LV", "MER000397", "Harbor Foods Industries NV", "ACC7291045512", "BADTYPE", "TAXLV09116124", "LV203135331", "billing@harbor-foods-industries-nv.example", "https://harbor-foods-industries-nv.example", "138 High St", "Bucharest", "09153", "Bank transfer", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        ["f78bee54-3cbd-4bbc-92c1-dec735c95136", "2025-11-03T06:29:02.199Z", "1434.79", "EUR", "NL", "IBAN", "PL", "MER000406", "Golden Labs Collective BV", "ZZ00123456789012", "IBAN", "TAXPL91541504", "PL269978265", "billing@golden-labs-collective-bv.example", "https://golden-labs-collective-bv.example", "153 Garden St", "Paris", "85881", "Marketplace", "true", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        ["02f65696-b65e-45f5-a95b-ad8b3dbd750f", "2025-10-17T12:49:46.180Z", "483.76", "EUR", "BE", "IBAN", "NO", "MER000037", "", "NO9386011117947", "IBAN", "TAXNO58561246", "", "billing@trafalgar-studios-llc.example", "https://trafalgar-studios-llc.example", "69 Queen St", "Berlin", "98188", "Bank transfer", "false", "false", "", "PAYER", "AJYJCADYGCB", "Harborline Processing", "AFBQBGKT", "BlueBridge PSP"],
        ["f10cabe7-da0c-43c6-ae30-e8466b7f4a51", "2025-11-13T03:17:30.627Z", "1433.12", "EUR", "ZZ", "IBAN", "HU", "MER000330", "Black Prince Forge Group", "HU37653306410429983812139410", "IBAN", "TAXHU78695928", "HU937958009", "billing@black-prince-forge-group.example", "https://black-prince-forge-group.example", "32 Mill St", "Vienna", "32459", "E-money", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
        ["41865007-426c-4b78-8e08-df4e5c1094de", "2025-12-01T17:56:56.800Z", "383.21", "EUR", "LT", "BAD", "NO", "MER000037", "Trafalgar Studios LLC", "NO9386011117947", "IBAN", "TAXNO58561246", "", "billing@trafalgar-studios-llc.example", "https://trafalgar-studios-llc.example", "69 Queen St", "Berlin", "98188", "Marketplace", "false", "false", "", "PAYER", "AJYJCADYGCB", "Harborline Processing", "AFBQBGKT", "BlueBridge PSP"],
        ["d986a173-cca1-4caf-9ccd-beb3b6cfabf8", "2025-11-02T08:47:48.054Z", "2126.30", "EURO", "CY", "BIC", "CH", "MER000240", "Terror Technologies Ltd", "CH4092571161475049431", "IBAN", "", "", "billing@terror-technologies-ltd.example", "https://terror-technologies-ltd.example", "51 Cedar St", "Sofia", "43984", "Direct debit", "true", "false", "", "PAYER", "AJYJCADYGCB", "Harborline Processing", "AFBQBGKT", "BlueBridge PSP"],
        ["8230f78d-baf0-4bba-83c4-3d8ec1e9792c", "2025-12-31T19:44:47.688Z", "491.93", "EUR", "SK", "IBAN", "ZZ", "MER000406", "Golden Labs Collective BV", "PL61109024025234567890123456", "IBAN", "TAXPL91541504", "PL269978265", "billing@golden-labs-collective-bv.example", "https://golden-labs-collective-bv.example", "153 Garden St", "Paris", "85881", "E-money", "false", "false", "", "PAYEE", "AFBQBGKT", "BlueBridge PSP", "AFBQBGKT", "BlueBridge PSP"],
      ],
      highlights: [
        { row: 0, cols: [10], tooltip: 'error-invalid-account-type' },
        { row: 1, cols: [9], tooltip: 'error-invalid-account' },
        { row: 2, cols: [8], tooltip: 'error-missing-payee-name' },
        { row: 3, cols: [4], tooltip: 'error-invalid-country' },
        { row: 4, cols: [5], tooltip: 'error-invalid-payer-source' },
        { row: 5, cols: [3], tooltip: 'error-invalid-currency' },
        { row: 6, cols: [6], tooltip: 'error-invalid-country' },
      ],
    },
    animation: {
      onEnter: 'errorDetection', // Scroll in then scroll to error columns
      onExit: 'scrollOut',
    },
    rule: {
      title: "Data quality checks",
      body:
        "CESOP schema enforces strict code lists. Invalid values will fail official validation. Detecting issues early allows for correction before XML generation.",
      list: [
        "Country codes: ISO 3166-1 alpha-2",
        "Currency codes: ISO 4217",
        "Date formats: YYYY-MM-DD",
        "Account identifiers: IBAN/OBAN/BIC/Other",
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
      recordId: "Correction Summary",
      compact: false,
      changes: [
        { field: "payee_account_type", before: "BADTYPE", after: "IBAN", rule: "Account type fix" },
        { field: "payee_account", before: "ZZ00123456789012", after: "PL61109024025234567890123456", rule: "Account identifier fix" },
        { field: "payee_name", before: "", after: "Payee MER000037", rule: "Missing->placeholder" },
        { field: "payer_ms_source", before: "BAD", after: "IBAN", rule: "Identifier source fix" },
        { field: "currency", before: "EURO", after: "EUR", rule: "ISO 4217 normalize" },
      ],
    },
    animation: {
      onEnter: 'staggerDiff', // Show corrections one by one
      onExit: 'scrollOut',
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
        "cesop_2025_Q4_AT_AFBQBGKT.xml",
        "cesop_2025_Q4_BG_AFBQBGKT.xml",
        "cesop_2025_Q4_CZ_AFBQBGKT.xml",
        "cesop_2025_Q4_DK_AFBQBGKT.xml",
        "cesop_2025_Q4_ES_AFBQBGKT.xml",
        "cesop_2025_Q4_FI_AFBQBGKT.xml",
      ],
      value: `<CESOP xmlns="urn:ec.europa.eu:taxud:fiscalis:cesop:v1" xmlns:cm="urn:eu:taxud:commontypes:v1" xmlns:iso="urn:eu:taxud:isotypes:v1" version="4.03">
  <MessageSpec>
    <TransmittingCountry>AT</TransmittingCountry>
    <MessageType>PMT</MessageType>
    <MessageTypeIndic>CESOP100</MessageTypeIndic>
    <MessageRefId>7ef69a98-3176-4242-b7cc-68f513e2f147</MessageRefId>
    <ReportingPeriod>
      <Quarter>4</Quarter>
      <Year>2025</Year>
    </ReportingPeriod>
    <Timestamp>2025-12-29T17:36:50.590Z</Timestamp>
  </MessageSpec>
  <PaymentDataBody>
    <ReportingPSP>
      <PSPId PSPIdType="BIC">AFBQBGKT</PSPId>
      <Name nameType="BUSINESS">BlueBridge PSP</Name>
    </ReportingPSP>
    <ReportedPayee>
      <Name nameType="BUSINESS">Malaya Collective NV</Name>
      <Country>AT</Country>`,
    },
    animation: {
      onEnter: 'typewriterXml', // XML loads in line by line with typewriter effect
      onExit: 'scrollOut',
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
        "Starting validation of folder: \"/cesop/data/output\"\n> Checking schema compliance...\n> Validating business rules...\n> Verifying cross-field consistency...\nGenerated CSV output: /cesop/data/output/validation_output.csv\nValidation successfully finished.\n\nResult: PASS (100%)",
    },
    animation: {
      onEnter: 'staggerLines', // Validation output appears line by line
      onExit: 'scrollOut',
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
        "Pipeline complete\n\nDeliverables:\n  - {reports} validated XML files\n  - Correction audit log\n  - Validation reports\n\nReady for CESOP portal submission",
    },
    animation: {
      onEnter: 'staggerLines',
      onExit: null,
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

const stepIndexMap = new Map(steps.map((step, index) => [step.id, index]));
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
let previousStepId = null;
let scrollTicking = false;
let isAnimating = false;
let lastStepChangeAt = 0;
let stepChangeToken = 0;

const stepElements = Array.from(document.querySelectorAll(".step"));
const timelineItems = Array.from(document.querySelectorAll(".timeline-item"));
const previewTitle = document.getElementById("previewTitle");
const previewMeta = document.getElementById("previewMeta");
const previewCode = document.getElementById("previewCode");
const previewContent = previewCode
  ? (previewCode.querySelector(".preview-content") || (() => {
    const content = document.createElement("span");
    content.className = "preview-content";
    previewCode.appendChild(content);
    return content;
  })())
  : null;
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

  // Build a map of highlighted cells with their tooltip keys and excluded status
  const markMap = new Map();
  const excludedRows = new Set();
  const headerOffset = header.length > 0 ? 1 : 0;
  const maxRows = Math.max(0, MAX_PREVIEW_LINES - headerOffset);
  const visibleRows = rows.slice(0, maxRows);

  highlights.forEach((highlight) => {
    if (!highlight || !Array.isArray(highlight.cols)) {
      return;
    }
    const rowIndex = (highlight.row || 0) + headerOffset;
    highlight.cols.forEach((col) => {
      markMap.set(`${rowIndex}:${col}`, {
        tooltip: highlight.tooltip || null,
        excluded: highlight.excluded || false,
      });
    });
    if (highlight.excluded) {
      excludedRows.add(rowIndex);
    }
  });

  const allRows = header.length > 0 ? [header, ...visibleRows] : visibleRows;
  const lines = allRows.map((row, rowIndex) => {
    const isExcludedRow = excludedRows.has(rowIndex);
    // Data attributes for animation targeting
    const rowAttrs = isExcludedRow
      ? `class="csv-row" data-excluded="true" data-row="${rowIndex}"`
      : `class="csv-row" data-row="${rowIndex}"`;

    const cells = row.map((value, colIndex) => {
      const classes = ["csv-cell", csvPalette[colIndex % csvPalette.length]];
      if (rowIndex === 0 && header.length > 0) {
        classes.push("csv-header");
      }

      const markInfo = markMap.get(`${rowIndex}:${colIndex}`);
      let tooltipAttr = '';
      if (markInfo) {
        classes.push("csv-mark");
        if (markInfo.excluded) {
          classes.push("csv-mark-excluded");
        }
        if (markInfo.tooltip) {
          tooltipAttr = ` data-tooltip="${markInfo.tooltip}"`;
          classes.push("has-tooltip");
        }
      }

      const text = escapeHtml(value ?? "");
      const suffix = colIndex < row.length - 1 ? "," : "";
      return `<span class="${classes.join(" ")}"${tooltipAttr}>${text}${suffix}</span>`;
    }).join("");

    return `<span ${rowAttrs}>${cells}</span>`;
  });

  // Join without newlines since css-row has display:block
  return clampPreviewLines(lines).join("");
}

function renderDiffSnippet(preview) {
  if (!preview || !Array.isArray(preview.changes) || preview.changes.length === 0) {
    return "No corrections applied.";
  }

  const lines = [];

  // Always show header
  const headerText = preview.recordId || "Correction Summary";
  lines.push(`<span class="diff-header">${escapeHtml(headerText)}</span>`);

  let shown = 0;
  for (const change of preview.changes) {
    // Check if we have room for this change (2 lines for before/after, optionally 1 for rule)
    const linesNeeded = change.rule ? 3 : 2;
    if (lines.length + linesNeeded > MAX_PREVIEW_LINES) {
      break;
    }

    // Each diff change is a self-contained block
    const beforeLine = `<span class="diff-line diff-remove">- ${escapeHtml(change.field)}: ${escapeHtml(change.before)}</span>`;
    const afterLine = `<span class="diff-line diff-add">+ ${escapeHtml(change.field)}: ${escapeHtml(change.after)}</span>`;

    lines.push(beforeLine);
    lines.push(afterLine);

    if (change.rule && lines.length < MAX_PREVIEW_LINES) {
      lines.push(`<span class="diff-line diff-rule">  ↳ ${escapeHtml(change.rule)}</span>`);
    }
    shown += 1;
  }

  // Show summary at the bottom if there are more changes
  if (preview.changes.length > shown && lines.length < MAX_PREVIEW_LINES) {
    const remaining = preview.changes.length - shown;
    lines.push(`<span class="diff-line diff-more">...and ${remaining} more correction${remaining > 1 ? 's' : ''}</span>`);
  }

  return lines.slice(0, MAX_PREVIEW_LINES).join("");
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

// Store current metric values for animation comparison
let currentMetricValues = {};

function animateMetricsUpdate(newMetrics) {
  const existingMetrics = previewMetrics.querySelectorAll('.metric');
  const newValues = {};

  // Build map of new values
  newMetrics.forEach(metric => {
    newValues[metric.label] = fillTemplate(metric.value);
  });

  // Check if we need to animate (values changed)
  let hasChanges = existingMetrics.length !== newMetrics.length;
  if (!hasChanges) {
    newMetrics.forEach(metric => {
      if (currentMetricValues[metric.label] !== newValues[metric.label]) {
        hasChanges = true;
      }
    });
  }

  if (hasChanges && existingMetrics.length > 0) {
    // Delay the metric animation by 500ms to let other animations settle
    setTimeout(() => {
      // Animate out existing values
      const currentMetrics = previewMetrics.querySelectorAll('.metric');
      currentMetrics.forEach((metricEl, index) => {
        const valueEl = metricEl.querySelector('.value');
        if (valueEl) {
          valueEl.style.setProperty('--metric-delay', `${index * 50}ms`);
          valueEl.classList.add('metric-value-exit');
        }
      });

      // After exit animation, update and animate in
      setTimeout(() => {
        renderMetrics(newMetrics, true);
        currentMetricValues = newValues;
      }, 150);
    }, 500);
  } else {
    // No animation needed, just render
    renderMetrics(newMetrics, false);
    currentMetricValues = newValues;
  }
}

function renderMetrics(metrics, animate) {
  previewMetrics.innerHTML = "";
  metrics.forEach((metric, index) => {
    const metricEl = document.createElement("div");
    metricEl.className = "metric";

    const label = document.createElement("div");
    label.className = "label";
    label.textContent = metric.label;

    const value = document.createElement("div");
    value.className = "value";
    if (animate) {
      value.style.setProperty('--metric-delay', `${index * 50}ms`);
      value.classList.add('metric-value-enter');
    }
    value.textContent = fillTemplate(metric.value);

    metricEl.appendChild(label);
    metricEl.appendChild(value);
    previewMetrics.appendChild(metricEl);
  });
}

// Store current rule content for animation comparison
let currentRuleTitle = '';

function animateRulePanelUpdate(newRule) {
  if (!newRule) return;

  const hasChanges = currentRuleTitle !== newRule.title;

  if (hasChanges && currentRuleTitle !== '') {
    // Animate out existing content
    ruleTitle.classList.add('rule-element-exit');
    ruleBody.classList.add('rule-element-exit');
    ruleList.querySelectorAll('li').forEach(li => li.classList.add('rule-element-exit'));

    // After exit animation completes, update and animate in
    setTimeout(() => {
      renderRulePanel(newRule, true);
      currentRuleTitle = newRule.title;
    }, 180);
  } else {
    // No animation needed, just render
    renderRulePanel(newRule, false);
    currentRuleTitle = newRule.title;
  }
}

function renderRulePanel(rule, animate) {
  // Clear exit animations
  ruleTitle.classList.remove('rule-element-exit');
  ruleBody.classList.remove('rule-element-exit');
  ruleList.classList.remove('rule-element-exit');

  // Set title
  ruleTitle.textContent = rule.title;
  if (animate) {
    ruleTitle.style.setProperty('--rule-delay', '0ms');
    ruleTitle.classList.add('rule-element-enter');
  } else {
    ruleTitle.classList.remove('rule-element-enter');
  }

  // Set body
  ruleBody.textContent = rule.body;
  if (animate) {
    ruleBody.style.setProperty('--rule-delay', '80ms');
    ruleBody.classList.add('rule-element-enter');
  } else {
    ruleBody.classList.remove('rule-element-enter');
  }

  // Set list items with staggered animation
  ruleList.innerHTML = "";
  rule.list.forEach((item, index) => {
    const li = document.createElement("li");
    li.textContent = item;
    if (animate) {
      li.style.setProperty('--rule-delay', `${160 + index * 60}ms`);
      li.classList.add('rule-element-enter');
    }
    ruleList.appendChild(li);
  });
}

function setPreview(step, animate = false) {
  previewTitle.textContent = step.title;
  previewMeta.textContent = fillTemplate(step.meta);

  previewCode.classList.toggle("csv-snippet", step.preview.type === "csv");
  previewCode.classList.toggle("diff-snippet", step.preview.type === "diff");

  const previewTarget = previewContent || previewCode;

  // Clear all animation classes to prevent ghost content
  previewTarget.classList.remove(
    'anim-stagger-container',
    'anim-typewriter-container',
    'anim-scroll-out',
    'anim-scroll-up',
    'anim-scroll-down',
    'anim-scroll-in',
    'anim-from-below',
    'anim-from-above',
    'anim-scroll-in-active',
    'anim-fade-out',
    'anim-fade-in'
  );
  previewTarget.style.transition = '';
  previewTarget.style.transform = '';
  previewTarget.style.opacity = '';

  // Reset scroll position
  previewCode.scrollLeft = 0;

  if (step.preview.type === "csv") {
    previewTarget.innerHTML = renderCsvSnippet(step.preview);
  } else if (step.preview.type === "xml") {
    previewTarget.textContent = renderXmlSnippet(step.preview);
  } else if (step.preview.type === "diff") {
    previewTarget.innerHTML = renderDiffSnippet(step.preview);
  } else if (step.preview.type === "html") {
    previewTarget.innerHTML = step.preview.value;
  } else {
    previewTarget.textContent = fillTemplate(clampPreviewText(step.preview.value));
  }

  // Animate metrics transition
  animateMetricsUpdate(step.metrics);

  // Animate rule panel transition
  animateRulePanelUpdate(step.rule);

  requestAnimationFrame(updatePreviewOffset);
}

function resolveErrorScrollCols(step, previewEl) {
  if (step && step.preview && Array.isArray(step.preview.highlights)) {
    const highlight = step.preview.highlights.find(
      (item) => item && Array.isArray(item.cols) && item.cols.length > 0
    );
    if (highlight) {
      return highlight.cols;
    }
  }

  if (previewEl) {
    const mark = previewEl.querySelector('.csv-row .csv-mark');
    if (mark) {
      const row = mark.closest('.csv-row');
      if (row) {
        const cells = Array.from(row.querySelectorAll('.csv-cell'));
        const idx = cells.indexOf(mark);
        if (idx >= 0) {
          return [idx];
        }
      }
    }
  }

  return null;
}

function applyStepFocus(step) {
  if (!step || !step.animation || !previewCode) {
    return;
  }

  switch (step.animation.onEnter) {
    case 'crossBorderFilter':
    case 'scrollToCountries': {
      PreviewAnimations.jumpToColumns(previewCode, [4, 6]);
      const excludedRows = previewCode.querySelectorAll('.csv-row[data-excluded="true"]');
      excludedRows.forEach((row) => row.classList.add('csv-row-highlight-excluded'));
      break;
    }
    case 'errorDetection':
      {
        const cols = resolveErrorScrollCols(step, previewCode);
        if (cols) {
          PreviewAnimations.jumpToColumns(previewCode, cols);
        }
      }
      break;
    default:
      break;
  }
}

function shouldSkipExit(prevStepId, nextStepId) {
  if (!prevStepId || !nextStepId) {
    return false;
  }
  if (prevStepId === nextStepId) {
    return true;
  }
  const sharePreview = (prevStepId === "raw" && nextStepId === "cross-border")
    || (prevStepId === "cross-border" && nextStepId === "raw");
  return sharePreview;
}

function shouldForceEnter(step, isRapid) {
  if (!step || !step.animation || !step.animation.onEnter) {
    return false;
  }
  if (!isRapid) {
    return false;
  }
  return step.id === "errors";
}

// Animation runner - executes animations based on step config
async function runStepAnimation(step, animationType, fromStep = null) {
  if (!step || !step.animation) return;

  const animName = step.animation[animationType];
  if (!animName) return;

  const ctx = PreviewAnimations.createContext();
  const contentEl = previewContent || previewCode;

  switch (animName) {
    case 'scrollToCountries':
      // Scroll horizontally to highlight country columns (4 and 6)
      await PreviewAnimations.horizontalScrollToColumns(previewCode, [4, 6], ctx);
      break;

    case 'crossBorderFilter':
      // Full cross-border filter animation sequence
      await PreviewAnimations.crossBorderFilterSequence(previewCode, ctx);
      break;

    case 'highlightCountries':
      // Flash highlight on marked cells
      await PreviewAnimations.flashHighlight(previewCode, '.csv-mark', ctx);
      break;

    case 'scrollOutExcluded':
      // Scroll excluded rows out
      await PreviewAnimations.scrollLinesOut(contentEl, 'up', ctx);
      break;

    case 'staggerLines': {
      // Show lines appearing one by one
      const content = step.preview.type === 'text'
        ? fillTemplate(clampPreviewText(step.preview.value))
        : contentEl.textContent;
      await PreviewAnimations.staggeredLinesIn(contentEl, content, ctx, {
        delay: PreviewAnimations.config.lineDelay,
        preserveFormatting: false,
      });
      break;
    }

    case 'scrollInFromBelow':
      // Scroll content in from below
      await PreviewAnimations.scrollLinesIn(contentEl, 'below', ctx);
      break;

    case 'errorDetection':
      // Error detection: phase 1 = fade/slide in, phase 2 = scroll + highlight
      await PreviewAnimations.scrollLinesIn(contentEl, 'below', ctx, 520);
      if (ctx.cancelled) return;
      await PreviewAnimations.sleep(180, ctx);
      if (ctx.cancelled) return;
      // Scroll to the first highlighted error column, then flash highlights
      {
        const cols = resolveErrorScrollCols(step, previewCode);
        if (cols) {
          await PreviewAnimations.horizontalScrollToColumns(previewCode, cols, ctx);
        }
      }
      if (ctx.cancelled) return;
      await PreviewAnimations.flashHighlight(previewCode, '.csv-mark', ctx);
      break;

    case 'staggerDiff': {
      // Show diff lines appearing one by one with stagger
      const diffContent = renderDiffSnippet(step.preview);
      await PreviewAnimations.staggeredLinesIn(contentEl, diffContent, ctx, {
        delay: 300,
        preserveFormatting: true,
      });
      break;
    }

    case 'typewriterXml': {
      // XML loads line by line with typewriter effect
      const xmlContent = renderXmlSnippet(step.preview);
      await PreviewAnimations.staggeredLinesIn(contentEl, xmlContent, ctx, {
        delay: 150,
        preserveFormatting: false,
      });
      break;
    }

    case 'scrollOut':
      await PreviewAnimations.scrollLinesOut(contentEl, 'up', ctx);
      break;

    default:
      break;
  }
}

function setActiveStep(id) {
  const step = steps.find((item) => item.id === id);
  if (!step) {
    return;
  }

  const prevStepId = activeStepId;
  const isNewStep = prevStepId !== id;
  const transitionToken = isNewStep ? ++stepChangeToken : stepChangeToken;
  const prevStep = steps.find((item) => item.id === prevStepId);
  const prevIndex = stepIndexMap.get(prevStepId);
  const nextIndex = stepIndexMap.get(id);
  const isJump = prevIndex !== undefined && nextIndex !== undefined && Math.abs(nextIndex - prevIndex) > 1;
  previousStepId = prevStepId;
  activeStepId = id;

  const now = performance.now();
  const isRapid = isNewStep && (now - lastStepChangeAt < 400 || isJump);
  if (isNewStep) {
    lastStepChangeAt = now;
  }

  // Cancel any running animations when switching steps
  PreviewAnimations.cancelAnimation();
  if (isAnimating) {
    isAnimating = false;
  }

  // Check if we need to run exit animation from previous step
  const canAnimate = isNewStep && !isRapid;
  const skipExit = shouldSkipExit(prevStepId, id);
  const needsExitAnim = canAnimate && !skipExit && prevStep && prevStep.animation && prevStep.animation.onExit;
  const forceEnter = shouldForceEnter(step, isRapid);

  if (needsExitAnim) {
    isAnimating = true;

    // Run exit animation on current content, then transition
    runStepAnimation(prevStep, 'onExit').then(() => {
      if (transitionToken !== stepChangeToken) {
        return;
      }
      // Now set new content and run enter animation
      setPreview(step, false);

      if (step.animation && step.animation.onEnter) {
        return runStepAnimation(step, 'onEnter');
      }
    }).finally(() => {
      if (transitionToken === stepChangeToken) {
        isAnimating = false;
      }
    });
  } else if ((canAnimate || forceEnter) && step.animation && step.animation.onEnter) {
    isAnimating = true;

    // Set content first, then animate
    setPreview(step, false);

    // Run enter animation asynchronously
    runStepAnimation(step, 'onEnter').finally(() => {
      if (transitionToken === stepChangeToken) {
        isAnimating = false;
      }
    });
  } else {
    setPreview(step, false);
    applyStepFocus(step);
  }

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

// Initialize tooltips on the preview code element
Tooltips.init();
Tooltips.attachToPreview(previewCode);
