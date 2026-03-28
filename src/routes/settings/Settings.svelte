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
  let config = null;
  let loading = true;
  let saving = false;
  let error = null;
  let success = false;
  let providers = [];
  let runningApps = [];
  let recentApps = [];
  let storageStats = null;
  let dataDir = '';
  let defaultDataDir = '';
  let successTimer = null;

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
      const [loadedConfig, loadedProviders, loadedStorageStats, loadedDataDir, loadedDefaultDataDir] = await Promise.all([
        invoke('get_config'),
        invoke('get_ai_providers'),
        invoke('get_storage_stats'),
        invoke('get_data_dir'),
        invoke('get_default_data_dir'),
      ]);

      config = loadedConfig;
      cache.setConfig(config);
      providers = loadedProviders;
      storageStats = loadedStorageStats;
      dataDir = loadedDataDir;
      defaultDataDir = loadedDefaultDataDir;

      // 确保对象存在
      if (!config.ai_provider) {
        config.ai_provider = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'llava', vision_model: 'llava' };
      }
      if (!config.text_model) {
        config.text_model = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'qwen2.5' };
      }
      if (!config.text_model_profiles) {
        config.text_model_profiles = [];
      }
      if (!config.vision_model) {
        config.vision_model = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'llava' };
      }
      if (typeof config.lightweight_mode !== 'boolean') {
        config.lightweight_mode = false;
      }
      if (!config.app_category_rules) config.app_category_rules = [];
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

    try {
      await invoke('save_config', { config });
      success = true;
      cache.setConfig(config);
      showToast('设置已保存', 'success');
      
      clearTimeout(successTimer);
      successTimer = setTimeout(() => {
        success = false;
        successTimer = null;
      }, 3000);
    } catch (e) {
      error = e.toString();
    } finally {
      saving = false;
    }
  }

  // 清理缓存回调
  async function handleClearCache() {
    const [latestStats, latestDataDir] = await Promise.all([
      invoke('get_storage_stats'),
      invoke('get_data_dir'),
    ]);
    storageStats = latestStats;
    dataDir = latestDataDir;
  }

  async function handleDataDirChanged() {
    const [latestStats, latestDataDir] = await Promise.all([
      invoke('get_storage_stats'),
      invoke('get_data_dir'),
    ]);
    storageStats = latestStats;
    dataDir = latestDataDir;
    cache.clear();
  }

  onMount(() => {
    const unsubscribeCache = cache.subscribe((state) => {
      if (!state.config) return;
      config = state.config;
    });

    loadConfig();
    loadRunningApps();
    loadRecentApps();

    return () => {
      unsubscribeCache();
      clearTimeout(successTimer);
    };
  });
</script>

<div class="page-shell">
  <div class="page-header">
    <div class="page-title-group">
      <div class="page-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>设置</h2>
        <p>应用配置与隐私规则</p>
      </div>
    </div>

    <!-- 保存按钮 -->
    <button
      on:click={saveConfig}
      disabled={loading || saving}
      class="settings-action-primary px-4 rounded-xl"
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
    <div class="page-banner-error mb-6">
      <div>
        <p class="font-semibold">加载配置失败</p>
        <p class="text-sm mt-1">{error}</p>
      </div>
      <button on:click={loadConfig} class="page-action-brand">重试</button>
    </div>
  {:else if config}
    <div class="w-full">
      <!-- 标签栏 -->
      <div class="page-tabs">
        {#each tabs as tab}
          <button
            on:click={() => activeTab = tab.id}
            class="page-tab-btn
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
        <div class="page-card">
          <h3 class="settings-card-title">模型连接</h3>
          <p class="settings-card-desc">配置当前默认模型，并管理多个可供助手页切换的连接</p>
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
          {dataDir}
          {defaultDataDir}
          on:change={() => {}}
          on:clearCache={handleClearCache}
          on:dataDirChanged={handleDataDirChanged}
        />
      {/if}
      </div>
    </div>
  {/if}
</div>
