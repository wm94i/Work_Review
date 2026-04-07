<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import StatsCard from '../lib/components/StatsCard.svelte';
  import AppUsageChart from '../lib/components/AppUsageChart.svelte';
  import ActivityHourlyChart from '../lib/components/ActivityHourlyChart.svelte';
  import { cache } from '../lib/stores/cache.js';
  import { confirm } from '../lib/stores/confirm.js';
  import { showToast } from '../lib/stores/toast.js';
  import { appIconStore, getIconCacheKey, preloadAppIcons } from '../lib/stores/iconCache.js';
  import {
    formatDurationLocalized,
    formatLocalizedDate,
    formatLocalizedTime,
    locale,
    t,
    translateSemanticCategoryLabel,
  } from '$lib/i18n/index.js';
  import { resolveAppIconSrc } from '../lib/utils/appVisuals.js';
  import { formatBrowserUrlForDisplay } from '../lib/utils/browserUrl.js';

  function getLocalDateString() {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, '0');
    const day = String(now.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  function parseDateString(dateValue) {
    return new Date(`${dateValue}T12:00:00`);
  }

  function getDateRangeLabel(dateFrom, dateTo) {
    if (!dateFrom && !dateTo) {
      return '';
    }
    if (dateFrom && !dateTo) {
      return formatLocalizedDate(parseDateString(dateFrom), {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        weekday: 'short',
      });
    }
    if (!dateFrom && dateTo) {
      return formatLocalizedDate(parseDateString(dateTo), {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        weekday: 'short',
      });
    }
    if (dateFrom === dateTo) {
      return formatLocalizedDate(parseDateString(dateFrom), {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        weekday: 'short',
      });
    }
    return `${formatLocalizedDate(parseDateString(dateFrom), { year: 'numeric', month: 'short', day: 'numeric' })} - ${formatLocalizedDate(parseDateString(dateTo), { year: 'numeric', month: 'short', day: 'numeric' })}`;
  }

  function getWeekRangeLabel(dateValue) {
    const anchor = parseDateString(dateValue);
    const monday = new Date(anchor);
    monday.setDate(anchor.getDate() - ((anchor.getDay() + 6) % 7));
    return `${formatLocalizedDate(monday, { year: 'numeric', month: 'short', day: 'numeric' })} - ${formatLocalizedDate(anchor, { year: 'numeric', month: 'short', day: 'numeric' })}`;
  }

  function formatOverviewDateInput(dateValue) {
    return dateValue ? dateValue.replace(/-/g, '/') : '';
  }

  function parseOverviewDateInput(value) {
    const normalized = value.trim().replace(/[.]/g, '-').replace(/[\/]/g, '-');
    if (!/^\d{4}-\d{2}-\d{2}$/.test(normalized)) {
      return null;
    }

    const parsed = parseDateString(normalized);
    return Number.isNaN(parsed.getTime()) ? null : normalized;
  }

  function formatIsoDate(date) {
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
  }

  function shiftIsoDate(dateValue, offsetDays) {
    const next = parseDateString(dateValue);
    next.setDate(next.getDate() + offsetDays);
    return formatIsoDate(next);
  }

  const APP_USAGE_VIEW_MODE_KEY = 'overview.appUsage.viewMode';
  const HOURLY_ACTIVITY_VIEW_MODE_KEY = 'overview.hourlyActivity.viewMode';

  let stats = null;
  let loading = true;
  let error = null;
  let unlisten = null;
  let currentTime = new Date();
  let overviewMode = 'today';
  let selectedDateFrom = getLocalDateString();
  let selectedDateTo = getLocalDateString();
  let clockInterval;
  let refreshInterval;
  let handleActivityAdded;
  let handleVisibilityChange;
  let overviewRefreshPromise = null;
  let overviewRequestId = 0;
  let lastCheckDate = currentTime.getDate();
  let appUsageViewMode = 'row';
  let hourlyActivityViewMode = 'column';
  let overviewViewModeReady = false;
  let overviewDateInputFrom = formatOverviewDateInput(selectedDateFrom);
  let overviewDateInputTo = formatOverviewDateInput(selectedDateTo);
  let editingOverviewDateFrom = false;
  let editingOverviewDateTo = false;
  
  let expandedDomains = new Set();
  let editingDomainKey = null;
  let editingSemanticCategory = '';
  let savingDomainKey = null;
  const semanticCategoryOptions = [
    '编码开发',
    '内容撰写',
    '资料阅读',
    '资料调研',
    '任务规划',
    '设计创作',
    'AI 协作',
    '即时聊天',
    '会议沟通',
    '视频内容',
    '音乐音频',
    '休息娱乐',
    '未知活动',
  ];
  
  // 浏览器统计弹窗
  let selectedBrowser = null;
  $: currentLocale = $locale;
  $: isSingleSelectedDate = selectedDateFrom === selectedDateTo;
  $: overviewSubtitle = overviewMode === 'date'
    ? getDateRangeLabel(selectedDateFrom, selectedDateTo)
    : overviewMode === 'week'
      ? `${t('overview.modeWeek')} · ${getWeekRangeLabel(getLocalDateString())}`
      : formatLocalizedDate(new Date(), { year: 'numeric', month: 'long', day: 'numeric', weekday: 'short' });
  $: overviewStatusLabel = overviewMode === 'today' ? t('overview.live') : t(`overview.${overviewMode === 'date' ? 'modeDate' : 'modeWeek'}`);
  $: overviewIsLive = overviewMode !== 'date';
  $: overviewTotalActivityTitle = overviewMode === 'week'
    ? t('overview.totalActivityWeek')
    : overviewMode === 'date'
      ? t(isSingleSelectedDate ? 'overview.totalActivityDate' : 'overview.totalActivityRange')
      : t('overview.totalActivityToday');
  $: overviewWorkDurationTitle = overviewMode === 'week'
    ? t('overview.workDurationWeek')
    : overviewMode === 'date'
      ? t(isSingleSelectedDate ? 'overview.workDurationDate' : 'overview.workDurationRange')
      : t('overview.workDurationToday');
  $: appUsageViewModeLabel = appUsageViewMode === 'column' ? t('overview.appUsageColumn') : t('overview.appUsageBar');
  $: hourlyActivityViewModeLabel = hourlyActivityViewMode === 'row' ? t('overview.hourlyActivityBar') : t('overview.hourlyActivityColumn');
  $: hourlyChartPeakHourLabel = overviewMode === 'week'
    ? t('hourlyChart.peakHourRange')
    : overviewMode === 'date'
      ? t(isSingleSelectedDate ? 'hourlyChart.peakHour' : 'hourlyChart.peakHourRange')
      : t('hourlyChart.peakHour');
  $: hourlyChartPeakDurationLabel = overviewMode === 'week'
    ? t('hourlyChart.peakDurationRange')
    : overviewMode === 'date'
      ? t(isSingleSelectedDate ? 'hourlyChart.peakDuration' : 'hourlyChart.peakDurationRange')
      : t('hourlyChart.peakDuration');
  $: hourlyChartDistributionTitle = overviewMode === 'week'
    ? t('hourlyChart.distributionTitleWeek')
    : overviewMode === 'date'
      ? t(isSingleSelectedDate ? 'hourlyChart.distributionTitleDate' : 'hourlyChart.distributionTitleRange')
      : t('hourlyChart.distributionTitleToday');
  $: hourlyChartDistributionSubtitleKey = overviewMode === 'week'
    ? 'hourlyChart.distributionSubtitleRange'
    : overviewMode === 'date'
      ? (isSingleSelectedDate ? 'hourlyChart.distributionSubtitle' : 'hourlyChart.distributionSubtitleRange')
      : 'hourlyChart.distributionSubtitle';
  $: overviewNoWebsiteVisitsText = overviewMode === 'week'
    ? t('overview.noWebsiteVisitsWeek')
    : overviewMode === 'date'
      ? t(isSingleSelectedDate ? 'overview.noWebsiteVisitsDate' : 'overview.noWebsiteVisitsRange')
      : t('overview.noWebsiteVisitsToday');
  $: overviewNoAppStatsText = overviewMode === 'week'
    ? t('overview.noAppStatsWeek')
    : overviewMode === 'date'
      ? t(isSingleSelectedDate ? 'overview.noAppStatsDate' : 'overview.noAppStatsRange')
      : t('overview.noAppStatsToday');
  $: if (!editingOverviewDateFrom && overviewDateInputFrom !== formatOverviewDateInput(selectedDateFrom)) {
    overviewDateInputFrom = formatOverviewDateInput(selectedDateFrom);
  }
  $: if (!editingOverviewDateTo && overviewDateInputTo !== formatOverviewDateInput(selectedDateTo)) {
    overviewDateInputTo = formatOverviewDateInput(selectedDateTo);
  }
  
  // 订阅全局图标缓存 store
  let appIcons = {};
  const unsubIcons = appIconStore.subscribe(v => appIcons = v);

  function readStoredOverviewViewMode(key, fallback) {
    try {
      const value = window.localStorage.getItem(key);
      return value || fallback;
    } catch {
      return fallback;
    }
  }

  function persistOverviewViewMode(key, value) {
    try {
      window.localStorage.setItem(key, value);
    } catch {
      // ignore persistence errors
    }
  }

  // 响应式图标加载：stats 变化时自动触发
  $: if (stats) {
    if (stats.browser_usage?.length) {
      preloadAppIcons(stats.browser_usage.map(b => ({
        appName: b.browser_name,
        executablePath: b.executable_path,
      })), invoke, { priority: true });
    }

    if (stats.app_usage?.length) {
      preloadAppIcons(stats.app_usage.slice(0, 10).map(a => ({
        appName: a.app_name,
        executablePath: a.executable_path,
      })), invoke);
    }
  }

  function formatDuration(seconds) {
    return formatDurationLocalized(seconds);
  }

  function getAppIconSrc(appName, executablePath = null) {
    return resolveAppIconSrc(
      appName,
      appIcons[getIconCacheKey({ appName, executablePath })]
    );
  }

  function getDomainSemanticLabel(domain) {
    return domain?.semantic_category?.trim()
      ? translateSemanticCategoryLabel(domain.semantic_category.trim())
      : t('overview.autoDetected');
  }

  function startDomainSemanticEdit(domain) {
    editingDomainKey = domain.domain;
    editingSemanticCategory = domain.semantic_category?.trim() || '';
  }

  function getSemanticCategoryOptions() {
    if (
      editingSemanticCategory &&
      !semanticCategoryOptions.includes(editingSemanticCategory)
    ) {
      return [editingSemanticCategory, ...semanticCategoryOptions];
    }
    return semanticCategoryOptions;
  }

  function cancelDomainSemanticEdit() {
    editingDomainKey = null;
    editingSemanticCategory = '';
    savingDomainKey = null;
  }

  function findBrowserUsage(browserName, executablePath = null) {
    return stats?.browser_usage?.find((browser) =>
      browser.browser_name === browserName && browser.executable_path === executablePath
    ) || stats?.browser_usage?.find((browser) => browser.browser_name === browserName) || null;
  }

  async function refreshOverviewSelection(browserName, executablePath = null) {
    await loadStats(true);
    selectedBrowser = findBrowserUsage(browserName, executablePath);
  }

  function shouldUseOverviewCache() {
    return overviewMode === 'today';
  }

  function shouldAutoRefreshOverview() {
    return overviewMode !== 'date';
  }

  function setOverviewMode(mode) {
    if (overviewMode === mode) {
      return;
    }
    overviewMode = mode;
    if (mode === 'date') {
      selectedDateFrom = getLocalDateString();
      selectedDateTo = getLocalDateString();
    }
    selectedBrowser = null;
    cancelDomainSemanticEdit();
    loadStats(true);
  }

  function normalizeSelectedDateRange() {
    if (selectedDateTo < selectedDateFrom) {
      selectedDateTo = selectedDateFrom;
    }
  }

  function handleOverviewDateChange() {
    normalizeSelectedDateRange();
    overviewDateInputFrom = formatOverviewDateInput(selectedDateFrom);
    overviewDateInputTo = formatOverviewDateInput(selectedDateTo);
    selectedBrowser = null;
    cancelDomainSemanticEdit();
    loadStats(true);
  }

  function commitOverviewDateInput(field) {
    const nextValue = field === 'start' ? overviewDateInputFrom : overviewDateInputTo;
    const parsed = parseOverviewDateInput(nextValue);

    if (!parsed) {
      overviewDateInputFrom = formatOverviewDateInput(selectedDateFrom);
      overviewDateInputTo = formatOverviewDateInput(selectedDateTo);
      return;
    }

    if (field === 'start') {
      selectedDateFrom = parsed;
      editingOverviewDateFrom = false;
    } else {
      selectedDateTo = parsed;
      editingOverviewDateTo = false;
    }

    handleOverviewDateChange();
  }

  function stepOverviewDateBoundary(field, offsetDays) {
    const today = getLocalDateString();

    if (field === 'start') {
      const nextStart = shiftIsoDate(selectedDateFrom, offsetDays);
      selectedDateFrom = nextStart > selectedDateTo ? selectedDateTo : nextStart;
    } else {
      const nextEnd = shiftIsoDate(selectedDateTo, offsetDays);
      const clampedEnd = nextEnd > today ? today : nextEnd;
      selectedDateTo = clampedEnd < selectedDateFrom ? selectedDateFrom : clampedEnd;
    }

    handleOverviewDateChange();
  }

  async function saveDomainSemanticRule(domain) {
    const nextCategory = editingSemanticCategory.trim();
    if (!domain || !nextCategory || savingDomainKey === domain.domain) return;
    if ((domain.semantic_category?.trim() || '') === nextCategory) {
      cancelDomainSemanticEdit();
      return;
    }

    const confirmed = await confirm({
      title: t('overview.changeDomainCategoryTitle'),
      message: t('overview.changeDomainCategoryMessage', {
        domain: domain.domain,
        category: translateSemanticCategoryLabel(nextCategory),
      }),
      confirmText: t('overview.confirmChange'),
      cancelText: t('overview.cancel'),
      tone: 'warning',
    });
    if (!confirmed) return;

    savingDomainKey = domain.domain;
    const browserName = selectedBrowser?.browser_name;
    const executablePath = selectedBrowser?.executable_path || null;

    try {
      const updatedCount = await invoke('set_domain_semantic_rule', {
        domain: domain.domain,
        semanticCategory: nextCategory,
        syncHistory: true,
      });

      await refreshOverviewSelection(browserName, executablePath);
      cancelDomainSemanticEdit();
      showToast(
        t('overview.domainSemanticUpdated', {
          domain: domain.domain,
          category: translateSemanticCategoryLabel(nextCategory),
          count: updatedCount,
        }),
        'success'
      );
    } catch (e) {
      console.error('修改网站语义分类失败:', e);
      showToast(
        t('overview.domainSemanticUpdateFailed', {
          domain: domain.domain,
          error: e,
        }),
        'error'
      );
      savingDomainKey = null;
    }
  }

  async function refreshOverviewStats({ silent = false } = {}) {
    if (overviewRefreshPromise) {
      return overviewRefreshPromise;
    }

    const requestId = ++overviewRequestId;
    overviewRefreshPromise = invoke('get_overview_stats', {
      mode: overviewMode,
      dateFrom: overviewMode === 'date' ? selectedDateFrom : undefined,
      dateTo: overviewMode === 'date' ? selectedDateTo : undefined,
    })
      .then((newStats) => {
        if (requestId !== overviewRequestId) {
          return;
        }
        stats = newStats;
        if (shouldUseOverviewCache()) {
          cache.setOverview(newStats);
        }
        error = null;
      })
      .catch((e) => {
        if (silent) {
          console.warn('后台刷新失败:', e);
          return;
        }
        error = e.toString();
      })
      .finally(() => {
        overviewRefreshPromise = null;
        loading = false;
      });

    return overviewRefreshPromise;
  }

  async function loadStats(forceRefresh = false) {
    if (!shouldUseOverviewCache()) {
      stats = null;
      loading = true;
      error = null;
      await refreshOverviewStats();
      return;
    }

    // 乐观更新策略：先显示缓存数据，后台刷新后再更新
    let cacheData;
    const unsubscribe = cache.subscribe(c => { cacheData = c; });
    unsubscribe();
    
    // 如果有缓存数据，立即显示（不显示 loading）
    if (cacheData.overview.data) {
      stats = cacheData.overview.data;
      loading = false;
      
      // 如果缓存有效且非强制刷新，直接返回
      if (!forceRefresh && cache.isValid(cacheData.overview)) {
        return;
      }

      await refreshOverviewStats({ silent: true });
    } else {
      // 首次加载，显示 loading
      loading = true;
      error = null;
      await refreshOverviewStats();
    }
  }

  onMount(async () => {
    appUsageViewMode = readStoredOverviewViewMode(APP_USAGE_VIEW_MODE_KEY, 'row');
    hourlyActivityViewMode = readStoredOverviewViewMode(HOURLY_ACTIVITY_VIEW_MODE_KEY, 'column');
    overviewViewModeReady = true;
    loadStats();
    if (!document.hidden) {
      clockInterval = setInterval(() => { 
        currentTime = new Date();
        if (!shouldAutoRefreshOverview()) {
          return;
        }
        // 跨天检测
        const newDate = currentTime.getDate();
        if (newDate !== lastCheckDate) {
          lastCheckDate = newDate;
          loadStats(true);
        }
      }, 1000);
      refreshInterval = setInterval(() => {
        if (shouldAutoRefreshOverview()) {
          loadStats();
        }
      }, 30000);
    }

    handleVisibilityChange = () => {
      if (document.hidden) {
        clearInterval(clockInterval);
        clearInterval(refreshInterval);
        clockInterval = null;
        refreshInterval = null;
      } else {
        currentTime = new Date();
        lastCheckDate = currentTime.getDate();
        clockInterval = setInterval(() => {
          currentTime = new Date();
          if (!shouldAutoRefreshOverview()) {
            return;
          }
          const newDate = currentTime.getDate();
          if (newDate !== lastCheckDate) {
            lastCheckDate = newDate;
            loadStats(true);
          }
        }, 1000);
        refreshInterval = setInterval(() => {
          if (shouldAutoRefreshOverview()) {
            loadStats();
          }
        }, 30000);
        loadStats(true);
      }
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);
    
    // 监听 Tauri 截屏事件（后备）
    unlisten = await listen('screenshot-taken', () => {
      if (!document.hidden && shouldAutoRefreshOverview()) {
        loadStats(true);
      }
    });
    
    // 监听全局 activity-added 事件（实时同步）
    handleActivityAdded = () => {
      if (!document.hidden && shouldAutoRefreshOverview()) {
        loadStats(true);
      }
    };
    window.addEventListener('activity-added', handleActivityAdded);
  });

  $: if (overviewViewModeReady) {
    persistOverviewViewMode(APP_USAGE_VIEW_MODE_KEY, appUsageViewMode);
  }

  $: if (overviewViewModeReady) {
    persistOverviewViewMode(HOURLY_ACTIVITY_VIEW_MODE_KEY, hourlyActivityViewMode);
  }

  onDestroy(() => {
    if (unlisten) unlisten();
    if (clockInterval) clearInterval(clockInterval);
    if (refreshInterval) clearInterval(refreshInterval);
    if (handleActivityAdded) window.removeEventListener('activity-added', handleActivityAdded);
    if (handleVisibilityChange) document.removeEventListener('visibilitychange', handleVisibilityChange);
    unsubIcons();
  });
</script>

<div class="page-shell" data-locale={currentLocale}>
  <!-- 页面标题 -->
  <div class="page-header">
    <div class="page-title-group">
      <div class="page-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 6.5A2.5 2.5 0 016.5 4H10v6H4V6.5Zm10 0A2.5 2.5 0 0116.5 4H20v6h-6V4Zm-10 11A2.5 2.5 0 016.5 15H10v5H6.5A2.5 2.5 0 014 17.5V15Zm10-2.5H20v2.5A2.5 2.5 0 0117.5 20H14v-5Z" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>{t('overview.title')}</h2>
        <p>
        {overviewSubtitle}
        {#if overviewMode === 'today'}
          <span class="ml-1.5 font-mono text-xs">{formatLocalizedTime(currentTime, { hour: '2-digit', minute: '2-digit' })}</span>
        {/if}
        </p>
      </div>
    </div>
    <div class="page-status-chip {overviewIsLive ? 'text-emerald-600 dark:text-emerald-400' : 'text-slate-500 dark:text-slate-400'}">
      <span class="w-1.5 h-1.5 rounded-full {overviewIsLive ? 'bg-emerald-500 animate-pulse' : 'bg-slate-400'}"></span>
      {overviewStatusLabel}
    </div>
  </div>

  <div class="overview-editorial-shell">
  <div class="overview-command-deck mb-4 flex flex-wrap items-center gap-2">
    <button
      type="button"
      class="page-control-btn {overviewMode === 'today' ? 'page-control-btn-active' : ''}"
      on:click={() => setOverviewMode('today')}
    >
      {t('overview.modeToday')}
    </button>
    <button
      type="button"
      class="page-control-btn {overviewMode === 'week' ? 'page-control-btn-active' : ''}"
      on:click={() => setOverviewMode('week')}
    >
      {t('overview.modeWeek')}
    </button>
    <button
      type="button"
      class="page-control-btn {overviewMode === 'date' ? 'page-control-btn-active' : ''}"
      on:click={() => setOverviewMode('date')}
    >
      {t('overview.modeDate')}
    </button>

    {#if overviewMode === 'date'}
      <div class="overview-date-bar">
        <button
          type="button"
          class="page-control-btn-icon"
          title={t('common.previous')}
          on:click={() => stepOverviewDateBoundary('start', -1)}
        >
          <svg class="h-4 w-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M15 19l-7-7 7-7" />
          </svg>
        </button>

        <div class="overview-date-field">
          <span class="overview-date-label">{t('overview.rangeStart')}</span>
          <input
            type="text"
            bind:value={overviewDateInputFrom}
            class="overview-date-input"
            inputmode="numeric"
            placeholder="YYYY/MM/DD"
            spellcheck="false"
            autocomplete="off"
            on:focus={() => { editingOverviewDateFrom = true; }}
            on:blur={() => commitOverviewDateInput('start')}
            on:keydown={(event) => event.key === 'Enter' && commitOverviewDateInput('start')}
          />
        </div>

        <span class="overview-date-separator">-</span>

        <div class="overview-date-field">
          <span class="overview-date-label">{t('overview.rangeEnd')}</span>
          <input
            type="text"
            bind:value={overviewDateInputTo}
            class="overview-date-input"
            inputmode="numeric"
            placeholder="YYYY/MM/DD"
            spellcheck="false"
            autocomplete="off"
            on:focus={() => { editingOverviewDateTo = true; }}
            on:blur={() => commitOverviewDateInput('end')}
            on:keydown={(event) => event.key === 'Enter' && commitOverviewDateInput('end')}
          />
        </div>

        <button
          type="button"
          class="page-control-btn-icon"
          title={t('common.next')}
          on:click={() => stepOverviewDateBoundary('end', 1)}
        >
          <svg class="h-4 w-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M9 5l7 7-7 7" />
          </svg>
        </button>
      </div>
    {/if}
  </div>

  <!-- 统计卡片：始终渲染，内部切换骨架/真实数据 -->
  <div class="overview-summary-grid grid grid-cols-2 lg:grid-cols-4 gap-3 mb-4">
    {#if loading || !stats}
      {#each [1,2,3,4] as _}
        <div class="min-h-[116px] p-5 rounded-2xl bg-white dark:bg-slate-800/80 border border-slate-100 dark:border-slate-700/60 animate-pulse">
          <div class="flex h-full items-center justify-between gap-4">
            <div class="flex-1">
              <div class="h-3 bg-slate-200 dark:bg-slate-700 rounded w-20"></div>
              <div class="h-8 bg-slate-200 dark:bg-slate-700 rounded w-1/2 mt-6"></div>
            </div>
            <div class="w-11 h-11 rounded-2xl bg-slate-100 dark:bg-slate-700 shrink-0"></div>
          </div>
        </div>
      {/each}
    {:else}
      <StatsCard title={overviewTotalActivityTitle} value={formatDuration(stats.total_duration)} icon="duration" color="indigo" />
      <StatsCard title={overviewWorkDurationTitle} value={formatDuration(stats.work_time_duration || 0)} icon="focus" color="emerald" />
      <StatsCard title={t('overview.browser')} value={formatDuration(stats.browser_duration)} icon="browser" color="blue" />
      <StatsCard title={t('overview.apps')} value={stats.app_usage.length} icon="apps" color="amber" />
    {/if}
  </div>

  {#if error}
    <div class="page-banner-error mb-4">
      <div>
        <p class="font-semibold">{t('overview.loadError')}</p>
        <p class="text-sm mt-1">{error}</p>
      </div>
      <button class="page-action-brand" on:click={loadStats}>{t('overview.retry')}</button>
    </div>
  {/if}

  <!-- 网站访问：始终渲染，加载中显示骨架，无数据显示占位文字 -->
  <div class="page-card overview-panel overview-panel-featured mb-4">
    <h3 class="page-section-title">{t('overview.websiteVisits')}</h3>
    {#if loading || !stats}
      <div class="overview-browser-gallery grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3 animate-pulse">
        {#each [1,2] as _}
          <div class="p-3.5 rounded-xl border border-slate-100 dark:border-slate-700">
            <div class="h-4 bg-slate-200 dark:bg-slate-700 rounded w-3/4 mb-2.5"></div>
            <div class="h-6 bg-slate-200 dark:bg-slate-700 rounded w-1/2 mb-1.5"></div>
            <div class="h-3 bg-slate-100 dark:bg-slate-700/50 rounded w-2/3"></div>
          </div>
        {/each}
      </div>
    {:else if stats.browser_usage && stats.browser_usage.length > 0}
      <div class="overview-browser-gallery grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
        {#each stats.browser_usage as browser}
          <button
            class="group overview-browser-card text-left p-3.5 rounded-xl border border-slate-100 dark:border-slate-700
                   bg-white dark:bg-slate-800/60
                   hover:border-slate-200 dark:hover:border-slate-600 hover:shadow-sm
                   transition-all duration-200"
            on:click={() => {
              selectedBrowser = browser;
              cancelDomainSemanticEdit();
            }}
          >
            <div class="flex items-center gap-2 mb-1.5">
              {#if getAppIconSrc(browser.browser_name, browser.executable_path)}
                <img src={getAppIconSrc(browser.browser_name, browser.executable_path)} alt="" class="w-6 h-6 rounded-md object-cover" />
              {:else}
                <span class="text-xl">🌐</span>
              {/if}
              <span class="font-medium text-slate-700 dark:text-slate-200 truncate">{browser.browser_name}</span>
            </div>
            <div class="text-lg font-bold text-slate-800 dark:text-white mb-1">
              {formatDuration(browser.duration)}
            </div>
            <div class="flex items-center gap-2 text-xs text-slate-400">
              <span>{t('overview.sitesCount', { count: browser.domains.length })}</span>
              <span>·</span>
              <span>{t('overview.pagesCount', { count: browser.domains.reduce((sum, d) => sum + d.urls.length, 0) })}</span>
            </div>
          </button>
        {/each}
      </div>
    {:else}
      <div class="empty-state-compact">
        <div class="empty-state-icon !w-12 !h-12 !mb-3 shadow-none">
          <span class="text-xl">🌐</span>
        </div>
        <p class="empty-state-copy">{overviewNoWebsiteVisitsText}</p>
      </div>
    {/if}
  </div>

  <div class="overview-section-grid">
  <!-- 应用使用：始终渲染 -->
  <div class="page-card overview-panel overview-panel-subtle mb-4">
    <div class="mb-3 flex items-center justify-between gap-3">
      <h3 class="page-section-title !mb-0">{t('overview.appUsage')}</h3>
      <button
        type="button"
        class="page-control-btn-icon"
        title={appUsageViewModeLabel}
        on:click={() => {
          appUsageViewMode = appUsageViewMode === 'row' ? 'column' : 'row';
        }}
      >
        {#if appUsageViewMode === 'row'}
          <svg class="h-4 w-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 7h16M4 12h12M4 17h8" />
          </svg>
        {:else}
          <svg class="h-4 w-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M6 18V9m6 9V6m6 12v-4" />
          </svg>
        {/if}
      </button>
    </div>
    {#if loading || !stats}
      <div class="animate-pulse">
        {#each [1,2,3,4] as _}
          <div class="flex items-center gap-3 mb-3">
            <div class="w-7 h-7 rounded bg-slate-200 dark:bg-slate-700 flex-shrink-0"></div>
            <div class="flex-1 h-3 bg-slate-200 dark:bg-slate-700 rounded"></div>
            <div class="w-16 h-3 bg-slate-100 dark:bg-slate-700/50 rounded"></div>
          </div>
        {/each}
      </div>
    {:else if stats.app_usage.length > 0}
      <AppUsageChart data={stats.app_usage} mode={appUsageViewMode} />
    {:else}
      <div class="empty-state-compact">
        <div class="empty-state-icon !w-12 !h-12 !mb-3 shadow-none">
          <span class="text-xl">📊</span>
        </div>
        <p class="empty-state-copy">{overviewNoAppStatsText}</p>
      </div>
    {/if}
  </div>

  <div class="page-card overview-panel overview-panel-subtle mb-4">
    <div class="mb-3 flex items-center justify-between gap-3">
      <h3 class="page-section-title !mb-0">{t('overview.hourlyActivity')}</h3>
      <button
        type="button"
        class="page-control-btn-icon"
        title={hourlyActivityViewModeLabel}
        on:click={() => {
          hourlyActivityViewMode = hourlyActivityViewMode === 'column' ? 'row' : 'column';
        }}
      >
        {#if hourlyActivityViewMode === 'column'}
          <svg class="h-4 w-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M6 18V9m6 9V6m6 12v-4" />
          </svg>
        {:else}
          <svg class="h-4 w-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 7h16M4 12h12M4 17h8" />
          </svg>
        {/if}
      </button>
    </div>
    {#if loading || !stats}
      <div class="animate-pulse rounded-2xl bg-white dark:bg-slate-800/60">
        <div class="mb-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
          {#each [1,2,3,4] as _}
            <div class="min-h-[88px] rounded-2xl border border-slate-100 dark:border-slate-700/60 p-4">
              <div class="h-3 w-16 rounded bg-slate-200 dark:bg-slate-700"></div>
              <div class="mt-4 h-7 w-20 rounded bg-slate-200 dark:bg-slate-700"></div>
            </div>
          {/each}
        </div>
        <div class="rounded-2xl border border-slate-100 dark:border-slate-700/60 p-4">
          <div class="flex h-40 items-end gap-1.5">
            {#each Array(24) as _, hour}
              <div class="flex h-full flex-1 flex-col items-center justify-end">
                <div
                  class="w-full rounded-t-lg bg-slate-200 dark:bg-slate-700"
                  style={`height: ${Math.max(((hour % 6) + 2) * 12, 18)}%; opacity: 0.8;`}
                ></div>
                <div class="mt-2 h-2 w-7 rounded bg-slate-100 dark:bg-slate-700/60"></div>
              </div>
            {/each}
          </div>
        </div>
      </div>
    {:else}
      <ActivityHourlyChart
        data={stats.hourly_activity_distribution}
        peakHourLabel={hourlyChartPeakHourLabel}
        peakDurationLabel={hourlyChartPeakDurationLabel}
        distributionTitle={hourlyChartDistributionTitle}
        distributionSubtitleKey={hourlyChartDistributionSubtitleKey}
        mode={hourlyActivityViewMode}
      />
    {/if}
  </div>
  </div>
</div>
</div>

<!-- 浏览器详情弹窗 -->
{#if selectedBrowser}
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="fixed inset-0 z-[140] bg-slate-950/52 backdrop-blur-md flex items-center justify-center p-4 animate-fadeIn"
  role="button"
  tabindex="0"
  on:click|self={() => selectedBrowser = null}
  on:keydown={(e) => e.key === 'Escape' && (selectedBrowser = null)}
>
  <div class="card overview-browser-dialog p-0 max-w-2xl w-full max-h-[85vh] overflow-hidden flex flex-col" role="dialog" aria-modal="true">
    <!-- 弹窗头部 -->
      <div class="flex items-center justify-between p-5 border-b border-slate-200 dark:border-slate-700 bg-gradient-to-r from-slate-50 to-white dark:from-slate-800 dark:to-slate-900">
        <div class="flex items-center gap-3">
        {#if getAppIconSrc(selectedBrowser.browser_name, selectedBrowser.executable_path)}
          <img src={getAppIconSrc(selectedBrowser.browser_name, selectedBrowser.executable_path)} alt="" class="w-8 h-8 rounded-lg object-cover" />
        {:else}
          <span class="text-3xl">🌐</span>
        {/if}
        <div>
          <h3 class="text-lg font-bold text-slate-800 dark:text-white">{selectedBrowser.browser_name}</h3>
          <p class="text-sm text-slate-500 dark:text-slate-400">
            {formatDuration(selectedBrowser.duration)} · {t('overview.sitesCount', { count: selectedBrowser.domains.length })} · {t('overview.pagesCount', { count: selectedBrowser.domains.reduce((sum, d) => sum + d.urls.length, 0) })}
          </p>
        </div>
      </div>
      <button class="p-2 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-lg transition-colors" on:click={() => selectedBrowser = null}>
        <svg class="w-5 h-5 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
    
    <!-- 弹窗内容 -->
    <div class="flex-1 overflow-y-auto p-5 space-y-4">
      {#each selectedBrowser.domains as domain}
        <div class="rounded-lg border border-slate-200 dark:border-slate-700 overflow-hidden">
          <!-- 域名头部 -->
          <div class="flex items-center justify-between p-3 bg-slate-50 dark:bg-slate-800/50">
            <div class="flex items-center gap-2">
              <span class="w-2 h-2 rounded-full bg-primary-500"></span>
              <span class="font-medium text-slate-700 dark:text-slate-200">{domain.domain}</span>
              <span class="text-xs text-slate-400 bg-slate-200 dark:bg-slate-700 px-1.5 py-0.5 rounded">
                {t('overview.modalPages', { count: domain.urls.length })}
              </span>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs px-2 py-1 rounded-full bg-primary-50 text-primary-700 dark:bg-primary-900/20 dark:text-primary-300">
                {t('overview.currentCategory', { label: getDomainSemanticLabel(domain) })}
              </span>
              <button
                class="text-xs px-2 py-1 rounded-full border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:border-primary-300 hover:text-primary-600 transition-colors"
                on:click={() => {
                  if (editingDomainKey === domain.domain) {
                    cancelDomainSemanticEdit();
                  } else {
                    startDomainSemanticEdit(domain);
                  }
                }}
              >
                {t('overview.changeCategory')}
              </button>
              <span class="text-sm font-medium text-slate-600 dark:text-slate-300">{formatDuration(domain.duration)}</span>
            </div>
          </div>

          {#if editingDomainKey === domain.domain}
            <div class="px-3 py-3 border-t border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900/40 space-y-2">
              <label
                for={`semantic-category-${domain.domain}`}
                class="block text-xs font-medium text-slate-500 dark:text-slate-400"
              >
                {t('overview.selectCategory')}
              </label>
              <p class="text-xs text-slate-400 dark:text-slate-500">
                {t('overview.semanticCategoryHelp')}
              </p>
              <div class="flex flex-col gap-2 md:flex-row">
                <select
                  id={`semantic-category-${domain.domain}`}
                  class="flex-1 rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-800 px-3 py-2 text-sm text-slate-700 dark:text-slate-100"
                  bind:value={editingSemanticCategory}
                >
                  {#each getSemanticCategoryOptions() as option}
                    <option value={option}>{translateSemanticCategoryLabel(option)}</option>
                  {/each}
                </select>
                <div class="flex items-center gap-2">
                  <button
                    class="px-3 py-2 rounded-lg bg-primary-500 text-white text-sm disabled:opacity-50"
                    disabled={!editingSemanticCategory.trim() || savingDomainKey === domain.domain}
                    on:click={() => saveDomainSemanticRule(domain)}
                  >
                    {#if savingDomainKey === domain.domain}
                      {t('overview.saving')}
                    {:else}
                      {t('overview.save')}
                    {/if}
                  </button>
                  <button
                    class="px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-600 text-sm text-slate-600 dark:text-slate-300"
                    disabled={savingDomainKey === domain.domain}
                    on:click={cancelDomainSemanticEdit}
                  >
                    {t('overview.cancel')}
                  </button>
                </div>
              </div>
            </div>
          {/if}
          
          <!-- URL 列表，支持展开/收起超出的部分 -->
          <div class="divide-y divide-slate-100 dark:divide-slate-700/50">
            {#each (expandedDomains.has(domain.domain) ? domain.urls : domain.urls.slice(0, 10)) as url}
              <div class="flex items-center justify-between p-3 hover:bg-slate-50 dark:hover:bg-slate-800/30 transition-colors">
                <div class="flex-1 min-w-0 mr-3">
                  <p
                    class="text-sm text-slate-600 dark:text-slate-300 truncate"
                    title={formatBrowserUrlForDisplay(url.url)}
                  >
                    {formatBrowserUrlForDisplay(url.url)}
                  </p>
                </div>
                <span class="text-xs text-slate-400 whitespace-nowrap">{formatDuration(url.duration)}</span>
              </div>
            {/each}
            {#if domain.urls.length > 10}
              <!-- 展开/收起按钮，让用户可以查看全部 URL -->
              <button
                class="w-full p-3 text-center text-xs text-primary-500 hover:text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-900/10 transition-colors flex items-center justify-center gap-1"
                on:click={() => {
                  if (expandedDomains.has(domain.domain)) {
                    expandedDomains.delete(domain.domain);
                  } else {
                    expandedDomains.add(domain.domain);
                  }
                  expandedDomains = expandedDomains;
                }}
              >
                {#if expandedDomains.has(domain.domain)}
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"/></svg>
                  {t('common.collapse')}
                {:else}
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/></svg>
                  {t('common.expandAll', { count: domain.urls.length })}
                {/if}
              </button>
            {/if}
          </div>
        </div>
      {/each}
      
      {#if selectedBrowser.domains.length === 0}
        <div class="text-center py-8 text-slate-400">
          <span class="text-3xl">📭</span>
          <p class="mt-2">{t('common.noRecords')}</p>
        </div>
      {/if}
    </div>
  </div>
</div>
{/if}
