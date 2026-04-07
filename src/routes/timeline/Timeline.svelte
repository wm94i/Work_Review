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
  import LocalizedDatePicker from '../../lib/components/LocalizedDatePicker.svelte';

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
  let handleVisibilityChange;
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

  function formatTimelineAnchor(timestamp) {
    return formatLocalizedTime(new Date(timestamp * 1000), {
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function getTimelineIconSrc(activity) {
    return resolveAppIconSrc(
      activity.app_name,
      appIcons[getIconCacheKey({ appName: activity.app_name, executablePath: activity.executable_path })]
    );
  }

  function getTimelineTitle(activity) {
    return formatWindowTitle(activity.window_title, activity.app_name, activity.browser_url);
  }

  function getTimelineThumbnail(activity) {
    if (!activity?.screenshot_path) {
      return null;
    }
    return thumbnailCache[activity.screenshot_path] || null;
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
      thumbnailCache = { ...thumbnailCache };
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
  const FEATURED_DURATION_THRESHOLD = 20 * 60;
  const FEATURED_CONTEXT_THRESHOLD = 10 * 60;
  const FEATURED_MIN_GAP = 2;
  const FEATURED_MAX_ITEMS = 4;
  let offset = 0;
  let hasMore = true;
  let loadingMore = false;

  function selectFeaturedActivityIds(items) {
    const featuredIds = [];
    const maxFeaturedCount = Math.min(FEATURED_MAX_ITEMS, Math.max(1, Math.ceil(items.length / 4)));
    let lastFeaturedIndex = -99;

    for (let index = 0; index < items.length; index += 1) {
      const activity = items[index];
      const previous = items[index - 1];

      if (!activity?.id || !activity.screenshot_path) {
        continue;
      }

      let score = 0;
      if ((activity.duration || 0) >= FEATURED_DURATION_THRESHOLD) {
        score += 3;
      } else if ((activity.duration || 0) >= FEATURED_CONTEXT_THRESHOLD) {
        score += 1;
      }
      if (activity.browser_url) {
        score += 1;
      }
      if (
        previous
        && (normalizeAppMatchKey(previous.app_name) !== normalizeAppMatchKey(activity.app_name)
          || (previous.category || 'other') !== (activity.category || 'other'))
      ) {
        score += 1;
      }
      if (index === 0) {
        score += 1;
      }
      if (score < 3 || index - lastFeaturedIndex < FEATURED_MIN_GAP) {
        continue;
      }

      featuredIds.push(activity.id);
      lastFeaturedIndex = index;

      if (featuredIds.length >= maxFeaturedCount) {
        break;
      }
    }

    if (featuredIds.length === 0) {
      const fallback = items.find((activity) => activity?.id && activity.screenshot_path);
      if (fallback) {
        featuredIds.push(fallback.id);
      }
    }

    return featuredIds;
  }

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

      cache.invalidate('overview');

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
  let featuredActivityIds = new Set();

  // 日期变化时重新加载
  $: if (selectedDate && selectedDate !== lastLoadedDate) {
    lastLoadedDate = selectedDate;
    loadTimeline();
  }

  $: featuredActivityIds = new Set(selectFeaturedActivityIds(activities));

  // 检查是否是今天
  $: isToday = selectedDate === getLocalDateString();

  onMount(async () => {
    if (!document.hidden) {
      clockInterval = setInterval(() => {
        currentTime = new Date();
      }, 1000);
    }

    handleVisibilityChange = () => {
      if (document.hidden) {
        clearInterval(clockInterval);
        clockInterval = null;
      } else {
        currentTime = new Date();
        clockInterval = setInterval(() => {
          currentTime = new Date();
        }, 1000);
        if (isToday) {
          loadTimeline();
        }
      }
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);
    
    // 初始加载通过响应式触发
    
    // 监听新截屏事件，智能更新（合并或新增）
    // 核心逻辑：后端已完成聚合，前端只按 id 替换，否则视作新活动插入
    unlisten = await listen('screenshot-taken', (event) => {
      if (isToday && !document.hidden) {
        const newActivity = event.payload;
        activities = upsertTimelineActivity(activities, newActivity);
        cache.clear();
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (clockInterval) clearInterval(clockInterval);
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
        <LocalizedDatePicker
          bind:value={selectedDate}
          localeCode={currentLocale}
          triggerClass="page-control-input w-auto"
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
    <div class="timeline-summary-strip">
      <div class="timeline-summary-copy">
        <span>{t('timeline.recordSummary', { dateLabel: isToday ? t('timeline.todayLabel') : selectedDate, count: activities.length })}</span>
        <span class="timeline-summary-divider">|</span>
        <span>00:00 - {formatTime(activities[0].timestamp)}</span>
      </div>
      
      <!-- 时段摘要链接 -->
        <a
          href="#/timeline/summary"
          class="page-control-btn timeline-summary-action"
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
    
    <div class="page-card timeline-editorial-board overflow-hidden p-0">
      <!-- 时间线列表 -->
      <div class="timeline-editorial-shell">
        <div class="timeline-rail" aria-hidden="true"></div>
        {#each activities as activity, i}
          {@const info = getCategoryMeta(activity.category)}
          {@const featured = featuredActivityIds.has(activity.id)}
          {@const timelineTitle = getTimelineTitle(activity)}
          <button
            class={`timeline-entry ${featured ? 'timeline-entry-featured' : 'timeline-entry-compact'}`}
            on:click={() => viewActivity(activity)}
          >
            <div class="timeline-entry-anchor">
              <div class="timeline-entry-time">{formatTimelineAnchor(activity.timestamp)}</div>
              <div class={`timeline-entry-marker ${featured ? 'timeline-entry-marker-featured' : ''}`}></div>
            </div>

            {#if featured}
              <div class="timeline-entry-card timeline-entry-card-featured">
                <div class="timeline-featured-media">
                  {#if getTimelineThumbnail(activity)}
                    <img
                      src={getTimelineThumbnail(activity)}
                      alt={t('timeline.detail.screenshotAlt')}
                      class="timeline-featured-image"
                    />
                  {:else}
                    <div class="timeline-featured-image timeline-featured-image-placeholder">
                      <div class="timeline-featured-image-glow"></div>
                    </div>
                  {/if}
                </div>
                <div class="timeline-featured-copy">
                  <div class="timeline-entry-meta timeline-entry-meta-featured">
                    <div class="timeline-entry-app">
                      <div class={`timeline-app-icon timeline-app-icon-${info.color}`}>
                        {#if getTimelineIconSrc(activity)}
                          <img src={getTimelineIconSrc(activity)}
                               alt={activity.app_name}
                               class="timeline-app-icon-image app-icon object-cover" />
                        {:else}
                          <span>{info.icon}</span>
                        {/if}
                      </div>
                      <div class="timeline-entry-heading timeline-entry-heading-featured">
                        <span class="timeline-entry-app-name">{activity.app_name}</span>
                        <span class="timeline-entry-category timeline-entry-category-pill">{info.name}</span>
                      </div>
                    </div>
                    <div class="timeline-entry-duration-chip">{formatDuration(activity.duration)}</div>
                  </div>
                  <p class="timeline-entry-title timeline-entry-title-featured" title={activity.window_title}>
                    {timelineTitle}
                  </p>
                  {#if activity.browser_url}
                    <p class="timeline-entry-url">{formatBrowserUrlForDisplay(activity.browser_url)}</p>
                  {/if}
                </div>
              </div>
            {:else}
              <div class="timeline-entry-card timeline-entry-card-compact timeline-entry-card-compact-grid">
                <div class="timeline-entry-app timeline-entry-app-compact">
                  <div class={`timeline-app-icon timeline-app-icon-${info.color}`}>
                    {#if getTimelineIconSrc(activity)}
                      <img src={getTimelineIconSrc(activity)}
                           alt={activity.app_name}
                           class="timeline-app-icon-image app-icon object-cover" />
                    {:else}
                      <span>{info.icon}</span>
                    {/if}
                  </div>
                  <div class="timeline-entry-heading">
                    <span class="timeline-entry-app-name">{activity.app_name}</span>
                    <span class="timeline-entry-category timeline-entry-category-pill">{info.name}</span>
                  </div>
                </div>
                <p class="timeline-entry-title timeline-entry-title-compact" title={activity.window_title}>
                  {timelineTitle}
                </p>
                <div class="timeline-entry-tail timeline-entry-tail-compact">
                  <span class="timeline-entry-duration">{formatDuration(activity.duration)}</span>
                  <svg class="timeline-entry-arrow" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                  </svg>
                </div>
              </div>
            {/if}
          </button>
        {/each}
      </div>

      <!-- 加载更多按钮 -->
      {#if hasMore}
        <div class="timeline-load-more">
          <button
            on:click={loadMore}
            disabled={loadingMore}
            class="timeline-load-more-btn"
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
        <div class="timeline-load-more timeline-load-more-end">
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
    class="fixed inset-0 z-[140] bg-slate-950/52 backdrop-blur-md flex items-center justify-center p-4 animate-fadeIn"
    role="button"
    tabindex="0"
    on:click|self={closeDetail}
    on:keydown={(e) => e.key === 'Escape' && closeDetail()}
  >
    <div class="timeline-detail-dialog bg-white dark:bg-slate-800 rounded-xl shadow-xl max-w-3xl w-full max-h-[90vh] overflow-auto" role="dialog" aria-modal="true">
      <!-- 头部 -->
      <div class="timeline-detail-header p-6 border-b border-slate-200 dark:border-slate-700">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class={`timeline-app-icon timeline-app-icon-lg timeline-app-icon-${info.color}`}>
              {#if getTimelineIconSrc(selectedActivity)}
                <img src={getTimelineIconSrc(selectedActivity)}
                     alt={selectedActivity.app_name}
                     class="timeline-app-icon-image timeline-app-icon-image-lg app-icon object-cover" />
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

<style>
  .timeline-summary-strip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    padding: 0 0.25rem;
  }

  .timeline-summary-copy {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
    color: #6b7280;
    font-size: 0.92rem;
  }

  .timeline-summary-divider {
    color: #d6d3d1;
  }

  .timeline-summary-action {
    background: rgba(255, 250, 240, 0.74);
    border-color: rgba(217, 119, 6, 0.12);
  }

  .timeline-editorial-board {
    position: relative;
    overflow: hidden;
    background: var(--editorial-surface-featured);
    border-color: rgba(255, 251, 235, 0.9);
    box-shadow:
      0 20px 48px rgba(15, 23, 42, 0.08),
      0 2px 10px rgba(15, 23, 42, 0.04),
      inset 0 1px 0 rgba(255, 255, 255, 0.85);
  }

  .timeline-editorial-board::before {
    content: '';
    position: absolute;
    inset: 0;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.22), transparent 28%),
      repeating-linear-gradient(
        135deg,
        rgba(120, 113, 108, 0.018) 0 6px,
        transparent 6px 16px
      );
    pointer-events: none;
  }

  .timeline-editorial-shell {
    --timeline-anchor-width: 6rem;
    position: relative;
    padding: 1.5rem 1.25rem 1.75rem;
  }

  .timeline-rail {
    position: absolute;
    left: calc(1.25rem + var(--timeline-anchor-width) - 0.875rem);
    top: 1.25rem;
    bottom: 1.25rem;
    width: 2px;
    border-radius: 999px;
    background: linear-gradient(180deg, rgba(31, 41, 55, 0.88), rgba(31, 41, 55, 0.08));
    opacity: 0.9;
    pointer-events: none;
  }

  .timeline-entry {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: var(--timeline-anchor-width) minmax(0, 1fr);
    gap: 1rem;
    width: 100%;
    padding: 0.2rem 0;
    text-align: left;
    transition:
      transform 180ms ease,
      filter 180ms ease;
  }

  .timeline-entry + .timeline-entry {
    margin-top: 0.4rem;
  }

  .timeline-entry:hover {
    transform: translateY(-1px);
    filter: saturate(1.02);
  }

  .timeline-entry-anchor {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 0.65rem;
    min-height: 100%;
    padding-top: 0.95rem;
  }

  .timeline-entry-time {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    font-size: 0.82rem;
    letter-spacing: 0.08em;
    color: #57534e;
  }

  .timeline-entry-marker {
    width: 0.8rem;
    height: 0.8rem;
    border-radius: 999px;
    background: #1f2937;
    box-shadow:
      0 0 0 0.32rem rgba(255, 251, 235, 0.96),
      0 0 0 0.4rem rgba(31, 41, 55, 0.08);
    flex-shrink: 0;
    transition:
      transform 180ms ease,
      box-shadow 180ms ease,
      background-color 180ms ease;
  }

  .timeline-entry:hover .timeline-entry-marker,
  .timeline-entry:focus-visible .timeline-entry-marker {
    transform: scale(1.05);
    box-shadow:
      0 0 0 0.32rem rgba(255, 251, 235, 0.98),
      0 0 0 0.5rem rgba(180, 83, 9, 0.12);
  }

  .timeline-entry-marker-featured {
    background: #b45309;
  }

  .timeline-entry-card {
    position: relative;
    border-radius: 1.35rem;
    border: 1px solid rgba(17, 24, 39, 0.08);
    overflow: hidden;
    transition:
      transform 180ms ease,
      border-color 180ms ease,
      box-shadow 180ms ease;
  }

  .timeline-entry:hover .timeline-entry-card,
  .timeline-entry:focus-visible .timeline-entry-card {
    border-color: rgba(180, 83, 9, 0.14);
    box-shadow: 0 16px 36px rgba(15, 23, 42, 0.1);
  }

  .timeline-entry-card-featured {
    display: grid;
    grid-template-columns: minmax(12rem, 16.5rem) minmax(0, 1fr);
    gap: 1rem;
    padding: 0.9rem;
    background: rgba(255, 255, 255, 0.78);
    box-shadow:
      0 16px 36px rgba(15, 23, 42, 0.08),
      inset 0 1px 0 rgba(255, 255, 255, 0.85);
  }

  .timeline-entry-card-compact {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.8rem 1rem;
    align-items: center;
    padding: 1rem 1.05rem;
    background: rgba(255, 255, 255, 0.62);
    backdrop-filter: blur(8px);
  }

  .timeline-featured-media {
    min-width: 0;
  }

  .timeline-featured-image {
    width: 100%;
    aspect-ratio: 1.38;
    border-radius: 1rem;
    object-fit: cover;
    background:
      linear-gradient(135deg, rgba(191, 219, 254, 0.82), rgba(254, 243, 199, 0.9)),
      repeating-linear-gradient(45deg, rgba(255, 255, 255, 0.2) 0 8px, rgba(255, 255, 255, 0.03) 8px 16px);
    border: 1px solid rgba(255, 255, 255, 0.62);
  }

  .timeline-featured-image-placeholder {
    position: relative;
    overflow: hidden;
  }

  .timeline-featured-image-glow {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(circle at 20% 20%, rgba(255, 255, 255, 0.52), transparent 36%),
      linear-gradient(135deg, rgba(191, 219, 254, 0.52), rgba(254, 243, 199, 0.68));
  }

  .timeline-featured-copy {
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.85rem;
  }

  .timeline-entry-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .timeline-entry-meta-featured {
    align-items: flex-start;
    gap: 1rem;
  }

  .timeline-entry-app {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.85rem;
    flex: 1 1 auto;
  }

  .timeline-app-icon {
    width: 2.75rem;
    height: 2.75rem;
    border-radius: 1rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    flex-shrink: 0;
    color: #111827;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.65);
  }

  .timeline-app-icon-lg {
    width: 3.2rem;
    height: 3.2rem;
    border-radius: 1.05rem;
    font-size: 1.5rem;
  }

  .timeline-app-icon-blue {
    background: rgba(219, 234, 254, 0.95);
  }

  .timeline-app-icon-green {
    background: rgba(220, 252, 231, 0.95);
  }

  .timeline-app-icon-yellow {
    background: rgba(254, 249, 195, 0.95);
  }

  .timeline-app-icon-purple {
    background: rgba(237, 233, 254, 0.95);
  }

  .timeline-app-icon-pink {
    background: rgba(252, 231, 243, 0.95);
  }

  .timeline-app-icon-red {
    background: rgba(254, 226, 226, 0.95);
  }

  .timeline-app-icon-gray {
    background: rgba(241, 245, 249, 0.95);
  }

  .timeline-app-icon-image {
    width: 1.9rem;
    height: 1.9rem;
    border-radius: 0.7rem;
  }

  .timeline-app-icon-image-lg {
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 0.8rem;
  }

  .timeline-entry-heading {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .timeline-entry-heading-featured {
    align-items: flex-start;
    gap: 0.45rem;
  }

  .timeline-entry-app-name {
    display: block;
    font-size: 0.98rem;
    font-weight: 600;
    color: #111827;
    letter-spacing: -0.01em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .timeline-entry-category {
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #a16207;
  }

  .timeline-entry-category-pill {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    min-height: 1.5rem;
    max-width: max-content;
    padding: 0.2rem 0.58rem;
    border-radius: 999px;
    border: 1px solid rgba(217, 119, 6, 0.18);
    background: rgba(255, 247, 237, 0.92);
    color: #b45309;
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    line-height: 1;
    text-transform: none;
    white-space: nowrap;
    writing-mode: horizontal-tb;
  }

  .timeline-entry-duration-chip {
    flex-shrink: 0;
    padding: 0.4rem 0.7rem;
    border-radius: 999px;
    background: rgba(255, 247, 237, 0.92);
    color: #9a3412;
    font-size: 0.78rem;
    font-weight: 600;
  }

  .timeline-entry-title {
    min-width: 0;
    color: #1f2937;
    margin: 0;
  }

  .timeline-entry-title-featured {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 1.02rem;
    line-height: 1.55;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .timeline-entry-title-compact {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.92rem;
    color: #57534e;
  }

  .timeline-entry-url {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.83rem;
    color: #78716c;
  }

  .timeline-entry-tail {
    display: inline-flex;
    align-items: center;
    gap: 0.65rem;
    color: #78716c;
    white-space: nowrap;
  }

  .timeline-entry-card-compact-grid {
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas:
      'app app'
      'title meta';
    align-items: start;
  }

  .timeline-entry-app-compact {
    grid-area: app;
  }

  .timeline-entry-card-compact-grid .timeline-entry-title-compact {
    grid-area: title;
  }

  .timeline-entry-tail-compact {
    grid-area: meta;
    justify-self: end;
    align-self: end;
  }

  .timeline-entry-duration {
    font-size: 0.85rem;
    font-weight: 500;
  }

  .timeline-entry-arrow {
    width: 1rem;
    height: 1rem;
    color: #a8a29e;
    flex-shrink: 0;
  }

  .timeline-load-more {
    position: relative;
    padding: 0 1.25rem 1.4rem calc(1.25rem + var(--timeline-anchor-width));
  }

  .timeline-load-more-btn {
    width: 100%;
    min-height: 2.75rem;
    padding: 0.65rem 1rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-size: 0.92rem;
    color: #57534e;
    border-radius: 1rem;
    border: 1px dashed rgba(120, 113, 108, 0.35);
    background: rgba(255, 255, 255, 0.54);
    transition:
      border-style 180ms ease,
      background-color 180ms ease;
  }

  .timeline-load-more-btn:hover:enabled {
    border-style: solid;
    background: rgba(255, 255, 255, 0.72);
  }

  .timeline-load-more-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .timeline-load-more-end {
    color: #a8a29e;
    text-align: center;
    font-size: 0.78rem;
  }

  .timeline-detail-dialog {
    background: var(--editorial-surface-featured);
  }

  .timeline-detail-header {
    background: var(--editorial-surface-subtle);
  }

  :global(.dark) .timeline-summary-copy {
    color: #94a3b8;
  }

  :global(.dark) .timeline-summary-divider {
    color: #475569;
  }

  :global(.dark) .timeline-summary-action {
    background: rgba(51, 65, 85, 0.72);
    border-color: rgba(245, 158, 11, 0.16);
  }

  :global(.dark) .timeline-editorial-board {
    background: var(--editorial-surface-featured);
    border-color: rgba(71, 85, 105, 0.58);
    box-shadow:
      0 24px 54px rgba(2, 6, 23, 0.34),
      inset 0 1px 0 rgba(255, 255, 255, 0.04);
  }

  :global(.dark) .timeline-editorial-board::before {
    background:
      linear-gradient(180deg, rgba(148, 163, 184, 0.06), transparent 28%),
      repeating-linear-gradient(
        135deg,
        rgba(148, 163, 184, 0.015) 0 6px,
        transparent 6px 16px
      );
  }

  :global(.dark) .timeline-rail {
    background: linear-gradient(180deg, rgba(248, 250, 252, 0.84), rgba(148, 163, 184, 0.08));
  }

  :global(.dark) .timeline-entry-time {
    color: #cbd5e1;
  }

  :global(.dark) .timeline-entry-marker {
    background: #e2e8f0;
    box-shadow:
      0 0 0 0.32rem rgba(15, 23, 42, 0.96),
      0 0 0 0.5rem rgba(148, 163, 184, 0.08);
  }

  :global(.dark) .timeline-entry:hover .timeline-entry-marker,
  :global(.dark) .timeline-entry:focus-visible .timeline-entry-marker {
    box-shadow:
      0 0 0 0.32rem rgba(15, 23, 42, 0.98),
      0 0 0 0.55rem rgba(245, 158, 11, 0.16);
  }

  :global(.dark) .timeline-entry-marker-featured {
    background: #fbbf24;
  }

  :global(.dark) .timeline-entry-card {
    border-color: rgba(148, 163, 184, 0.12);
  }

  :global(.dark) .timeline-entry:hover .timeline-entry-card,
  :global(.dark) .timeline-entry:focus-visible .timeline-entry-card {
    border-color: rgba(251, 191, 36, 0.18);
    box-shadow: 0 18px 42px rgba(2, 6, 23, 0.34);
  }

  :global(.dark) .timeline-entry-card-featured {
    background: rgba(15, 23, 42, 0.66);
  }

  :global(.dark) .timeline-entry-card-compact {
    background: rgba(15, 23, 42, 0.54);
  }

  :global(.dark) .timeline-featured-image {
    border-color: rgba(148, 163, 184, 0.12);
  }

  :global(.dark) .timeline-entry-app-name,
  :global(.dark) .timeline-entry-title {
    color: #f8fafc;
  }

  :global(.dark) .timeline-entry-category {
    color: #fbbf24;
  }

  :global(.dark) .timeline-entry-category-pill {
    border-color: rgba(245, 158, 11, 0.22);
    background: rgba(120, 53, 15, 0.28);
    color: #fcd34d;
  }

  :global(.dark) .timeline-entry-title-compact,
  :global(.dark) .timeline-entry-url,
  :global(.dark) .timeline-entry-tail {
    color: #94a3b8;
  }

  :global(.dark) .timeline-entry-duration-chip {
    background: rgba(120, 53, 15, 0.26);
    color: #fdba74;
  }

  :global(.dark) .timeline-app-icon-blue {
    background: rgba(30, 64, 175, 0.34);
  }

  :global(.dark) .timeline-app-icon-green {
    background: rgba(22, 101, 52, 0.34);
  }

  :global(.dark) .timeline-app-icon-yellow {
    background: rgba(133, 77, 14, 0.34);
  }

  :global(.dark) .timeline-app-icon-purple {
    background: rgba(91, 33, 182, 0.34);
  }

  :global(.dark) .timeline-app-icon-pink {
    background: rgba(157, 23, 77, 0.34);
  }

  :global(.dark) .timeline-app-icon-red {
    background: rgba(153, 27, 27, 0.34);
  }

  :global(.dark) .timeline-app-icon-gray {
    background: rgba(51, 65, 85, 0.74);
  }

  :global(.dark) .timeline-load-more-btn {
    color: #cbd5e1;
    border-color: rgba(148, 163, 184, 0.24);
    background: rgba(15, 23, 42, 0.48);
  }

  :global(.dark) .timeline-load-more-btn:hover:enabled {
    background: rgba(15, 23, 42, 0.68);
  }

  :global(.dark) .timeline-load-more-end {
    color: #64748b;
  }

  :global(.dark) .timeline-detail-dialog {
    background: var(--editorial-surface-featured);
  }

  :global(.dark) .timeline-detail-header {
    background: var(--editorial-surface-subtle);
  }

  @media (max-width: 860px) {
    .timeline-entry-card-featured {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 640px) {
    .timeline-summary-strip {
      align-items: flex-start;
      flex-direction: column;
    }

    .timeline-editorial-shell {
      --timeline-anchor-width: 4.8rem;
      padding: 1.1rem 0.85rem 1.35rem;
    }

    .timeline-rail {
      left: calc(0.85rem + var(--timeline-anchor-width) - 0.72rem);
    }

    .timeline-entry {
      gap: 0.7rem;
    }

    .timeline-entry-card-compact-grid {
      grid-template-columns: minmax(0, 1fr);
      grid-template-areas:
        'app'
        'title'
        'meta';
    }

    .timeline-entry-tail-compact {
      justify-self: start;
    }

    .timeline-entry-anchor {
      gap: 0.45rem;
      padding-top: 0.8rem;
    }

    .timeline-entry-time {
      font-size: 0.74rem;
      letter-spacing: 0.05em;
    }

    .timeline-entry-marker {
      width: 0.68rem;
      height: 0.68rem;
    }

    .timeline-entry-card-compact {
      grid-template-columns: 1fr;
    }

    .timeline-entry-tail {
      justify-content: space-between;
    }

    .timeline-load-more {
      padding: 0 0.85rem 1.1rem calc(0.85rem + var(--timeline-anchor-width));
    }
  }
</style>
