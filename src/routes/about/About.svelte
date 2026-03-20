<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-shell';
  import { getVersion } from '@tauri-apps/api/app';
  import { runUpdateFlow } from '$lib/utils/updater.js';

  let appVersion = '';
  let dataDir = '';
  
  let isCheckingUpdate = false;
  let updateStatus = '';

  onMount(async () => {
    try {
      appVersion = await getVersion();
      dataDir = await invoke('get_data_dir');
    } catch (e) {
      console.error('初始化失败:', e);
      appVersion = '1.0.0';
    }
  });

  async function openGitHub() {
    // 使用正确的仓库名（大小写一致）
    await open('https://github.com/wm94i/Work_Review');
  }
  // 通过后端命令直接调用系统文件管理器打开数据目录
  // 绕过 plugin-shell 对本地路径的兼容性问题
  async function openDataDir() {
    try {
      await invoke('open_data_dir');
    } catch (e) {
      console.error('打开目录失败:', e);
    }
  }

  // 检查更新
  async function checkForUpdates() {
    if (isCheckingUpdate) return;
    
    isCheckingUpdate = true;
    updateStatus = '正在检查更新...';

    await runUpdateFlow({
      onStatusChange: (status) => {
        updateStatus = status;
      },
    });

    isCheckingUpdate = false;
    if (updateStatus) {
      setTimeout(() => updateStatus = '', 3000);
    }
  }
</script>

<div class="h-full flex flex-col items-center justify-center p-8 animate-fadeIn">
  <div class="w-full max-w-sm text-center">
    <!-- Logo -->
    <div class="inline-flex items-center justify-center w-20 h-20 rounded-2xl shadow-2xl shadow-primary-500/20 mb-5 transform hover:scale-105 transition-transform overflow-hidden">
      <img src="/icons/256x256.png" alt="Work Review" class="w-full h-full object-cover" />
    </div>

    <h1 class="text-2xl font-bold text-slate-800 dark:text-white mb-1">Work Review</h1>
    <p class="text-slate-500 dark:text-slate-400 text-sm mb-3">个人工作回顾与 AI 日报助手</p>

    <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-slate-100 dark:bg-slate-800 text-xs text-slate-500 dark:text-slate-400 mb-8">
      <span class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
      v{appVersion}
    </div>

    <!-- 操作按钮区 -->
    <div class="flex flex-col gap-3 mb-8">
      <!-- 第一排并排按钮 -->
      <div class="flex gap-3">
        <button on:click={openGitHub} class="flex-1 flex items-center justify-center gap-2 px-5 py-3 text-sm font-medium rounded-xl bg-indigo-500 hover:bg-indigo-600 text-white transition-all shadow-sm">
          <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
          GitHub
        </button>
        <button on:click={openDataDir} class="flex-1 flex items-center justify-center gap-2 px-5 py-3 text-sm font-medium rounded-xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 shadow-sm transition-all text-nowrap">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
          数据目录
        </button>
      </div>
      
      <!-- 第二排：检查更新 -->
      <div class="flex flex-col w-full gap-1.5 mt-2">
        <button
          on:click={checkForUpdates}
          disabled={isCheckingUpdate}
          class="w-full flex items-center justify-center gap-2 px-5 py-3 text-sm font-medium rounded-xl bg-white dark:bg-slate-800 border-2 border-slate-200 dark:border-slate-700 text-emerald-600 dark:text-emerald-400 hover:border-emerald-500 hover:text-emerald-700 dark:hover:border-emerald-500/50 dark:hover:text-emerald-300 shadow-sm transition-all disabled:opacity-50 disabled:cursor-wait"
        >
          {#if isCheckingUpdate}
            <svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-emerald-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            正在更新...
          {:else}
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>
            立即更新
          {/if}
        </button>
        {#if updateStatus}
          <span class="text-xs text-slate-500 dark:text-slate-400 transition-opacity mt-1">{updateStatus}</span>
        {/if}
      </div>
    </div>


    <!-- 技术栈 -->
    <div class="flex flex-wrap justify-center gap-2 mb-8">
      <span class="px-2.5 py-1 rounded-lg bg-orange-50 dark:bg-orange-900/20 text-orange-600 dark:text-orange-400 text-xs border border-orange-200/50 dark:border-orange-800/30">Tauri 2</span>
      <span class="px-2.5 py-1 rounded-lg bg-orange-50 dark:bg-orange-900/20 text-orange-600 dark:text-orange-400 text-xs border border-orange-200/50 dark:border-orange-800/30">Svelte</span>
      <span class="px-2.5 py-1 rounded-lg bg-orange-50 dark:bg-orange-900/20 text-orange-600 dark:text-orange-400 text-xs border border-orange-200/50 dark:border-orange-800/30">Rust</span>
      <span class="px-2.5 py-1 rounded-lg bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 text-xs border border-blue-200/50 dark:border-blue-800/30">SQLite</span>
    </div>
  </div>

  <p class="text-center text-xs text-slate-300 dark:text-slate-600 mt-auto">
    Made with ❤️ for workers who deserve better
  </p>
</div>
