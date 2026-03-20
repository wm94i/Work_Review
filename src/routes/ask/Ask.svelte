<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { marked } from 'marked';

  function todayString() {
    const now = new Date();
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
  }

  function daysAgoString(days) {
    const date = new Date();
    date.setDate(date.getDate() - days);
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
  }

  let query = '';
  let dateFrom = daysAgoString(7);
  let dateTo = todayString();
  let searchResults = [];
  let answer = null;
  let error = null;
  let searching = false;
  let asking = false;
  let config = null;

  onMount(async () => {
    try {
      config = await invoke('get_config');
    } catch (e) {
      console.warn('加载配置失败:', e);
    }
  });

  function formatDuration(seconds) {
    if (!seconds || seconds <= 0) return '';
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    if (hours > 0) return `${hours}小时${minutes}分钟`;
    if (minutes > 0) return `${minutes}分钟`;
    return `${secs}秒`;
  }

  function sourceLabel(sourceType) {
    const labels = {
      activity: '活动记录',
      hourly_summary: '小时摘要',
      daily_report: '日报',
    };
    return labels[sourceType] || sourceType;
  }

  function applyRangePreset(days) {
    dateFrom = daysAgoString(days);
    dateTo = todayString();
  }

  function normalizedDate(value) {
    return value && value.trim() ? value : null;
  }

  async function handleSearch() {
    if (!query.trim()) return;
    searching = true;
    error = null;
    answer = null;

    try {
      searchResults = await invoke('search_memory', {
        query,
        dateFrom: normalizedDate(dateFrom),
        dateTo: normalizedDate(dateTo),
        limit: 20,
      });
    } catch (e) {
      error = e.toString();
      searchResults = [];
    } finally {
      searching = false;
    }
  }

  async function handleAsk() {
    if (!query.trim()) return;
    asking = true;
    error = null;

    try {
      answer = await invoke('ask_memory', {
        question: query,
        dateFrom: normalizedDate(dateFrom),
        dateTo: normalizedDate(dateTo),
      });
      searchResults = answer?.references || [];
    } catch (e) {
      error = e.toString();
      answer = null;
    } finally {
      asking = false;
    }
  }

  $: textModelConfigured = !!(config?.text_model?.endpoint && config?.text_model?.model);
</script>

<div class="page-shell">
  <div class="page-header">
    <div class="page-title-group">
      <div class="page-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M8 10h8M8 14h5m-7 6h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2Zm0-16h12" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>工作记忆</h2>
        <p>搜索活动、OCR、网页、小时摘要和日报；可切到 AI 生成自然语言回答。</p>
      </div>
    </div>
    <div class="page-status-chip text-indigo-600 dark:text-indigo-400">
      <span class="w-1.5 h-1.5 bg-indigo-500 rounded-full"></span>
      {#if textModelConfigured}基础 + AI{:else}基础检索{/if}
    </div>
  </div>

  <div class="page-card mb-4">
    <div class="grid lg:grid-cols-[minmax(0,1fr)_180px_180px] gap-3 mb-3">
      <div>
        <label for="memory-query" class="settings-label mb-1.5">问题 / 关键词</label>
        <input
          id="memory-query"
          bind:value={query}
          class="page-control-input w-full"
          placeholder="例如：我上周主要在做什么？ / deepseek / PRD / github.com"
          on:keydown={(e) => e.key === 'Enter' && !e.shiftKey && handleAsk()}
        />
      </div>
      <div>
        <label for="memory-date-from" class="settings-label mb-1.5">开始日期</label>
        <input id="memory-date-from" bind:value={dateFrom} type="date" class="page-control-input w-full" />
      </div>
      <div>
        <label for="memory-date-to" class="settings-label mb-1.5">结束日期</label>
        <input id="memory-date-to" bind:value={dateTo} type="date" class="page-control-input w-full" />
      </div>
    </div>

    <div class="flex flex-wrap items-center gap-2 mb-4">
      <button class="page-control-btn" on:click={() => applyRangePreset(0)}>今天</button>
      <button class="page-control-btn" on:click={() => applyRangePreset(7)}>近 7 天</button>
      <button class="page-control-btn" on:click={() => applyRangePreset(30)}>近 30 天</button>
      <button class="page-action-secondary" on:click={handleSearch} disabled={searching || asking || !query.trim()}>
        {#if searching}搜索中...{:else}基础检索{/if}
      </button>
      <button class="page-action-brand" on:click={handleAsk} disabled={searching || asking || !query.trim()}>
        {#if asking}回答中...{:else}AI 问答{/if}
      </button>
    </div>

    {#if !textModelConfigured}
      <div class="page-banner-warning">
        <div>
          <p class="font-semibold">当前未配置文本模型</p>
          <p class="text-sm mt-1">AI 问答会自动回退到基础回答模式；基础检索仍可正常使用。</p>
        </div>
      </div>
    {/if}

    {#if error}
      <div class="page-banner-error mt-4">
        <div>
          <p class="font-semibold">工作记忆请求失败</p>
          <p class="text-sm mt-1">{error}</p>
        </div>
      </div>
    {/if}
  </div>

  {#if answer}
    <div class="page-card mb-4">
      <div class="flex flex-wrap items-center gap-2 mb-3">
        <h3 class="page-section-title !mb-0">回答</h3>
        <span class="{answer.usedAi ? 'page-inline-chip-brand' : 'page-inline-chip-muted'}">
          {#if answer.usedAi}
            AI 增强{#if answer.modelName} · {answer.modelName}{/if}
          {:else}
            基础回答
          {/if}
        </span>
      </div>
      <div class="prose prose-slate max-w-none dark:prose-invert prose-p:leading-7 prose-li:leading-7">
        {@html marked.parse(answer.answer || '')}
      </div>
    </div>
  {/if}

  <div class="page-card">
    <div class="flex items-center justify-between gap-3 mb-3">
      <h3 class="page-section-title !mb-0">相关记忆</h3>
      <span class="page-inline-chip-muted">{searchResults.length} 条</span>
    </div>

    {#if searchResults.length === 0}
      <div class="flex items-center justify-center py-8">
        <p class="empty-state-copy">输入问题后可以先做基础检索，或直接点 AI 问答。</p>
      </div>
    {:else}
      <div class="space-y-3">
        {#each searchResults as item}
          <div class="rounded-xl border border-slate-100 dark:border-slate-700 bg-white dark:bg-slate-800/60 p-4">
            <div class="flex flex-wrap items-center gap-2 mb-2">
              <span class="page-inline-chip-muted">{sourceLabel(item.sourceType)}</span>
              <span class="page-inline-chip-muted">{item.date}</span>
              {#if item.appName}
                <span class="page-inline-chip-muted">{item.appName}</span>
              {/if}
              {#if item.duration}
                <span class="page-inline-chip-muted">{formatDuration(item.duration)}</span>
              {/if}
              <span class="page-inline-chip-muted">相关度 {item.score}</span>
            </div>

            <h4 class="font-semibold text-slate-800 dark:text-slate-100 mb-1.5">{item.title}</h4>

            {#if item.browserUrl}
              <p class="text-xs text-slate-400 break-all mb-1.5">{item.browserUrl}</p>
            {/if}

            {#if item.excerpt}
              <p class="text-sm leading-6 text-slate-600 dark:text-slate-300">{item.excerpt}</p>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
