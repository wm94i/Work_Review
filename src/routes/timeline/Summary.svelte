<script>
  import { invoke } from '@tauri-apps/api/core';
  import { link } from 'svelte-spa-router';

  function getLocalDateString() {
    const now = new Date();
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
  }

  let summaries = [];
  let loading = true;
  let error = null;
  let selectedDate = getLocalDateString();
  let lastLoadedDate = null;

  // 格式化摘要内容：按逗号分隔成多行
  function formatSummary(text) {
    if (!text) return [];
    // 按中文逗号和句号分割
    return text.split(/[，。,]/).filter(s => s.trim().length > 0);
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

<div class="p-6 animate-fadeIn">
  <!-- 页面头部 -->
  <div class="flex items-center justify-between mb-6">
    <div class="flex items-center gap-3">
      <a href="/timeline" use:link class="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-700">
        <svg class="w-5 h-5 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </a>
      <h2 class="text-lg font-semibold text-slate-800 dark:text-white">时段摘要</h2>
    </div>
    
    <input
      type="date"
      bind:value={selectedDate}
      max={getLocalDateString()}
      class="px-3 py-1.5 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-800 dark:text-white"
    />
  </div>

  {#if loading}
    <div class="flex justify-center py-8">
      <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-primary-500"></div>
    </div>
  {:else if error}
    <div class="card p-4 text-center">
      <p class="text-red-500 text-sm">{error}</p>
    </div>
  {:else if summaries.length === 0}
    <div class="card p-6 text-center">
      <span class="text-2xl">📊</span>
      <p class="text-slate-500 dark:text-slate-400 text-sm mt-2">暂无数据</p>
    </div>
  {:else}
    <div class="space-y-3">
      {#each summaries as summary}
        <div class="card p-4">
          <div class="flex gap-4">
            <!-- 时间 -->
            <div class="w-14 flex-shrink-0 text-center">
              <div class="text-lg font-bold text-primary-600 dark:text-primary-400">
                {String(summary.hour).padStart(2, '0')}:00
              </div>
              <div class="text-xs text-slate-400">
                {Math.round(summary.total_duration / 60)}分钟
              </div>
            </div>
            
            <!-- 摘要内容 -->
            <div class="flex-1 min-w-0">
              <ul class="text-slate-700 dark:text-slate-200 text-sm space-y-1">
                {#each formatSummary(summary.summary) as item}
                  <li class="flex items-start gap-2">
                    <span class="text-primary-500 mt-0.5">•</span>
                    <span>{item}</span>
                  </li>
                {/each}
              </ul>
              {#if summary.main_apps}
                <div class="flex flex-wrap gap-1 mt-2">
                  {#each summary.main_apps.split(', ').slice(0, 4) as app}
                    <span class="px-2 py-0.5 text-xs bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-300 rounded">
                      {app}
                    </span>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
