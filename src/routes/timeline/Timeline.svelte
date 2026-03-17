<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-shell';
  import { cache } from '../../lib/stores/cache.js';

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
  let showSummary = false;
  let currentTime = new Date();
  let clockInterval;

  // LRU 缓存：防止长时间运行内存无限增长
  // 缩略图 ~80KB/条，60 条 ≈ 5MB；高清图 ~300KB/条，20 条 ≈ 6MB
  const THUMBNAIL_CACHE_LIMIT = 60;
  const FULLIMAGE_CACHE_LIMIT = 20;
  let thumbnailCache = {};
  let thumbnailKeys = [];   // 插入顺序追踪，用于淘汰最旧条目
  let fullImageCache = {};
  let fullImageKeys = [];

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

  // 应用图标缓存（localStorage 持久化，上限 100 个应用）
  const ICON_CACHE_KEY = 'work_review_app_icons';
  const ICON_CACHE_LIMIT = 100;
  let appIconCache = {};
  
  // 从 localStorage 恢复缓存
  try {
    const cached = localStorage.getItem(ICON_CACHE_KEY);
    if (cached) {
      appIconCache = JSON.parse(cached);
    }
  } catch (e) {
    console.warn('恢复图标缓存失败:', e);
  }
  
  // 保存图标缓存到 localStorage
  function saveIconCache() {
    try {
      localStorage.setItem(ICON_CACHE_KEY, JSON.stringify(appIconCache));
    } catch (e) {
      console.warn('保存图标缓存失败:', e);
    }
  }

  // 加载应用图标（带条目上限，超出时淘汰最旧的缓存）
  async function loadAppIcon(appName) {
    if (appIconCache[appName] !== undefined) {
      return appIconCache[appName];
    }
    try {
      const base64 = await invoke('get_app_icon', { appName });
      if (base64 && base64.length > 100) {
        appIconCache = { ...appIconCache, [appName]: base64 };
      } else {
        appIconCache = { ...appIconCache, [appName]: null };
      }
      // 超出上限时淘汰最旧条目
      const keys = Object.keys(appIconCache);
      if (keys.length > ICON_CACHE_LIMIT) {
        const toRemove = keys.slice(0, keys.length - ICON_CACHE_LIMIT);
        toRemove.forEach(k => delete appIconCache[k]);
      }
      saveIconCache();
      return appIconCache[appName];
    } catch (e) {
      console.warn('加载应用图标失败:', appName, e);
      appIconCache = { ...appIconCache, [appName]: null };
      return null;
    }
  }

  // 分类名称和颜色
  const categoryInfo = {
    development: { name: '开发工具', color: 'blue', icon: '⚡' },
    browser: { name: '浏览器', color: 'green', icon: '🌐' },
    communication: { name: '通讯协作', color: 'yellow', icon: '💬' },
    office: { name: '办公软件', color: 'purple', icon: '📝' },
    design: { name: '设计工具', color: 'pink', icon: '🎨' },
    entertainment: { name: '娱乐摸鱼', color: 'red', icon: '🎮' },
    other: { name: '其他', color: 'gray', icon: '📁' },
  };

  // 格式化时间
  function formatTime(timestamp) {
    return new Date(timestamp * 1000).toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  }

  // 格式化时长
  function formatDuration(seconds) {
    if (seconds < 60) return `${seconds}秒`;
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    if (minutes < 60) {
      return secs > 0 ? `${minutes}分${secs}秒` : `${minutes}分钟`;
    }
    const hours = Math.floor(minutes / 60);
    const mins = minutes % 60;
    return mins > 0 ? `${hours}小时${mins}分` : `${hours}小时`;
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
        const url = new URL(browserUrl);
        return url.hostname;
      } catch {
        return browserUrl.substring(0, 40);
      }
    }
    
    // 完全无信息
    return `${appName} 使用中`;
  }

  // 加载缩略图（列表用，400px），使用 LRU 缓存控制内存
  async function loadThumbnail(screenshotPath) {
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
      
      // 前端去重保障：按 app_name + browser_url 合并（防止后端 GROUP BY 未生效的情况）
      const mergedMap = new Map();
      for (const activity of activitiesData) {
        const key = `${activity.app_name}|${activity.browser_url || ''}`;
        if (mergedMap.has(key)) {
          const existing = mergedMap.get(key);
          // 合并：保留最新时间戳、累加时长
          existing.duration = (existing.duration || 0) + (activity.duration || 0);
          if (activity.timestamp > existing.timestamp) {
            existing.timestamp = activity.timestamp;
            existing.screenshot_path = activity.screenshot_path;
            existing.window_title = activity.window_title;
          }
        } else {
          mergedMap.set(key, { ...activity });
        }
      }
      // 按时间戳降序排序
      activities = Array.from(mergedMap.values()).sort((a, b) => b.timestamp - a.timestamp);
      
      hourlySummaries = summariesData;
      offset = activities.length;
      hasMore = activitiesData.length >= PAGE_SIZE;
      
      // 保存到缓存（使用去重后的数据）
      cache.setTimeline(selectedDate, activities, summariesData);
      
      // 预加载缩略图
      activities.forEach(a => loadThumbnail(a.screenshot_path));
      
      // 后台预加载前 6 张高清图（避免点击时等待）
      activities.slice(0, 6).forEach(a => loadFullImage(a.screenshot_path));
      
      // 预加载应用图标（获取唯一应用名并批量加载）
      const uniqueAppNames = [...new Set(activities.map(a => a.app_name))];
      uniqueAppNames.forEach(appName => loadAppIcon(appName));
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
    // 核心逻辑：后端已做合并，前端只需按 id 更新或去重
    unlisten = await listen('screenshot-taken', (event) => {
      if (isToday) {
        const newActivity = event.payload;
        const isBrowser = newActivity.browser_url && newActivity.browser_url.length > 0;
        
        // 1. 优先按 id 查找是否已存在（后端合并场景）
        const existingById = activities.findIndex(a => a.id === newActivity.id);
        if (existingById >= 0) {
          // 原地更新：后端已合并，前端同步更新
          activities = activities.map(a => a.id === newActivity.id ? newActivity : a);
          cache.clear();
          return;
        }
        
        // 2. 对于非浏览器应用，按 app_name 查找并替换（实时合并）
        if (!isBrowser) {
          const existingByApp = activities.findIndex(a => 
            a.app_name === newActivity.app_name && 
            (!a.browser_url || a.browser_url.length === 0)
          );
          if (existingByApp >= 0) {
            // 替换旧记录（后端的 duration 已是合并后的值，无需前端累加）
            activities = activities.map((a, i) => i === existingByApp ? newActivity : a);
            cache.clear();
            return;
          }
        }
        
        // 3. 浏览器应用按 URL 查找合并
        if (isBrowser) {
          const existingByUrl = activities.findIndex(a => 
            a.browser_url === newActivity.browser_url
          );
          if (existingByUrl >= 0) {
            // 替换旧记录
            activities = activities.map((a, i) => i === existingByUrl ? newActivity : a);
            cache.clear();
            return;
          }
        }
        
        // 4. 真正的新记录：添加到开头
        activities = [newActivity, ...activities];
        cache.clear();
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (clockInterval) clearInterval(clockInterval);
  });
</script>

<div class="px-5 pt-1 pb-5 animate-fadeIn">
  <!-- 页面标题 -->
  <div class="flex items-center justify-between mb-5">
    <div>
      <h2 class="text-xl font-bold text-slate-800 dark:text-white">时间线</h2>
      <p class="text-sm text-slate-400 dark:text-slate-500 mt-0.5">
        活动记录
        {#if isToday}
          <span class="ml-1.5 inline-flex items-center gap-1.5">
            <span class="w-1.5 h-1.5 bg-emerald-500 rounded-full animate-pulse"></span>
            <span class="font-mono text-xs text-emerald-600 dark:text-emerald-400">{currentTime.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })}</span>
          </span>
        {/if}
      </p>
    </div>
    <div class="flex items-center gap-3">
      <input
        type="date"
        bind:value={selectedDate}
        class="px-3 py-2 text-sm rounded-xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
      />
      <button class="p-2 rounded-xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors" on:click={loadTimeline} title="刷新">
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
    <div class="card p-6 text-center">
      <p class="text-red-500">加载失败: {error}</p>
      <button class="btn btn-primary mt-4" on:click={loadTimeline}>重试</button>
    </div>
  {:else if activities.length === 0}
    <div class="p-12 rounded-2xl bg-white dark:bg-slate-800/80 border border-slate-100 dark:border-slate-700/60 text-center">
      <div class="w-14 h-14 rounded-2xl bg-slate-50 dark:bg-slate-700/50 flex items-center justify-center mx-auto mb-4">
        <span class="text-2xl">📝</span>
      </div>
      <p class="text-slate-400 text-sm">该日期暂无活动记录</p>
    </div>
  {:else}
    <!-- 统计摘要 -->
    <div class="mb-4 flex items-center justify-between">
      <div class="flex items-center gap-3 text-sm text-slate-500 dark:text-slate-400">
        <span>{isToday ? '今日' : selectedDate} {activities.length} 条记录</span>
        <span class="text-slate-300 dark:text-slate-600">|</span>
        <span>00:00 - {formatTime(activities[0].timestamp)}</span>
      </div>
      
      <!-- 时段摘要链接 -->
        <a
          href="#/timeline/summary"
          class="px-3 py-1.5 text-xs rounded-lg transition-colors flex items-center gap-1.5 bg-slate-100 text-slate-600 hover:bg-slate-200 dark:bg-slate-700 dark:text-slate-300 hover:text-primary-600 dark:hover:text-primary-400"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
          </svg>
          时段摘要
          {#if hourlySummaries.length > 0}
            <span class="px-1.5 py-0.5 text-xs bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 rounded-full">{hourlySummaries.length}</span>
          {/if}
          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
        </a>
    </div>
    
    <div class="card overflow-hidden">
      <!-- 时间线列表 -->
      <div class="divide-y divide-slate-200 dark:divide-slate-700">
        {#each activities as activity, i}
          {@const info = categoryInfo[activity.category] || categoryInfo.other}
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
              {#if appIconCache[activity.app_name]}
                <img src={`data:image/png;base64,${appIconCache[activity.app_name]}`} 
                     alt={activity.app_name} 
                     class="w-7 h-7 app-icon" />
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
            class="w-full py-2 flex items-center justify-center gap-2 text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-700/50 rounded-lg transition-colors border border-dashed border-slate-300 dark:border-slate-600 hover:border-solid disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {#if loadingMore}
              <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-slate-500"></div>
              加载中...
            {:else}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
              </svg>
              加载更多
            {/if}
          </button>
        </div>
      {:else if activities.length > 0}
        <div class="p-4 text-center text-xs text-slate-400 border-t border-slate-200 dark:border-slate-700">
          没有更多记录了
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- 活动详情弹窗 -->
{#if selectedActivity}
  {@const info = categoryInfo[selectedActivity.category] || categoryInfo.other}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
    role="button"
    tabindex="0"
    on:click={closeDetail}
    on:keydown={(e) => e.key === 'Escape' && closeDetail()}
  >
    <div class="bg-white dark:bg-slate-800 rounded-xl shadow-xl max-w-3xl w-full max-h-[90vh] overflow-auto" role="dialog" aria-modal="true" on:click|stopPropagation on:keydown|stopPropagation>
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
              {#if appIconCache[selectedActivity.app_name]}
                <img src={`data:image/png;base64,${appIconCache[selectedActivity.app_name]}`} 
                     alt={selectedActivity.app_name} 
                     class="w-9 h-9 app-icon" />
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
        <!-- 截图预览 -->
        <div>
          <span class="text-sm font-medium text-slate-500 dark:text-slate-400">屏幕截图</span>
          <!-- 容器居中对齐，避免图片尺寸小时产生大面积空白 -->
          <div class="mt-2 rounded-lg overflow-hidden bg-slate-100 dark:bg-slate-700 flex items-center justify-center min-h-[120px]">
            {#if selectedActivity.thumbnailLoading}
              <div class="py-12 flex items-center justify-center">
                <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
              </div>
            {:else if selectedActivity.thumbnail}
              <!-- max-h 限制高度防止超高图片撑开弹窗，object-contain 保持比例居中 -->
              <img src={selectedActivity.thumbnail} alt="屏幕截图" class="max-w-full max-h-96 object-contain" />
            {:else}
              <div class="py-12 flex items-center justify-center text-slate-400">
                <span>截图加载失败</span>
              </div>
            {/if}
          </div>
        </div>

        <div>
          <span class="text-sm font-medium text-slate-500 dark:text-slate-400">窗口标题</span>
          <!-- 字号略大，提升可读性 -->
          <p class="text-base text-slate-800 dark:text-white mt-1 break-all leading-relaxed">{selectedActivity.window_title || '无标题'}</p>
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <span class="text-sm font-medium text-slate-500 dark:text-slate-400">记录时间</span>
            <p class="text-base text-slate-800 dark:text-white mt-1 font-mono">{formatTime(selectedActivity.timestamp)}</p>
          </div>
          <div>
            <span class="text-sm font-medium text-slate-500 dark:text-slate-400">持续时长</span>
            <p class="text-base text-slate-800 dark:text-white mt-1">{formatDuration(selectedActivity.duration)}</p>
          </div>
        </div>
        {#if selectedActivity.browser_url}
          <div>
            <span class="text-sm font-medium text-slate-500 dark:text-slate-400">访问网址</span>
            <button 
              on:click={() => openUrl(selectedActivity.browser_url)}
              class="text-primary-600 dark:text-primary-400 mt-1 text-sm hover:underline break-all block text-left cursor-pointer"
            >
              {selectedActivity.browser_url}
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
