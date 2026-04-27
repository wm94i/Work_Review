<script>
  import { invoke } from '@tauri-apps/api/core';
  import { link } from 'svelte-spa-router';
  import { formatDurationLocalized, locale, t } from '$lib/i18n/index.js';
  import LocalizedDatePicker from '../../lib/components/LocalizedDatePicker.svelte';
  import {
    getFullSummary,
    getMainApps,
    getPrimarySummary,
    getSecondarySummary,
    getSummaryRhythmMeta,
  } from './summaryPresentation.js';

  function getLocalDateString() {
    const now = new Date();
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
  }

  let summaries = [];
  let loading = true;
  let error = null;
  let selectedDate = getLocalDateString();
  let lastLoadedDate = null;
  let expandedHours = new Set();
  $: currentLocale = $locale;
  $: peakDuration = summaries.reduce((max, summary) => Math.max(max, summary.total_duration || 0), 0);

  function toggleExpand(hour) {
    if (expandedHours.has(hour)) {
      expandedHours.delete(hour);
    } else {
      expandedHours.add(hour);
    }
    expandedHours = expandedHours;
  }

  function needsExpand(summary) {
    const full = getFullSummary(summary.summary);
    const primary = getPrimarySummary(summary.summary);
    const secondary = getSecondarySummary(summary.summary);
    const displayed = [primary, secondary].filter(Boolean).join('');
    return full.length > displayed.length + 2;
  }

  function formatHourLabel(hour) {
    return `${String(hour).padStart(2, '0')}:00`;
  }

  function isPeakSummary(summary) {
    return summaries.length > 1 && peakDuration > 0 && summary.total_duration === peakDuration;
  }

  async function loadSummaries() {
    loading = true;
    error = null;
    try {
      summaries = await invoke('get_hourly_summaries', { date: selectedDate });
    } catch (e) {
      error = e.toString();
    } finally {
      loading = false;
    }
  }

  // 仅在日期变化时加载，避免 onMount + 响应式语句双重触发
  $: if (selectedDate && selectedDate !== lastLoadedDate) {
    lastLoadedDate = selectedDate;
    loadSummaries();
  }
</script>

<div class="page-shell summary-page-shell" data-locale={currentLocale}>
  <div class="page-header">
    <div class="page-title-group summary-page-title-group">
      <a href="/timeline" use:link class="summary-back-btn" aria-label={t('timeline.title')}>
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </a>
      <div class="page-title-badge summary-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 7h14M5 12h10M5 17h14" />
          <circle cx="17" cy="12" r="2.3" stroke-width="1.8" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>{t('timelineSummary.title')}</h2>
        <p>{t('timelineSummary.description')}</p>
      </div>
    </div>

    <div class="page-toolbar">
    {#key `timeline-summary-date-${currentLocale}`}
      <LocalizedDatePicker
        bind:value={selectedDate}
        max={getLocalDateString()}
        localeCode={currentLocale}
        triggerClass="page-control-input w-auto"
      />
    {/key}
    </div>
  </div>

  {#if loading}
    <div class="page-card-soft summary-state-card">
      <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-primary-500"></div>
    </div>
  {:else if error}
    <div class="page-card-soft summary-state-card">
      <p class="summary-state-error">{error}</p>
    </div>
  {:else if summaries.length === 0}
    <div class="page-card-soft summary-state-card">
      <span class="summary-state-icon">📊</span>
      <p class="summary-state-copy">{t('timelineSummary.noData')}</p>
    </div>
  {:else}
    <div class="summary-editorial-shell">
      {#each summaries as summary}
        {@const apps = getMainApps(summary.main_apps)}
        {@const peak = isPeakSummary(summary)}
        {@const expanded = expandedHours.has(summary.hour)}
        {@const canExpand = needsExpand(summary)}
        {@const rhythm = getSummaryRhythmMeta(summary.total_duration)}
        <section class={`summary-band ${peak ? 'summary-band-peak' : ''}`}>
          <div class="summary-band-anchor">
            <div class="summary-band-hour">{formatHourLabel(summary.hour)}</div>
            <div class="summary-band-duration">{formatDurationLocalized(summary.total_duration)}</div>
          </div>

          <div class="summary-band-card">
            <div class="summary-band-card-header">
              <p class="summary-primary-copy">
                {#if expanded}
                  {getFullSummary(summary.summary) || t('timelineSummary.noData')}
                {:else}
                  {getPrimarySummary(summary.summary) || t('timelineSummary.noData')}
                {/if}
              </p>
              {#if peak}
                <span class="summary-peak-badge">{t('timelineSummary.peakBadge')}</span>
              {/if}
            </div>

            <div class="summary-meta-row">
              <span class={`summary-rhythm-chip summary-rhythm-chip-${rhythm.tone}`}>
                {t(`timelineSummary.rhythm.${rhythm.tone}`)}
              </span>
              {#if apps.length > 0}
                <span class="summary-app-count">{t('timelineSummary.appsCount', { count: apps.length })}</span>
              {/if}
            </div>

            {#if !expanded}
              {@const secondarySummary = getSecondarySummary(summary.summary)}
              {#if secondarySummary}
                <p class="summary-secondary-copy">{secondarySummary}</p>
              {/if}
            {/if}

            {#if canExpand}
              <button
                type="button"
                class="summary-expand-btn"
                on:click={() => toggleExpand(summary.hour)}
              >
                {expanded ? t('timelineSummary.collapse') : t('timelineSummary.expandFull')}
                <svg class="w-3.5 h-3.5 transition-transform duration-200 {expanded ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                </svg>
              </button>
            {/if}

            {#if apps.length > 0}
              <div class="summary-app-tags">
                {#each apps as app}
                  <span class="summary-app-tag">{app}</span>
                {/each}
              </div>
            {/if}
          </div>
        </section>
      {/each}
    </div>
  {/if}
</div>

<style>
  .summary-page-shell {
    padding-top: 1.5rem;
  }

  .summary-page-title-group {
    gap: 0.9rem;
  }

  .summary-back-btn {
    width: 2.6rem;
    height: 2.6rem;
    border-radius: 999px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: #57534e;
    background: rgba(255, 255, 255, 0.76);
    border: 1px solid rgba(120, 113, 108, 0.12);
    box-shadow: 0 10px 24px rgba(15, 23, 42, 0.06);
    transition:
      transform 180ms ease,
      background-color 180ms ease,
      border-color 180ms ease;
  }

  .summary-back-btn:hover {
    transform: translateY(-1px);
    background: rgba(255, 251, 235, 0.96);
    border-color: rgba(180, 83, 9, 0.18);
  }

  .summary-title-badge {
    color: #a16207;
    box-shadow:
      0 12px 30px rgba(217, 119, 6, 0.16),
      inset 0 1px 0 rgba(255, 255, 255, 0.82);
    background:
      radial-gradient(circle at top left, rgba(255, 251, 235, 0.98), rgba(255, 247, 237, 0.92)),
      linear-gradient(135deg, rgba(254, 243, 199, 0.84), rgba(255, 255, 255, 0.9));
  }

  .summary-state-card {
    min-height: 11rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.85rem;
  }

  .summary-state-icon {
    font-size: 2rem;
  }

  .summary-state-copy {
    margin: 0;
    font-size: 0.92rem;
    color: #78716c;
  }

  .summary-state-error {
    margin: 0;
    color: #dc2626;
    font-size: 0.92rem;
  }

  .summary-editorial-shell {
    --summary-anchor-width: 6.2rem;
    position: relative;
    padding: 1rem 0.25rem 0.5rem;
  }

  .summary-editorial-shell::before {
    content: '';
    position: absolute;
    left: calc(0.25rem + var(--summary-anchor-width) + 0.5rem);
    top: 0.4rem;
    bottom: 0.4rem;
    width: 2px;
    border-radius: 999px;
    background: linear-gradient(180deg, rgba(31, 41, 55, 0.88), rgba(31, 41, 55, 0.08));
    opacity: 0.9;
  }

  .summary-band {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: var(--summary-anchor-width) minmax(0, 1fr);
    gap: 1rem;
    align-items: start;
  }

  .summary-band + .summary-band {
    margin-top: 0.9rem;
  }

  .summary-band-anchor {
    position: relative;
    padding-top: 1rem;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.28rem;
  }

  .summary-band-anchor::after {
    content: '';
    position: absolute;
    top: 1.2rem;
    right: 0.05rem;
    width: 0.8rem;
    height: 0.8rem;
    border-radius: 999px;
    background: #1f2937;
    box-shadow:
      0 0 0 0.32rem rgba(255, 251, 235, 0.96),
      0 0 0 0.4rem rgba(31, 41, 55, 0.08);
  }

  .summary-band-peak .summary-band-anchor::after {
    background: #b45309;
    box-shadow:
      0 0 0 0.32rem rgba(255, 251, 235, 0.96),
      0 0 0 0.5rem rgba(180, 83, 9, 0.12);
  }

  .summary-band-hour {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    font-size: 0.92rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: #44403c;
  }

  .summary-band-duration {
    font-size: 0.78rem;
    color: #a8a29e;
  }

  .summary-band-card {
    border-radius: 1.35rem;
    padding: 1.05rem 1.15rem;
    background:
      radial-gradient(circle at top left, rgba(255, 255, 255, 0.98), rgba(255, 248, 238, 0.92) 52%, rgba(255, 243, 229, 0.88) 100%);
    border: 1px solid rgba(17, 24, 39, 0.08);
    box-shadow:
      0 16px 34px rgba(15, 23, 42, 0.08),
      inset 0 1px 0 rgba(255, 255, 255, 0.84);
  }

  .summary-band-peak .summary-band-card {
    border-color: rgba(180, 83, 9, 0.14);
    box-shadow:
      0 18px 38px rgba(15, 23, 42, 0.1),
      0 0 0 1px rgba(255, 247, 237, 0.78);
  }

  .summary-band-card-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.85rem;
  }

  .summary-primary-copy {
    margin: 0;
    flex: 1;
    color: #1f2937;
    font-size: 0.98rem;
    line-height: 1.7;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .summary-peak-badge {
    flex-shrink: 0;
    padding: 0.38rem 0.68rem;
    border-radius: 999px;
    background: rgba(255, 247, 237, 0.96);
    color: #9a3412;
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.08em;
  }

  .summary-meta-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.55rem;
    margin-top: 0.85rem;
  }

  .summary-rhythm-chip {
    display: inline-flex;
    align-items: center;
    min-height: 1.7rem;
    padding: 0.18rem 0.68rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    line-height: 1;
  }

  .summary-rhythm-chip-deep {
    background: rgba(30, 41, 59, 0.92);
    color: #f8fafc;
  }

  .summary-rhythm-chip-steady {
    background: rgba(224, 231, 255, 0.88);
    color: #4338ca;
  }

  .summary-rhythm-chip-light {
    background: rgba(255, 247, 237, 0.92);
    color: #c2410c;
  }

  .summary-app-count {
    font-size: 0.76rem;
    color: #78716c;
  }

  .summary-secondary-copy {
    margin: 0.72rem 0 0;
    color: #57534e;
    font-size: 0.84rem;
    line-height: 1.7;
  }

  .summary-expand-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    margin-top: 0.65rem;
    padding: 0;
    border: none;
    background: none;
    color: #78716c;
    font-size: 0.76rem;
    font-weight: 500;
    cursor: pointer;
    transition: color 180ms ease;
  }

  .summary-expand-btn:hover {
    color: #44403c;
  }

  :global(.dark) .summary-expand-btn {
    color: #94a3b8;
  }

  :global(.dark) .summary-expand-btn:hover {
    color: #cbd5e1;
  }

  .summary-app-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
    margin-top: 0.9rem;
  }

  .summary-app-tag {
    display: inline-flex;
    align-items: center;
    min-height: 1.8rem;
    padding: 0.22rem 0.72rem;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.82);
    border: 1px solid rgba(120, 113, 108, 0.12);
    color: #57534e;
    font-size: 0.78rem;
    line-height: 1;
  }

  :global(.dark) .summary-back-btn {
    color: #e2e8f0;
    background: rgba(30, 41, 59, 0.82);
    border-color: rgba(148, 163, 184, 0.14);
    box-shadow: 0 12px 28px rgba(2, 6, 23, 0.3);
  }

  :global(.dark) .summary-back-btn:hover {
    background: rgba(51, 65, 85, 0.92);
    border-color: rgba(251, 191, 36, 0.18);
  }

  :global(.dark) .summary-title-badge {
    color: #fbbf24;
    background:
      radial-gradient(circle at top left, rgba(120, 53, 15, 0.28), rgba(30, 41, 59, 0.92)),
      linear-gradient(135deg, rgba(120, 53, 15, 0.22), rgba(15, 23, 42, 0.94));
  }

  :global(.dark) .summary-state-copy {
    color: #94a3b8;
  }

  :global(.dark) .summary-editorial-shell::before {
    background: linear-gradient(180deg, rgba(248, 250, 252, 0.84), rgba(148, 163, 184, 0.08));
  }

  :global(.dark) .summary-band-anchor::after {
    background: #e2e8f0;
    box-shadow:
      0 0 0 0.32rem rgba(15, 23, 42, 0.96),
      0 0 0 0.5rem rgba(148, 163, 184, 0.08);
  }

  :global(.dark) .summary-band-peak .summary-band-anchor::after {
    background: #fbbf24;
    box-shadow:
      0 0 0 0.32rem rgba(15, 23, 42, 0.96),
      0 0 0 0.5rem rgba(245, 158, 11, 0.16);
  }

  :global(.dark) .summary-band-hour {
    color: #f8fafc;
  }

  :global(.dark) .summary-band-duration {
    color: #94a3b8;
  }

  :global(.dark) .summary-band-card {
    background:
      radial-gradient(circle at top left, rgba(71, 85, 105, 0.2), rgba(30, 41, 59, 0.92) 46%, rgba(15, 23, 42, 0.98) 100%);
    border-color: rgba(148, 163, 184, 0.12);
    box-shadow:
      0 18px 38px rgba(2, 6, 23, 0.34),
      inset 0 1px 0 rgba(255, 255, 255, 0.04);
  }

  :global(.dark) .summary-band-peak .summary-band-card {
    border-color: rgba(251, 191, 36, 0.16);
  }

  :global(.dark) .summary-primary-copy {
    color: #f8fafc;
  }

  :global(.dark) .summary-peak-badge {
    background: rgba(120, 53, 15, 0.24);
    color: #fdba74;
  }

  :global(.dark) .summary-rhythm-chip-deep {
    background: rgba(248, 250, 252, 0.16);
    color: #f8fafc;
  }

  :global(.dark) .summary-rhythm-chip-steady {
    background: rgba(79, 70, 229, 0.22);
    color: #c4b5fd;
  }

  :global(.dark) .summary-rhythm-chip-light {
    background: rgba(120, 53, 15, 0.24);
    color: #fdba74;
  }

  :global(.dark) .summary-app-count {
    color: #94a3b8;
  }

  :global(.dark) .summary-secondary-copy {
    color: #cbd5e1;
  }

  :global(.dark) .summary-app-tag {
    background: rgba(15, 23, 42, 0.48);
    border-color: rgba(148, 163, 184, 0.12);
    color: #cbd5e1;
  }

  @media (max-width: 640px) {
    .summary-editorial-shell {
      --summary-anchor-width: 5rem;
      padding-top: 0.5rem;
    }

    .summary-editorial-shell::before {
      left: calc(0.25rem + var(--summary-anchor-width) - 0.72rem);
    }

    .summary-band {
      gap: 0.7rem;
    }

    .summary-band-anchor {
      padding-top: 0.85rem;
    }

    .summary-band-anchor::after {
      width: 0.68rem;
      height: 0.68rem;
    }

    .summary-band-card-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .summary-primary-copy {
      font-size: 0.92rem;
    }
  }
</style>
