<script>
  import { onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { formatDurationLocalized, locale, t } from '$lib/i18n/index.js';
  import { appIconStore, getIconCacheKey, preloadAppIcons } from '../stores/iconCache.js';
  import { resolveAppIconSrc } from '../utils/appVisuals.js';

  export let data = [];

  // 订阅全局图标缓存
  let appIcons = {};
  const unsubIcons = appIconStore.subscribe(v => appIcons = v);
  onDestroy(() => unsubIcons());

  // 展开/收起状态
  const DEFAULT_COUNT = 8;
  let expanded = false;
  $: currentLocale = $locale;

  // 格式化时长
  function formatDuration(seconds) {
    currentLocale;
    return formatDurationLocalized(seconds, { compact: true });
  }

  // 颜色列表
  const colors = [
    '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6',
    '#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1',
  ];

  // 数据变化时预加载图标
  $: if (data) {
    preloadAppIcons(
      displayApps.map(a => ({
        appName: a.app_name,
        executablePath: a.executable_path,
      })),
      invoke
    );
  }

  // 展开时显示全部，收起时显示前 8
  $: displayApps = expanded ? data : data.slice(0, DEFAULT_COUNT);
  $: hasMore = data.length > DEFAULT_COUNT;
  $: maxDuration = displayApps.length > 0 ? Math.max(...displayApps.map(a => a.duration)) : 1;
</script>

<div class="space-y-2" data-locale={currentLocale}>
  {#each displayApps as app, i}
    {@const iconSrc = resolveAppIconSrc(
      app.app_name,
      appIcons[getIconCacheKey({ appName: app.app_name, executablePath: app.executable_path })]
    )}
    <div class="flex items-center gap-2.5">
      <!-- 应用图标或序号 -->
      <div class="w-6 h-6 flex-shrink-0 flex items-center justify-center">
        {#if iconSrc}
          <img src={iconSrc} alt="" class="w-5 h-5 rounded-md object-cover" />
        {:else}
          <span class="w-5 h-5 flex items-center justify-center rounded bg-slate-100 dark:bg-slate-700 text-xs text-slate-500">{i + 1}</span>
        {/if}
      </div>
      <!-- 应用名 -->
      <span class="w-24 text-xs text-slate-600 dark:text-slate-300 truncate flex-shrink-0">{app.app_name}</span>
      <!-- 进度条 -->
      <div class="flex-1 h-4 bg-slate-100 dark:bg-slate-700/50 rounded-full overflow-hidden">
        <div
          class="h-full rounded-full transition-all duration-500"
          style="width: {Math.max((app.duration / maxDuration) * 100, 2)}%; background-color: {colors[i % colors.length]}; opacity: 0.8"
        ></div>
      </div>
      <!-- 时长 -->
      <span class="text-xs text-slate-500 dark:text-slate-400 w-14 text-right flex-shrink-0">{formatDuration(app.duration)}</span>
    </div>
  {/each}

  {#if hasMore}
    <button
      class="w-full text-center text-xs text-slate-400 hover:text-primary-500 dark:text-slate-500 dark:hover:text-primary-400 py-1 transition-colors"
      on:click={() => expanded = !expanded}
    >
      {expanded ? t('overview.appUsageCollapse') : t('overview.appUsageExpandAll', { count: data.length })}
    </button>
  {/if}
</div>
