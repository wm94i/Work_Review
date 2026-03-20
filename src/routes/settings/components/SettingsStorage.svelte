<script>
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { ask } from '@tauri-apps/plugin-dialog';
  import { cache } from '../../../lib/stores/cache.js';
  import { showToast } from '$lib/stores/toast.js';
  
  export let config;
  export let storageStats = null;
  
  const dispatch = createEventDispatcher();
  let isClearing = false;

  function clearCache() {
    cache.clear();
    showToast('缓存已清理');
    dispatch('clearCache');
  }

  async function clearOldData() {
    const confirmed = await ask('确认删除今天之前的所有活动记录和截图？此操作不可恢复！', {
      title: '确认清理历史数据',
      kind: 'warning',
    });

    if (!confirmed) {
      return;
    }
    
    isClearing = true;
    try {
      const result = await invoke('clear_old_activities');
      showToast(result.message);
      cache.clear();
      dispatch('clearCache');
    } catch (e) {
      showToast('清理失败: ' + e, 'error');
    } finally {
      isClearing = false;
    }
  }

  function handleChange() {
    dispatch('change', config);
  }

  // 计算存储使用百分比
  $: usagePercent = storageStats 
    ? Math.min(Math.round((storageStats.total_size_mb / storageStats.storage_limit_mb) * 100), 100) 
    : 0;

  // 使用量颜色
  $: usageColor = usagePercent > 80 ? 'bg-red-500' : usagePercent > 50 ? 'bg-amber-500' : 'bg-emerald-500';
</script>

<!-- 记录设置 -->
<div class="card p-5 mb-5">
  <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-200 mb-1">记录设置</h3>
  <p class="text-xs text-slate-400 dark:text-slate-500 mb-4">控制活动记录的频率和保留策略</p>
  
  <div class="space-y-5">
    <!-- 轮询间隔 -->
    <div>
      <div class="flex items-center justify-between mb-1.5">
        <label for="screenshot-interval" class="text-sm font-medium text-slate-700 dark:text-slate-300">活动轮询间隔</label>
        <span class="text-sm font-mono text-primary-600 dark:text-primary-400">{config.screenshot_interval}秒</span>
      </div>
      <input
        id="screenshot-interval"
        type="range"
        bind:value={config.screenshot_interval}
        on:change={handleChange}
        min="10"
        max="120"
        step="5"
        class="w-full h-1.5 bg-slate-200 dark:bg-slate-700 rounded-full appearance-none cursor-pointer accent-primary-500"
      />
      <div class="flex justify-between text-xs text-slate-400 mt-1">
        <span>10秒（更精确）</span>
        <span>120秒（更省电）</span>
      </div>
      <p class="text-xs text-slate-400 mt-2">每隔此时长检测一次当前活动窗口并执行 OCR</p>
    </div>

    <!-- 数据保留 -->
    <div>
      <div class="flex items-center justify-between mb-1.5">
        <label for="retention-days" class="text-sm font-medium text-slate-700 dark:text-slate-300">数据保留天数</label>
        <span class="text-sm font-mono text-primary-600 dark:text-primary-400">{config.storage.screenshot_retention_days}天</span>
      </div>
      <input
        id="retention-days"
        type="range"
        bind:value={config.storage.screenshot_retention_days}
        on:change={() => {
          config.storage.metadata_retention_days = config.storage.screenshot_retention_days;
          handleChange();
        }}
        min="1"
        max="90"
        step="1"
        class="w-full h-1.5 bg-slate-200 dark:bg-slate-700 rounded-full appearance-none cursor-pointer accent-primary-500"
      />
      <div class="flex justify-between text-xs text-slate-400 mt-1">
        <span>1天</span>
        <span>90天</span>
      </div>
      <p class="text-xs text-slate-400 mt-2">超过此天数的活动记录和截图将被自动清理</p>
    </div>
  </div>
</div>

<!-- 存储统计 -->
{#if storageStats}
<div class="card p-5 mb-5">
  <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-200 mb-4">存储使用</h3>
  
  <!-- 存储进度条 -->
  <div class="mb-5">
    <div class="flex items-end justify-between mb-2">
      <div>
        <span class="text-2xl font-bold text-slate-800 dark:text-white">{storageStats.total_size_mb}</span>
        <span class="text-sm text-slate-500"> / {storageStats.storage_limit_mb} MB</span>
      </div>
      <span class="text-sm font-medium {usagePercent > 80 ? 'text-red-500' : 'text-slate-500'}">{usagePercent}%</span>
    </div>
    <div class="w-full h-2.5 bg-slate-100 dark:bg-slate-700 rounded-full overflow-hidden">
      <div 
        class="h-full rounded-full transition-all duration-500 {usageColor}"
        style="width: {usagePercent}%"
      ></div>
    </div>
  </div>

  <!-- 统计卡片 -->
  <div class="grid grid-cols-3 gap-3">
    <div class="text-center p-3 bg-slate-50 dark:bg-slate-700/30 rounded-xl">
      <p class="text-xl font-bold text-slate-800 dark:text-white">{storageStats.total_files}</p>
      <p class="text-xs text-slate-500 mt-0.5">截图数</p>
    </div>
    <div class="text-center p-3 bg-slate-50 dark:bg-slate-700/30 rounded-xl">
      <p class="text-xl font-bold text-slate-800 dark:text-white">{storageStats.total_size_mb} MB</p>
      <p class="text-xs text-slate-500 mt-0.5">已用空间</p>
    </div>
    <div class="text-center p-3 bg-slate-50 dark:bg-slate-700/30 rounded-xl">
      <p class="text-xl font-bold text-slate-800 dark:text-white">{storageStats.retention_days} 天</p>
      <p class="text-xs text-slate-500 mt-0.5">保留期限</p>
    </div>
  </div>
</div>

<!-- 数据管理 -->
<div class="card p-5">
  <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-200 mb-4">数据管理</h3>
  <div class="space-y-3">
    <!-- 清理缓存 -->
    <div class="flex items-center justify-between p-3 bg-slate-50 dark:bg-slate-700/30 rounded-xl">
      <div>
        <p class="text-sm font-medium text-slate-700 dark:text-slate-300">清理页面缓存</p>
        <p class="text-xs text-slate-400 mt-0.5">解决数据显示异常问题，不影响已保存的数据</p>
      </div>
      <button
        on:click={clearCache}
        class="px-3 py-1.5 text-xs font-medium bg-slate-200 hover:bg-slate-300 dark:bg-slate-600 dark:hover:bg-slate-500 text-slate-700 dark:text-slate-300 rounded-lg transition-colors"
      >
        清理缓存
      </button>
    </div>
    
    <!-- 清理历史 -->
    <div class="flex items-center justify-between p-3 bg-red-50/50 dark:bg-red-900/10 rounded-xl border border-red-100 dark:border-red-900/30">
      <div>
        <p class="text-sm font-medium text-red-600 dark:text-red-400">清理历史数据</p>
        <p class="text-xs text-slate-400 mt-0.5">删除今天之前的所有活动记录和截图，不可恢复</p>
      </div>
      <button
        on:click={clearOldData}
        disabled={isClearing}
        class="px-3 py-1.5 text-xs font-medium bg-red-100 hover:bg-red-200 dark:bg-red-900/30 dark:hover:bg-red-900/50 text-red-700 dark:text-red-400 rounded-lg transition-colors disabled:opacity-50"
      >
        {#if isClearing}
          清理中...
        {:else}
          清理历史
        {/if}
      </button>
    </div>
  </div>
</div>
{/if}
