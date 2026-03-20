<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { cache } from '../../lib/stores/cache.js';
  import { showToast } from '../../lib/stores/toast.js';

  import SettingsGeneral from './components/SettingsGeneral.svelte';
  import SettingsAI from './components/SettingsAI.svelte';
  import SettingsAppearance from './components/SettingsAppearance.svelte';
  import SettingsPrivacy from './components/SettingsPrivacy.svelte';
  import SettingsStorage from './components/SettingsStorage.svelte';
  import { aiStore } from '../../lib/stores/ai.js';

  let config = null;
  let loading = true;
  let saving = false;
  let error = null;
  let success = false;
  let providers = [];
  let runningApps = [];
  let recentApps = [];
  let storageStats = null;

  // 当前激活的标签
  let activeTab = 'general';

  const tabs = [
    { id: 'general', label: '常规', icon: 'general' },
    { id: 'ai', label: 'AI 模型', icon: 'ai' },
    { id: 'appearance', label: '外观', icon: 'appearance' },
    { id: 'privacy', label: '隐私', icon: 'privacy' },
    { id: 'storage', label: '存储', icon: 'storage' },
  ];

  // 加载配置
  async function loadConfig() {
    loading = true;
    error = null;
    try {
      const [loadedConfig, loadedProviders, loadedStorageStats] = await Promise.all([
        invoke('get_config'),
        invoke('get_ai_providers'),
        invoke('get_storage_stats'),
      ]);

      config = loadedConfig;
      cache.setConfig(config);
      providers = loadedProviders;
      storageStats = loadedStorageStats;

      // 确保对象存在
      if (!config.ai_provider) {
        config.ai_provider = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'llava', vision_model: 'llava' };
      }
      if (!config.text_model) {
        config.text_model = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'qwen2.5' };
      }
      if (!config.vision_model) {
        config.vision_model = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'llava' };
      }
      if (!config.privacy.app_rules) config.privacy.app_rules = [];
      if (!config.privacy.sensitive_keywords) config.privacy.sensitive_keywords = [];
    } catch (e) {
      error = e.toString();
      console.error('加载配置失败:', e);
    } finally {
      loading = false;
    }
  }

  // 加载运行中的应用
  async function loadRunningApps() {
    try {
      runningApps = await invoke('get_running_apps');
    } catch (e) {
      console.error('获取运行应用失败:', e);
      runningApps = [];
    }
  }

  // 加载历史应用列表
  async function loadRecentApps() {
    try {
      recentApps = await invoke('get_recent_apps');
    } catch (e) {
      console.error('获取历史应用失败:', e);
      recentApps = [];
    }
  }

  // 保存配置
  async function saveConfig() {
    saving = true;
    error = null;
    success = false;

    // 验证 AI 设置
    let aiState;
    const unsub = aiStore.subscribe(s => aiState = s);
    unsub();

    let fallbackWarning = false;
    if (config.ai_mode === 'summary' && (!aiState || !aiState.textConnectionVerified)) {
      config.ai_mode = 'local';
      fallbackWarning = true;
    }

    try {
      await invoke('save_config', { config });
      success = true;
      cache.setConfig(config);
      
      if (fallbackWarning) {
        showToast('AI 模型尚未通过测试，日报模式已自动重置为基础模板', 'warning');
      }
      
      setTimeout(() => success = false, 3000);
    } catch (e) {
      error = e.toString();
    } finally {
      saving = false;
    }
  }

  // 清理缓存回调
  async function handleClearCache() {
    storageStats = await invoke('get_storage_stats');
  }

  onMount(() => {
    loadConfig();
    loadRunningApps();
    loadRecentApps();
  });
</script>

<div class="p-5 animate-fadeIn">
  <div class="flex justify-between items-center mb-5">
    <div>
      <h2 class="text-lg font-semibold text-slate-800 dark:text-white">设置</h2>
      <p class="text-sm text-slate-400 dark:text-slate-500 mt-0.5">应用配置与隐私规则</p>
    </div>

    <!-- 保存按钮 -->
    <button
      on:click={saveConfig}
      disabled={loading || saving}
      class="px-4 py-2 text-sm font-medium rounded-xl bg-indigo-500 hover:bg-indigo-600 text-white transition-all flex items-center gap-2 disabled:opacity-50"
    >
      {#if saving}
        <div class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
        保存中...
      {:else if success}
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" /></svg>
        已保存
      {:else}
        保存设置
      {/if}
    </button>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
    </div>
  {:else if error}
    <div class="p-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 rounded-lg mb-6">
      <p class="font-bold">加载配置失败</p>
      <p class="text-sm">{error}</p>
      <button on:click={loadConfig} class="mt-2 text-sm underline">重试</button>
    </div>
  {:else if config}
    <div class="max-w-2xl">
      <!-- 标签栏 -->
      <div class="flex gap-1 mb-5 p-1 bg-slate-100/50 dark:bg-slate-800/50 rounded-xl">
        {#each tabs as tab}
          <button
            on:click={() => activeTab = tab.id}
            class="flex-1 flex items-center justify-center gap-1.5 py-2 px-2 rounded-lg text-xs font-medium transition-all duration-200
                   {activeTab === tab.id
                     ? 'bg-white dark:bg-slate-700 text-indigo-600 dark:text-indigo-400 shadow-sm'
                     : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300'}"
          >
            {#if tab.icon === 'general'}
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>
            {:else if tab.icon === 'ai'}
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" /></svg>
            {:else if tab.icon === 'appearance'}
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" /></svg>
            {:else if tab.icon === 'privacy'}
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" /></svg>
            {:else if tab.icon === 'storage'}
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" /></svg>
            {/if}
            <span>{tab.label}</span>
          </button>
        {/each}
      </div>

      <!-- 内容区域 -->
      <div>
      {#if activeTab === 'general'}
        <SettingsGeneral bind:config on:change={() => {}} />
      {:else if activeTab === 'ai'}
        <div class="card p-5">
          <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-200 mb-1">AI 模型配置</h3>
          <p class="text-xs text-slate-400 dark:text-slate-500 mb-4">配置 AI 模型用于生成工作日报</p>
          <SettingsAI bind:config {providers} on:change={() => {}} />
        </div>
      {:else if activeTab === 'appearance'}
        <SettingsAppearance bind:config on:change={() => {}} />
      {:else if activeTab === 'privacy'}
        <SettingsPrivacy
          bind:config
          {runningApps}
          {recentApps}
          on:change={() => {}}
        />
      {:else if activeTab === 'storage'}
        <SettingsStorage
          bind:config
          {storageStats}
          on:change={() => {}}
          on:clearCache={handleClearCache}
        />
      {/if}
      </div>
    </div>
  {/if}
</div>
