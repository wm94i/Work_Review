<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-shell';
  import { getVersion } from '@tauri-apps/api/app';

  let appVersion = '';
  let dataDir = '';

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

    <!-- 操作按钮 -->
    <div class="flex gap-3 mb-6">
      <button on:click={openGitHub} class="flex-1 flex items-center justify-center gap-2 px-5 py-3 text-sm font-medium rounded-xl bg-indigo-500 hover:bg-indigo-600 text-white transition-all">
        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
        GitHub
      </button>
      <button on:click={openDataDir} class="flex-1 flex items-center justify-center gap-2 px-5 py-3 text-sm font-medium rounded-xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition-all">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
        数据目录
      </button>
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
