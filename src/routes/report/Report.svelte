<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import { open } from '@tauri-apps/plugin-shell';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import { showToast } from '../../lib/stores/toast.js';
  import { cache } from '../../lib/stores/cache.js';
  import { formatLocalizedDate, formatLocalizedTime, locale, t } from '$lib/i18n/index.js';
  import { shouldShowPromptAppliedToast } from './reportPromptFeedback.js';
  import { resolveReportMeta } from './reportMeta.js';
  import LocalizedDatePicker from '../../lib/components/LocalizedDatePicker.svelte';

  function getLocalDateString() {
    const now = new Date();
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
  }

  function getYesterdayDateString() {
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);
    return `${yesterday.getFullYear()}-${String(yesterday.getMonth() + 1).padStart(2, '0')}-${String(yesterday.getDate()).padStart(2, '0')}`;
  }

  let report = null;
  let loading = false;
  let generating = false;
  let error = null;
  let selectedDate = getLocalDateString();
  let isYesterdayReport = false; // 标记是否显示的是昨日日报
  let config = null;
  let lastLoadedDate = '';
  let reportRequestId = 0;
  let exportInProgress = false;
  let promptSaving = false;
  let cacheData = null;
  cache.subscribe(v => {
    cacheData = v;
    // 首次或缓存有值时，立即从缓存恢复配置（避免页面切换闪烁）
    if (!config && v?.config) {
      config = v.config;
    }
  });
  $: generating = cacheData?.reportGenerating ?? false;
  $: generating = cacheData?.reportGenerating ?? false;
  $: currentLocale = $locale;
  $: currentReportCacheKey = `${selectedDate}:${currentLocale}`;

  // 获取 AI 模式显示名称
  function getAiModeName(mode) {
    const normalizedMode = (mode || '').toString().trim().toLowerCase();
    const modeNames = {
      'local': t('report.modeNames.local'),
      'summary': t('report.modeNames.summary'),
      'cloud': t('report.modeNames.cloud')
    };
    return modeNames[normalizedMode] || mode || t('report.modeNames.unknown');
  }

  function getFallbackReasonText(meta) {
    return meta?.fallbackReason || t('report.savedReportNotAi');
  }

  async function loadConfig() {
    try {
      const cfg = await invoke('get_config');
      cache.setConfig(cfg);
    } catch (e) {
      console.error('加载配置失败:', e);
    }
  }

  async function loadReport(previousReport = null) {
    const requestId = ++reportRequestId;

    // 乐观更新：先显示缓存数据
    let cacheData;
    const unsubscribe = cache.subscribe(c => { cacheData = c; });
    unsubscribe();
    
    if (cacheData.reports[currentReportCacheKey]?.data) {
      report = cacheData.reports[currentReportCacheKey].data;
      isYesterdayReport = false;
      loading = false;
      
      // 缓存有效则直接返回
      if (cache.isValid(cacheData.reports[currentReportCacheKey], 'reports')) {
        return;
      }
      
      // 后台静默刷新
      try {
        const savedReport = await invoke('get_saved_report', { date: selectedDate, locale: currentLocale });
        if (requestId !== reportRequestId) return;
        if (savedReport) {
          report = savedReport;
          cache.setReport(currentReportCacheKey, savedReport);
        }
      } catch (e) {
        console.warn('后台刷新日报失败:', e);
      }
    } else {
      // 首次加载
      loading = true;
      error = null;
      try {
        const savedReport = await invoke('get_saved_report', { date: selectedDate, locale: currentLocale });
        if (requestId !== reportRequestId) return;
        if (savedReport) {
          report = savedReport;
          isYesterdayReport = false;
          cache.setReport(currentReportCacheKey, savedReport);
        } else {
          if (!savedReport && previousReport?.date === selectedDate && previousReport?.content) {
            generating = true;
            await invoke('generate_report', { date: selectedDate, force: false, locale: currentLocale });
            const localizedReport = await invoke('get_saved_report', { date: selectedDate, locale: currentLocale });

            if (localizedReport) {
              report = localizedReport;
              isYesterdayReport = false;
              cache.setReport(currentReportCacheKey, localizedReport);
              return;
            }
          }

          // 如果选择今天且今天无日报，尝试加载昨日日报
          if (selectedDate === getLocalDateString()) {
            const yesterday = getYesterdayDateString();
            const yesterdayReport = await invoke('get_saved_report', { date: yesterday, locale: currentLocale });
            if (yesterdayReport) {
              report = yesterdayReport;
              isYesterdayReport = true;
            } else {
              report = null;
              isYesterdayReport = false;
            }
          } else {
             report = null;
             isYesterdayReport = false;
          }
        }
      } catch (e) {
        error = e.toString();
      } finally {
        generating = false;
        loading = false;
      }
    }
  }

  function selectDate(date) {
    if (!date || date === selectedDate) return;
    selectedDate = date;
  }

  async function generateReport(force = true) {
    cache.setReportGenerating(true);
    error = null;
    try {
      if (config?.ai_mode === 'summary') {
        await persistReportPrompt();
      }
      await invoke('generate_report', { date: selectedDate, force, locale: currentLocale });
      const savedReport = await invoke('get_saved_report', { date: selectedDate, locale: currentLocale });
      report = savedReport || { date: selectedDate, content: '', created_at: Date.now() / 1000 };
      isYesterdayReport = false;
      cache.setReport(currentReportCacheKey, report);

      if (
        shouldShowPromptAppliedToast({
          configAiMode: config?.ai_mode,
          customPrompt: config?.daily_report_custom_prompt,
          reportAiMode: savedReport?.ai_mode,
        })
      ) {
        showToast(t('report.promptApplied'), 'success');
      }
    } catch (e) {
      error = e.toString();
    } finally {
      cache.setReportGenerating(false);
    }
  }

  async function persistReportPrompt() {
    if (!config || config.ai_mode !== 'summary' || promptSaving) {
      return;
    }

    promptSaving = true;
    try {
      config.daily_report_custom_prompt = (config.daily_report_custom_prompt || '').trim();
      await invoke('save_config', { config });
    } finally {
      promptSaving = false;
    }
  }

  async function exportReportMarkdown() {
    if (!report) return;

    exportInProgress = true;
    try {
      let exportDir = config?.daily_report_export_dir || null;
      if (!exportDir) {
        const selected = await openDialog({
          directory: true,
          multiple: false,
        });

        if (!selected || Array.isArray(selected)) {
          return;
        }

        exportDir = selected;
      }

      const exportPath = await invoke('export_report_markdown', {
        date: report.date || selectedDate,
        content: report.content,
        exportDir,
      });
      showToast(t('report.exportSuccess', { path: exportPath }), 'success');
    } catch (e) {
      showToast(t('report.exportFailed', { error: e }), 'error');
    } finally {
      exportInProgress = false;
    }
  }

  function renderMarkdown(content) {
    const rawHtml = marked(content);
    return DOMPurify.sanitize(rawHtml);
  }

  async function handleReportLinkClick(event) {
    const link = event.target.closest('a[href]');
    if (!link) return;

    const href = link.getAttribute('href');
    if (!href || href.startsWith('#')) return;

    event.preventDefault();
    try {
      await open(href);
    } catch (e) {
      console.error('打开日报链接失败:', e);
    }
  }

  function interceptReportLinks(node) {
    const listener = (event) => {
      handleReportLinkClick(event);
    };

    node.addEventListener('click', listener);

    return {
      destroy() {
        node.removeEventListener('click', listener);
      }
    };
  }

  // 结构化编辑：将 markdown 按 ## 标题拆分为段落
  let editingSection = -1; // 当前正在编辑的段落索引
  let editingContent = ''; // 编辑中的内容

  function parseSections(content) {
    if (!content) return [];
    const lines = content.split('\n');
    const sections = [];
    let currentTitle = '';
    let currentLines = [];

    for (const line of lines) {
      // <details> 块作为独立段落，与上方内容分离
      if (line.startsWith('<details>') || line.startsWith('## ')) {
        if (currentTitle || currentLines.length) {
          sections.push({ title: currentTitle, body: currentLines.join('\n') });
        }
        currentTitle = line.startsWith('## ') ? line : '';
        currentLines = line.startsWith('<details>') ? [line] : [];
      } else {
        currentLines.push(line);
      }
    }
    if (currentTitle || currentLines.length) {
      sections.push({ title: currentTitle, body: currentLines.join('\n') });
    }

    return sections;
  }

  function startEditSection(sections, index) {
    editingSection = index;
    const section = sections[index];
    editingContent = section.title ? section.title + '\n' + section.body : section.body;
  }

  function cancelEditSection() {
    editingSection = -1;
    editingContent = '';
  }

  async function saveEditSection(sections, index) {
    const newContent = editingContent.trim();
    const newSections = [...sections];
    const parsed = parseSections(newContent || '');
    if (parsed.length > 0) {
      newSections[index] = parsed[0];
      // If user added more ## headers, merge them in
      if (parsed.length > 1) {
        newSections.splice(index + 1, 0, ...parsed.slice(1));
      }
    }

    const fullContent = newSections.map(s => {
      if (s.title && s.body) return s.title + '\n' + s.body;
      return s.title || s.body;
    }).join('\n');

    try {
      await invoke('update_report_content', { date: selectedDate, locale: currentLocale, content: fullContent });
      report = { ...report, content: fullContent };
      cache.setReport(currentReportCacheKey, report);
      editingSection = -1;
      editingContent = '';
    } catch (e) {
      showToast(t('report.editSectionFailed') + ': ' + e, 'error');
    }
  }

  function formatReportDate(dateStr) {
    const date = new Date(dateStr);
    return formatLocalizedDate(date, { year: 'numeric', month: 'long', day: 'numeric', weekday: 'long' });
  }

  $: if (currentReportCacheKey && currentReportCacheKey !== lastLoadedDate) {
    const previousReport = report;
    lastLoadedDate = currentReportCacheKey;
    report = null;
    editingSection = -1;
    isYesterdayReport = false;
    loadReport(previousReport);
  }

  $: reportSections = parseSections(report?.content || '');

  $: reportMeta = resolveReportMeta(report, config);

  onMount(() => {
    loadConfig();
  });
</script>

<div class="page-shell report-editorial-shell" data-locale={currentLocale}>
  <!-- 页面标题 -->
  <div class="report-hero">
    <div class="report-hero-main">
      <div class="page-title-group report-hero-copy">
      <div class="page-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M8 7h8M8 12h8M8 17h5" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M7 3h7l5 5v10a3 3 0 01-3 3H7a3 3 0 01-3-3V6a3 3 0 013-3Z" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>
          {selectedDate === getLocalDateString() ? t('report.todayReport') : t('report.historyReport')}
        </h2>
        <div class="report-hero-meta">
          <div class="report-hero-date-row">
            <span class="report-hero-date">{formatReportDate(selectedDate)}</span>
            {#if config || report}
              <span class="report-hero-mode-chip">{getAiModeName(reportMeta.reportMode)}</span>
            {/if}
          </div>
          {#if config || report}
            {#if reportMeta.showUsageMismatchNotice}
              <p class="report-hero-mode-note">{t('report.aiNotAppliedPrefix')}{getFallbackReasonText(reportMeta)}</p>
            {/if}
          {/if}
        </div>
      </div>
    </div>
      <div class="report-hero-actions">
      <div class="page-toolbar-end">
        <button
          class="page-control-btn {selectedDate === getLocalDateString() ? 'page-control-btn-active' : ''}"
          on:click={() => selectDate(getLocalDateString())}
        >
          {t('report.today')}
        </button>
        <button
          class="page-control-btn {selectedDate === getYesterdayDateString() ? 'page-control-btn-active' : ''}"
          on:click={() => selectDate(getYesterdayDateString())}
        >
          {t('report.yesterday')}
        </button>
        {#key `report-date-${currentLocale}`}
          <LocalizedDatePicker
            bind:value={selectedDate}
            max={getLocalDateString()}
            localeCode={currentLocale}
            triggerClass="page-control-input w-auto"
          />
        {/key}
      </div>
      <div class="flex flex-wrap justify-end gap-2">
        {#if report}
          <button
            class="page-action-secondary min-h-10 px-4 py-2"
            on:click={exportReportMarkdown}
            disabled={exportInProgress}
            title={config?.daily_report_export_dir ? '' : t('report.exportWithoutDefaultDir')}
          >
            {#if exportInProgress}
              <div class="animate-spin rounded-full h-4 w-4 border-2 border-current border-t-transparent"></div>
              {t('report.exporting')}
            {:else}
              {t('report.exportMarkdown')}
            {/if}
          </button>
          <button
            class="page-action-warn"
            on:click={() => generateReport(true)}
            disabled={generating}
          >
            {#if generating}
              <div class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
              {t('report.generating')}
            {:else}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              {t('report.regenerate')}
            {/if}
          </button>
        {/if}
      </div>
    </div>
    </div>
  </div>

  <div class="report-editorial-stack">
  {#if config && config.ai_mode === 'summary'}
    <div class="page-card report-sheet report-sheet-controls">
      <label for="daily-report-custom-prompt" class="settings-label mb-1.5">{t('report.promptLabel')}</label>
      <textarea
        id="daily-report-custom-prompt"
        bind:value={config.daily_report_custom_prompt}
        on:change={persistReportPrompt}
        rows="3"
        class="control-input resize-y min-h-[80px]"
        placeholder={t('report.promptPlaceholder')}
      ></textarea>
    </div>
  {/if}

  <!-- 日报内容 -->
  {#if loading}
    <div class="empty-state-lg">
      <div class="empty-state-icon">
        <div class="animate-spin rounded-full h-8 w-8 border-2 border-indigo-500 border-t-transparent"></div>
      </div>
      <h3 class="empty-state-title">{t('report.loadingTitle')}</h3>
      <p class="empty-state-copy mt-1">{t('report.loadingCopy')}</p>
    </div>
  {:else if error}
    <div class="page-banner-error">
      <div>
        <div class="flex items-center gap-3 text-red-500 mb-2">
        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <span class="font-medium">{t('report.generateFailed')}</span>
      </div>
      <p class="text-sm">{error}</p>
      </div>
      <button class="page-action-brand" on:click={() => generateReport(true)}>{t('common.retry')}</button>
    </div>
  {:else if report}
    <!-- 昨日日报提示 -->
    {#if isYesterdayReport}
      <div class="page-banner-warning report-fallback-banner mb-4">
        <div class="report-fallback-copy">
          <div class="flex items-center gap-2 text-sm">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          {t('report.showingYesterday', { date: formatReportDate(report.date) })}
          </div>
        </div>
        <div class="report-fallback-action">
          <button
            class="page-action-warn report-fallback-button min-h-9 px-3 text-xs rounded-lg shadow-none"
            on:click={() => generateReport(false)}
            disabled={generating}
          >
            {#if generating}
              <div class="inline-flex items-center gap-2">
                <div class="animate-spin rounded-full h-3 w-3 border-2 border-white border-t-transparent"></div>
                <span>{t('report.generating')}</span>
              </div>
            {:else}
              ✨ {t('report.generatingToday')}
            {/if}
          </button>
        </div>
      </div>
    {/if}
    <div class="page-card report-sheet report-article-card">
      <div class="report-sheet-content">
        <div class="report-sheet-meta text-xs text-slate-400 mb-4 flex items-center gap-2">
          <div class="w-1.5 h-1.5 rounded-full {isYesterdayReport ? 'bg-amber-500' : 'bg-emerald-500'}"></div>
          {isYesterdayReport ? t('report.yesterdayPrefix') : ''}{t('report.generatedAt', { time: formatLocalizedDate(new Date(report.created_at * 1000), { year: 'numeric', month: '2-digit', day: '2-digit' }) + ' ' + formatLocalizedTime(new Date(report.created_at * 1000), { hour: '2-digit', minute: '2-digit', second: '2-digit' }) })}
        </div>
        <div class="markdown-body report-sheet-body prose prose-slate dark:prose-invert max-w-none">
          {#each reportSections as section, i}
            <div class="report-section">
              <div class="report-section-header">
                <div
                  use:interceptReportLinks
                  class="report-section-content"
                >
                  {@html renderMarkdown(section.title + '\n' + section.body)}
                </div>
                <button
                  class="report-section-edit-btn"
                  on:click={() => startEditSection(reportSections, i)}
                  title={t('report.editSection')}
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                  </svg>
                </button>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>
    {:else}
    <div class="empty-state-lg">
      <div class="empty-state-icon !w-16 !h-16 !mb-5 bg-amber-50 dark:bg-amber-950/30">
        <span class="text-3xl">📝</span>
      </div>
      <h3 class="empty-state-title">
        {selectedDate === getLocalDateString() ? t('report.noReportToday') : t('report.noReportForDate', { date: selectedDate })}
      </h3>
      <p class="empty-state-copy mb-5">
        {t('report.aiWillGenerate')}
      </p>
      <button
        class="page-action-warn min-h-11 px-6 py-3"
        on:click={() => generateReport(false)}
        disabled={generating}
      >
        {#if generating}
          <div class="inline-flex items-center gap-2">
            <div class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
            {t('report.generating')}
          </div>
        {:else}
          ✨ {selectedDate === getLocalDateString() ? t('report.generatingToday') : t('report.generatingSelected')}
        {/if}
      </button>
    </div>
  {/if}
</div>
</div>

<!-- 段落编辑弹窗 -->
{#if editingSection >= 0}
  <div class="modal-overlay" on:click|self={cancelEditSection}>
    <div class="modal-panel" on:click|stopPropagation>
      <div class="modal-header">
        <h3 class="modal-title">{t('report.editSection')}</h3>
        <button class="modal-close" on:click={cancelEditSection}>
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
      <div class="modal-body">
        <textarea
          class="report-edit-textarea"
          bind:value={editingContent}
        ></textarea>
      </div>
      <div class="modal-footer">
        <button class="page-control-btn" on:click={cancelEditSection}>
          {t('report.cancelEdit')}
        </button>
        <button
          class="page-action-brand"
          on:click={() => saveEditSection(reportSections, editingSection)}
        >
          {t('report.saveSection')}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- 表格 / 标题 / 列表等 markdown 样式已统一放到 app.css .markdown-body -->
