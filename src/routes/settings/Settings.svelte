<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { cache } from '../../lib/stores/cache.js';
  import { locale, t } from '$lib/i18n/index.js';
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
  $: currentLocale = $locale;

  // 当前激活的标签
  let activeTab = 'general';

  const tabs = [
    { id: 'general', labelKey: 'settings.tabs.general', icon: 'general' },
    { id: 'ai', labelKey: 'settings.tabs.ai', icon: 'ai' },
    { id: 'appearance', labelKey: 'settings.tabs.appearance', icon: 'appearance' },
    { id: 'privacy', labelKey: 'settings.tabs.privacy', icon: 'privacy' },
    { id: 'storage', labelKey: 'settings.tabs.storage', icon: 'storage' },
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
      if (typeof config.daily_report_custom_prompt !== 'string') {
        config.daily_report_custom_prompt = '';
      }
      if (typeof config.daily_report_export_dir !== 'string' && config.daily_report_export_dir !== null) {
        config.daily_report_export_dir = null;
      }
      if (!config.vision_model) {
        config.vision_model = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'llava' };
      }
      if (typeof config.lightweight_mode !== 'boolean') {
        config.lightweight_mode = false;
      }
      if (typeof config.break_reminder_enabled !== 'boolean') {
        config.break_reminder_enabled = false;
      }
      if (![30, 45, 50, 60, 90, 120].includes(config.break_reminder_interval_minutes)) {
        config.break_reminder_interval_minutes = 50;
      }
      if (typeof config.auto_start_silent !== 'boolean') {
        config.auto_start_silent = false;
      }
      if (!config.storage) {
        config.storage = {
          screenshot_retention_days: 7,
          metadata_retention_days: 30,
          storage_limit_mb: 2048,
          jpeg_quality: 85,
          max_image_width: 1280,
          screenshots_enabled: true,
          screenshot_display_mode: 'active_window',
        };
      }
      if (typeof config.storage.screenshots_enabled !== 'boolean') {
        config.storage.screenshots_enabled = true;
      }
      if (!config.storage.screenshot_display_mode) {
        config.storage.screenshot_display_mode = 'active_window';
      }
      if (!config.app_category_rules) config.app_category_rules = [];
      if (!config.privacy.app_rules) config.privacy.app_rules = [];
      if (!config.privacy.excluded_keywords) config.privacy.excluded_keywords = [];
      delete config.privacy.sensitive_keywords;
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
      delete config.privacy?.sensitive_keywords;
      await invoke('save_config', { config });
      success = true;
      cache.setConfig(config);
      showToast(t('settings.saveSuccessToast'), 'success');
      
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

<div class="page-shell settings-editorial-shell" data-locale={currentLocale}>
  <div class="page-header">
    <div class="page-title-group">
      <div class="page-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>{t('settings.title')}</h2>
        <p>{t('settings.subtitle')}</p>
      </div>
    </div>

    <!-- 保存按钮 -->
    <div class="settings-save-dock">
      <button
        on:click={saveConfig}
        disabled={loading || saving}
        class="settings-action-primary px-4 rounded-xl"
      >
        {#if saving}
          <div class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
          {t('settings.saving')}
        {:else if success}
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" /></svg>
          {t('settings.saved')}
        {:else}
          {t('settings.save')}
        {/if}
      </button>
    </div>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
    </div>
  {:else if error}
    <div class="page-banner-error mb-6">
      <div>
        <p class="font-semibold">{t('settings.loadError')}</p>
        <p class="text-sm mt-1">{error}</p>
      </div>
      <button on:click={loadConfig} class="page-action-brand">{t('settings.retry')}</button>
    </div>
  {:else if config}
    <div class="w-full settings-editorial-board">
      <div class="settings-stage-layout">
        <div class="settings-tab-rail">
          {#each tabs as tab}
            <button
              on:click={() => activeTab = tab.id}
              class="settings-tab-rail-item {activeTab === tab.id ? 'settings-tab-rail-item-active' : ''}"
            >
              <span class="settings-tab-rail-icon">
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
              </span>
              <span>{t(tab.labelKey)}</span>
            </button>
          {/each}
        </div>

        <div class="settings-stage-shell">
        {#if activeTab === 'general'}
          <SettingsGeneral bind:config on:change={() => {}} />
        {:else if activeTab === 'ai'}
          <div class="page-card">
            <h3 class="settings-card-title">{t('settings.aiCardTitle')}</h3>
            <p class="settings-card-desc">{t('settings.aiCardDescription')}</p>
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
    </div>
  {/if}
</div>
