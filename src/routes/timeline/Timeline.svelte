<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-shell';
  import { cache } from '../../lib/stores/cache.js';
  import { confirm } from '../../lib/stores/confirm.js';
  import { showToast } from '../../lib/stores/toast.js';
  import { appIconStore, getIconCacheKey, preloadAppIcons } from '../../lib/stores/iconCache.js';
  import {
    formatDurationLocalized,
    formatLocalizedTime,
    locale,
    t,
    translateCategoryLabel,
  } from '$lib/i18n/index.js';
  import { resolveAppIconSrc } from '../../lib/utils/appVisuals.js';
  import { formatBrowserUrlForDisplay } from '../../lib/utils/browserUrl.js';
  import { prepareTimelineActivities, upsertTimelineActivity } from './timelineData.js';

  // 获取本地日期（避免 UTC 时区问题）
  function getLocalDateString() {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, '0');
    const day = String(now.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  let activities = [];
  let hourlySummaries = [];
  let loading = true;
  let error = null;
  let selectedDate = getLocalDateString();
  let selectedActivity = null;
  let unlisten = null;
  let currentTime = new Date();
  let clockInterval;
  let appIcons = {};

  // LRU 缓存：防止长时间运行内存无限增长
  // 缩略图 ~80KB/条，60 条 ≈ 5MB；高清图 ~300KB/条，20 条 ≈ 6MB
  const THUMBNAIL_CACHE_LIMIT = 60;
  const FULLIMAGE_CACHE_LIMIT = 20;
  let thumbnailCache = {};
  let thumbnailKeys = [];   // 插入顺序追踪，用于淘汰最旧条目
  let fullImageCache = {};
  let fullImageKeys = [];
  $: currentLocale = $locale;

  // 向 LRU 缓存中写入，超出上限时淘汰最旧条目释放内存
  function lruSet(cache, keys, limit, key, value) {
    if (!(key in cache)) {
      keys.push(key);
    }
    cache[key] = value;
    while (keys.length > limit) {
      const evicted = keys.shift();
      delete cache[evicted];
    }
  }

  // 清空图片缓存（日期切换时调用，释放旧数据占用的内存）
  function clearImageCaches() {
    thumbnailCache = {};
    thumbnailKeys = [];
    fullImageCache = {};
    fullImageKeys = [];
  }

  const unsubIcons = appIconStore.subscribe(v => appIcons = v);

  // 分类名称和颜色
  const categoryInfo = {
    development: { color: 'blue', icon: '⚡' },
    browser: { color: 'green', icon: '🌐' },
    communication: { color: 'yellow', icon: '💬' },
    office: { color: 'purple', icon: '📝' },
    design: { color: 'pink', icon: '🎨' },
    entertainment: { color: 'red', icon: '🎮' },
    other: { color: 'gray', icon: '📁' },
  };
  const categoryOptions = Object.keys(categoryInfo).map((value) => ({
    value,
  }));
  let categorySaving = false;

  function getCategoryMeta(category) {
    return {
      ...(categoryInfo[category] || categoryInfo.other),
      name: translateCategoryLabel(category || 'other'),
    };
  }

  // 格式化时间
  function formatTime(timestamp) {
    return formatLocalizedTime(new Date(timestamp * 1000), {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  }

  // 格式化时长
  function formatDuration(seconds) {
    return formatDurationLocalized(seconds);
  }

  function getTimelineIconSrc(activity) {
    return resolveAppIconSrc(
      activity.app_name,
      appIcons[getIconCacheKey({ appName: activity.app_name, executablePath: activity.executable_path })]
    );
  }

  function normalizeAppMatchKey(appName) {
    return (appName || '').trim().toLowerCase();
  }

  // 优化窗口标题显示
  function formatWindowTitle(title, appName, browserUrl = null) {
    // 如果有有效标题
    if (title && title.trim() !== '') {
      // 移除常见的应用名称后缀
      let cleanTitle = title
        .replace(/ - Google Chrome$/i, '')
        .replace(/ - Chrome$/i, '')
        .replace(/ - Mozilla Firefox$/i, '')
        .replace(/ - Firefox$/i, '')
        .replace(/ - Safari$/i, '')
        .replace(/ - Microsoft Edge$/i, '')
        .replace(/ - Visual Studio Code$/i, '')
        .replace(/ · GitHub$/i, '')
        .replace(/ - YouTube$/i, '')
        .trim();
      
      // 如果标题太长，截断
      if (cleanTitle.length > 60) {
        cleanTitle = cleanTitle.substring(0, 57) + '...';
      }
      
      return cleanTitle || title;
    }
    
    // 无标题时，如果有 URL 显示域名
    if (browserUrl) {
      try {
        const url = new URL(formatBrowserUrlForDisplay(browserUrl));
        return url.hostname;
      } catch {
        return formatBrowserUrlForDisplay(browserUrl).substring(0, 40);
      }
    }
    
    // 完全无信息
    return t('timeline.inUse', { appName });
  }

  // 加载缩略图（列表用，400px），使用 LRU 缓存控制内存
  async function loadThumbnail(screenshotPath) {
    if (!screenshotPath) {
      return null;
    }
    if (thumbnailCache[screenshotPath]) {
      return thumbnailCache[screenshotPath];
    }
    try {
      const base64 = await invoke('get_screenshot_thumbnail', { path: screenshotPath });
      const dataUrl = `data:image/jpeg;base64,${base64}`;
      lruSet(thumbnailCache, thumbnailKeys, THUMBNAIL_CACHE_LIMIT, screenshotPath, dataUrl);
      return dataUrl;
    } catch (e) {
      console.warn('加载缩略图失败:', e);
      return null;
    }
  }

  // 加载高分辨率图片（详情用，1200px），使用 LRU 缓存控制内存
  async function loadFullImage(screenshotPath) {
    if (!screenshotPath) {
      return null;
    }
    if (fullImageCache[screenshotPath]) {
      return fullImageCache[screenshotPath];
    }
    try {
      const base64 = await invoke('get_screenshot_full', { path: screenshotPath });
      const dataUrl = `data:image/jpeg;base64,${base64}`;
      lruSet(fullImageCache, fullImageKeys, FULLIMAGE_CACHE_LIMIT, screenshotPath, dataUrl);
      return dataUrl;
    } catch (e) {
      console.warn('加载高清图失败:', e);
      return await loadThumbnail(screenshotPath);
    }
  }

  const PAGE_SIZE = 12; // 每次加载 12 条 (3行 x 4列)
  let offset = 0;
  let hasMore = true;
  let loadingMore = false;

  // 加载时间线数据（重置）
  async function loadTimeline() {
    // 禁用缓存：每次都从后端加载最新数据，确保数据一致性
    // 后端已实现 GROUP BY 聚合，无需前端缓存旧数据

    // 2. 缓存未命中，请求后端
    loading = true;
    error = null;
    offset = 0;
    hasMore = true;
    // 日期切换时释放旧图片缓存，防止内存无限增长
    clearImageCaches();
    
    try {
      const [activitiesData, summariesData] = await Promise.all([
        invoke('get_timeline', { date: selectedDate, limit: PAGE_SIZE, offset: 0 }),
        invoke('get_hourly_summaries', { date: selectedDate }),
      ]);

      activities = prepareTimelineActivities(activitiesData);
      
      hourlySummaries = summariesData;
      offset = activities.length;
      hasMore = activitiesData.length >= PAGE_SIZE;
      
      // 保存到缓存（直接使用后端返回结果）
      cache.setTimeline(selectedDate, activities, summariesData);
      
      // 预加载缩略图
      activities.forEach(a => loadThumbnail(a.screenshot_path));
      
      // 后台预加载前 6 张高清图（避免点击时等待）
      activities.slice(0, 6).forEach(a => loadFullImage(a.screenshot_path));
      
      // 预加载应用图标（获取唯一应用名并批量加载）
      const uniqueIconEntries = Array.from(
        new Map(
          activities.map((activity) => [
            getIconCacheKey({ appName: activity.app_name, executablePath: activity.executable_path }),
            { appName: activity.app_name, executablePath: activity.executable_path },
          ])
        ).values()
      );
      preloadAppIcons(uniqueIconEntries, invoke);
    } catch (e) {
      error = e.toString();
      console.error('获取时间线失败:', e);
    } finally {
      loading = false;
    }
  }

  // 加载更多
  async function loadMore() {
    if (loadingMore || !hasMore) return;
    loadingMore = true;

    try {
      const moreActivities = await invoke('get_timeline', { 
        date: selectedDate, 
        limit: PAGE_SIZE, 
        offset: offset 
      });

      if (moreActivities.length > 0) {
        activities = [...activities, ...moreActivities];
        offset += moreActivities.length;
        // 预加载新图片
        moreActivities.forEach(a => loadThumbnail(a.screenshot_path));
        const iconEntries = Array.from(
          new Map(
            moreActivities.map((activity) => [
              getIconCacheKey({ appName: activity.app_name, executablePath: activity.executable_path }),
              { appName: activity.app_name, executablePath: activity.executable_path },
            ])
          ).values()
        );
        preloadAppIcons(iconEntries, invoke);
      }
      
      if (moreActivities.length < PAGE_SIZE) {
        hasMore = false;
      }
    } catch (e) {
      console.error('加载更多失败:', e);
    } finally {
      loadingMore = false;
    }
  }

  // 查看活动详情
  async function viewActivity(activity) {
    selectedActivity = { ...activity, thumbnailLoading: true };
    
    // 从数据库获取最新数据（包括 OCR 结果）
    if (activity.id) {
      try {
        const freshActivity = await invoke('get_activity', { id: activity.id });
        if (freshActivity) {
          activity = freshActivity;
        }
      } catch (e) {
        console.warn('获取最新活动数据失败:', e);
      }
    }
    
    // 加载详情页的高清图
    const thumbnail = await loadFullImage(activity.screenshot_path);
    selectedActivity = { ...activity, thumbnail, thumbnailLoading: false };
  }

  // 打开外部链接
  async function openUrl(url) {
    if (url) {
      try {
        await open(url);
      } catch (e) {
        console.error('打开链接失败:', e);
      }
    }
  }

  // 关闭详情
  function closeDetail() {
    selectedActivity = null;
    categorySaving = false;
  }

  async function changeAppCategory(activity, nextCategory) {
    if (!activity || !nextCategory || categorySaving) return;
    if ((activity.category || 'other') === nextCategory) return;

    const targetInfo = getCategoryMeta(nextCategory);
    const confirmed = await confirm({
      title: t('timeline.changeCategoryTitle'),
      message: t('timeline.changeCategoryMessage', {
        appName: activity.app_name,
        category: targetInfo.name,
      }),
      confirmText: t('timeline.confirmChange'),
      cancelText: t('timeline.cancel'),
      tone: 'warning',
    });
    if (!confirmed) return;

    categorySaving = true;
    try {
      const updatedCount = await invoke('set_app_category_rule', {
        appName: activity.app_name,
        category: nextCategory,
        syncHistory: true,
      });

      const appMatchKey = normalizeAppMatchKey(activity.app_name);
      activities = activities.map((item) =>
        normalizeAppMatchKey(item.app_name) === appMatchKey
          ? { ...item, category: nextCategory }
          : item
      );

      if (selectedActivity && normalizeAppMatchKey(selectedActivity.app_name) === appMatchKey) {
        selectedActivity = { ...selectedActivity, category: nextCategory };
      }

      showToast(
        t('timeline.categoryUpdated', {
          appName: activity.app_name,
          category: targetInfo.name,
          count: updatedCount,
        }),
        'success'
      );
    } catch (e) {
      console.error('修改应用默认分类失败:', e);
      showToast(
        t('timeline.categoryUpdateFailed', {
          appName: activity.app_name,
          error: e,
        }),
        'error'
      );
    } finally {
      categorySaving = false;
    }
  }

  // 记录上次加载的日期
  let lastLoadedDate = null;

  // 日期变化时重新加载
  $: if (selectedDate && selectedDate !== lastLoadedDate) {
    lastLoadedDate = selectedDate;
    loadTimeline();
  }

  // 检查是否是今天
  $: isToday = selectedDate === getLocalDateString();

  onMount(async () => {
    // 每秒更新时钟
    clockInterval = setInterval(() => {
      currentTime = new Date();
    }, 1000);
    
    // 初始加载通过响应式触发
    
    // 监听新截屏事件，智能更新（合并或新增）
    // 核心逻辑：后端已完成聚合，前端只按 id 替换，否则视作新活动插入
    unlisten = await listen('screenshot-taken', (event) => {
      if (isToday) {
        const newActivity = event.payload;
        activities = upsertTimelineActivity(activities, newActivity);
        cache.clear();
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (clockInterval) clearInterval(clockInterval);
    unsubIcons();
  });
</script>

<div class="page-shell" data-locale={currentLocale}>
  <!-- 页面标题 -->
  <div class="page-header">
    <div class="page-title-group">
      <div class="page-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 7h14M5 12h9M5 17h14" />
          <circle cx="17" cy="12" r="2.5" stroke-width="1.8" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>{t('timeline.title')}</h2>
        <p>
        {t('timeline.subtitle')}
        {#if isToday}
          <span class="ml-1.5 inline-flex items-center gap-1.5">
            <span class="w-1.5 h-1.5 bg-emerald-500 rounded-full animate-pulse"></span>
            <span class="font-mono text-xs text-emerald-600 dark:text-emerald-400">{formatLocalizedTime(currentTime, { hour: '2-digit', minute: '2-digit' })}</span>
          </span>
        {/if}
        </p>
      </div>
    </div>
    <div class="page-toolbar">
      {#key `timeline-date-${currentLocale}`}
        <input
          type="date"
          bind:value={selectedDate}
          lang={currentLocale}
          class="page-control-input"
        />
      {/key}
      <button class="page-control-btn-icon" on:click={loadTimeline} title={t('timeline.refreshTitle')}>
        <svg class="w-4 h-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
      </button>
    </div>
  </div>

  {#if loading}
    <div class="flex items-center justify-center h-64">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
    </div>
  {:else if error}
    <div class="page-banner-error">
      <div>
        <p class="font-semibold">{t('timeline.loadError')}</p>
        <p class="text-sm mt-1">{error}</p>
      </div>
      <button class="page-action-brand" on:click={loadTimeline}>{t('timeline.retry')}</button>
    </div>
  {:else if activities.length === 0}
    <div class="empty-state-lg">
      <div class="empty-state-icon">
        <span class="text-2xl">📝</span>
      </div>
      <p class="empty-state-copy">{t('timeline.empty')}</p>
    </div>
  {:else}
    <!-- 统计摘要 -->
    <div class="mb-4 flex items-center justify-between">
      <div class="flex items-center gap-3 text-sm text-slate-500 dark:text-slate-400">
        <span>{t('timeline.recordSummary', { dateLabel: isToday ? t('timeline.todayLabel') : selectedDate, count: activities.length })}</span>
        <span class="text-slate-300 dark:text-slate-600">|</span>
        <span>00:00 - {formatTime(activities[0].timestamp)}</span>
      </div>
      
      <!-- 时段摘要链接 -->
        <a
          href="#/timeline/summary"
          class="page-control-btn"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
          </svg>
          {t('timeline.periodSummary')}
          {#if hourlySummaries.length > 0}
            <span class="px-1.5 py-0.5 text-xs bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 rounded-full">{hourlySummaries.length}</span>
          {/if}
          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
        </a>
    </div>
    
    <div class="page-card overflow-hidden p-0">
      <!-- 时间线列表 -->
      <div class="divide-y divide-slate-200 dark:divide-slate-700">
        {#each activities as activity, i}
          {@const info = getCategoryMeta(activity.category)}
          <button
            class="w-full p-4 flex items-center gap-4 hover:bg-slate-50 dark:hover:bg-slate-700/50 transition-colors text-left"
            on:click={() => viewActivity(activity)}
          >
            <!-- 时间 -->
            <div class="w-20 text-sm text-slate-500 dark:text-slate-400 font-mono">
              {formatTime(activity.timestamp)}
            </div>

            <!-- 应用图标 -->
            <div class="w-10 h-10 rounded-lg flex items-center justify-center text-lg flex-shrink-0 overflow-hidden
              {info.color === 'blue' ? 'bg-blue-100 dark:bg-blue-900/30' : ''}
              {info.color === 'green' ? 'bg-green-100 dark:bg-green-900/30' : ''}
              {info.color === 'yellow' ? 'bg-yellow-100 dark:bg-yellow-900/30' : ''}
              {info.color === 'purple' ? 'bg-purple-100 dark:bg-purple-900/30' : ''}
              {info.color === 'pink' ? 'bg-pink-100 dark:bg-pink-900/30' : ''}
              {info.color === 'red' ? 'bg-red-100 dark:bg-red-900/30' : ''}
              {info.color === 'gray' ? 'bg-slate-100 dark:bg-slate-700' : ''}
            ">
              {#if getTimelineIconSrc(activity)}
                <img src={getTimelineIconSrc(activity)}
                     alt={activity.app_name}
                     class="w-7 h-7 rounded-md app-icon object-cover" />
              {:else}
                {info.icon}
              {/if}
            </div>

            <!-- 应用信息 -->
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <span class="font-medium text-slate-800 dark:text-white">{activity.app_name}</span>
                <span class="text-[10px] text-slate-400 dark:text-slate-500">{info.name}</span>
              </div>
              <p class="text-sm text-slate-500 dark:text-slate-400 truncate mt-0.5" title={activity.window_title}>
                {formatWindowTitle(activity.window_title, activity.app_name, activity.browser_url)}
              </p>
            </div>

            <!-- 时长 -->
            <div class="text-sm text-slate-500 dark:text-slate-400 flex-shrink-0">
              {formatDuration(activity.duration)}
            </div>

            <!-- 箭头 -->
            <svg class="w-5 h-5 text-slate-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
            </svg>
          </button>
        {/each}
      </div>

      <!-- 加载更多按钮 -->
      {#if hasMore}
        <div class="p-4 border-t border-slate-200 dark:border-slate-700">
          <button
            on:click={loadMore}
            disabled={loadingMore}
            class="w-full min-h-10 py-2 flex items-center justify-center gap-2 text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-700/50 rounded-lg transition-colors border border-dashed border-slate-300 dark:border-slate-600 hover:border-solid disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {#if loadingMore}
              <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-slate-500"></div>
              {t('timeline.loadingMore')}
            {:else}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
              </svg>
              {t('timeline.loadMore')}
            {/if}
          </button>
        </div>
      {:else if activities.length > 0}
        <div class="p-4 text-center text-xs text-slate-400 border-t border-slate-200 dark:border-slate-700">
          {t('timeline.noMore')}
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- 活动详情弹窗 -->
{#if selectedActivity}
  {@const info = getCategoryMeta(selectedActivity.category)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
    role="button"
    tabindex="0"
    on:click|self={closeDetail}
    on:keydown={(e) => e.key === 'Escape' && closeDetail()}
  >
    <div class="bg-white dark:bg-slate-800 rounded-xl shadow-xl max-w-3xl w-full max-h-[90vh] overflow-auto" role="dialog" aria-modal="true">
      <!-- 头部 -->
      <div class="p-6 border-b border-slate-200 dark:border-slate-700">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="w-12 h-12 rounded-lg flex items-center justify-center text-2xl overflow-hidden
              {info.color === 'blue' ? 'bg-blue-100 dark:bg-blue-900/30' : ''}
              {info.color === 'green' ? 'bg-green-100 dark:bg-green-900/30' : ''}
              {info.color === 'yellow' ? 'bg-yellow-100 dark:bg-yellow-900/30' : ''}
              {info.color === 'purple' ? 'bg-purple-100 dark:bg-purple-900/30' : ''}
              {info.color === 'pink' ? 'bg-pink-100 dark:bg-pink-900/30' : ''}
              {info.color === 'red' ? 'bg-red-100 dark:bg-red-900/30' : ''}
              {info.color === 'gray' ? 'bg-slate-100 dark:bg-slate-700' : ''}
            ">
              {#if getTimelineIconSrc(selectedActivity)}
                <img src={getTimelineIconSrc(selectedActivity)}
                     alt={selectedActivity.app_name}
                     class="w-9 h-9 rounded-lg app-icon object-cover" />
              {:else}
                {info.icon}
              {/if}
            </div>
            <div>
              <h3 class="text-lg font-semibold text-slate-800 dark:text-white">{selectedActivity.app_name}</h3>
              <p class="text-sm text-slate-500 dark:text-slate-400">{info.name}</p>
            </div>
          </div>
          <button class="btn btn-ghost" on:click={closeDetail}>
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      <!-- 内容 -->
      <div class="p-6 space-y-4">
        <div>
          <div class="flex items-center justify-between gap-3">
            <div>
              <span class="text-sm font-medium text-slate-500 dark:text-slate-400">{t('timeline.detail.appCategory')}</span>
              <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">
                {t('timeline.detail.appCategoryHelp')}
              </p>
            </div>
            {#if categorySaving}
              <span class="text-xs text-slate-400">{t('timeline.detail.saving')}</span>
            {/if}
          </div>
          <div class="mt-3 grid grid-cols-2 gap-2 md:grid-cols-4">
            {#each categoryOptions as option}
              <button
                on:click={() => changeAppCategory(selectedActivity, option.value)}
                class="segment-btn rounded-lg border px-3 py-2 text-sm
                  {(selectedActivity.category || 'other') === option.value
                    ? 'settings-segment-success'
                    : 'settings-segment-idle'}"
                disabled={categorySaving}
              >
                {translateCategoryLabel(option.value)}
              </button>
            {/each}
          </div>
        </div>

        <!-- 截图预览 -->
        <div>
          <span class="text-sm font-medium text-slate-500 dark:text-slate-400">{t('timeline.detail.screenshot')}</span>
          <!-- 容器居中对齐，避免图片尺寸小时产生大面积空白 -->
          <div class="mt-2 rounded-lg overflow-hidden bg-slate-100 dark:bg-slate-700 flex items-center justify-center min-h-[120px]">
            {#if selectedActivity.thumbnailLoading}
              <div class="py-12 flex items-center justify-center">
                <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
              </div>
            {:else if selectedActivity.thumbnail}
              <!-- max-h 限制高度防止超高图片撑开弹窗，object-contain 保持比例居中 -->
              <img src={selectedActivity.thumbnail} alt={t('timeline.detail.screenshotAlt')} class="max-w-full max-h-96 object-contain" />
            {:else if selectedActivity.screenshot_path}
              <div class="py-12 flex items-center justify-center text-slate-400">
                <span>{t('timeline.detail.screenshotLoadFailed')}</span>
              </div>
            {:else}
              <div class="py-12 flex items-center justify-center text-slate-400">
                <span>{t('timeline.detail.screenshotMissing')}</span>
              </div>
            {/if}
          </div>
        </div>

        <div>
          <span class="text-sm font-medium text-slate-500 dark:text-slate-400">{t('timeline.detail.windowTitle')}</span>
          <p class="text-base text-slate-800 dark:text-white mt-1 break-all leading-relaxed">{selectedActivity.window_title || t('timeline.noTitle')}</p>
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <span class="text-sm font-medium text-slate-500 dark:text-slate-400">{t('timeline.detail.recordTime')}</span>
            <p class="text-base text-slate-800 dark:text-white mt-1 font-mono">{formatTime(selectedActivity.timestamp)}</p>
          </div>
          <div>
            <span class="text-sm font-medium text-slate-500 dark:text-slate-400">{t('timeline.detail.duration')}</span>
            <p class="text-base text-slate-800 dark:text-white mt-1">{formatDuration(selectedActivity.duration)}</p>
          </div>
        </div>
        {#if selectedActivity.browser_url}
          <div>
            <span class="text-sm font-medium text-slate-500 dark:text-slate-400">{t('timeline.detail.visitedUrl')}</span>
            <button 
              on:click={() => openUrl(selectedActivity.browser_url)}
              class="text-primary-600 dark:text-primary-400 mt-1 text-sm hover:underline break-all block text-left cursor-pointer"
            >
              {formatBrowserUrlForDisplay(selectedActivity.browser_url)}
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
