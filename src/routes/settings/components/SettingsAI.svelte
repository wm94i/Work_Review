<script>
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { aiStore } from '$lib/stores/ai.js';
  import { locale, t } from '$lib/i18n/index.js';

  export let config;
  export let providers = [];

  const dispatch = createEventDispatcher();
  $: currentLocale = $locale;
  let aiModes = [];
  let localizedProviders = [];

  // 日报生成模式：基础模板 vs AI 增强
  const aiModeConfigs = [
    {
      value: 'local',
      labelKey: 'settingsAI.modeLocal',
      descriptionKey: 'settingsAI.modeLocalDesc',
      requiresText: false
    },
    {
      value: 'summary',
      labelKey: 'settingsAI.modeSummary',
      descriptionKey: 'settingsAI.modeSummaryDesc',
      requiresText: true
    },
  ];
  $: {
    currentLocale;
    aiModes = aiModeConfigs.map((mode) => ({
      ...mode,
      label: t(mode.labelKey),
      description: t(mode.descriptionKey),
    }));
  }

  const providerLabels = {
    ollama: {
      'zh-CN': { name: 'Ollama (本地)', description: '本机运行开源模型，数据不出本机' },
      en: { name: 'Ollama (Local)', description: 'Runs open models on your device, data stays local' },
      'zh-TW': { name: 'Ollama（本機）', description: '在本機執行開源模型，資料不會離開本機' },
    },
    openai: {
      'zh-CN': { name: 'OpenAI / 兼容 API', description: '支持官方及兼容端点（Azure、Cloudflare 等）' },
      en: { name: 'OpenAI / Compatible', description: 'Official OpenAI and compatible endpoints (Azure, Cloudflare, etc.)' },
      'zh-TW': { name: 'OpenAI / 相容 API', description: '支援官方與相容端點（Azure、Cloudflare 等）' },
    },
    siliconflow: {
      'zh-CN': { name: '硅基流动 SiliconFlow', description: '国内高性价比 API' },
      en: { name: 'SiliconFlow', description: 'Cost-effective domestic API' },
      'zh-TW': { name: '矽基流動 SiliconFlow', description: '高性價比 API' },
    },
    deepseek: {
      'zh-CN': { name: 'DeepSeek', description: '国产开源模型，兼容 OpenAI 格式' },
      en: { name: 'DeepSeek', description: 'Open-source model with OpenAI-compatible format' },
      'zh-TW': { name: 'DeepSeek', description: '開源模型，支援 OpenAI 相容格式' },
    },
    qwen: {
      'zh-CN': { name: '通义千问 Qwen', description: '阿里云通义大模型' },
      en: { name: 'Qwen', description: 'Alibaba Tongyi models' },
      'zh-TW': { name: '通義千問 Qwen', description: '阿里雲通義模型' },
    },
    zhipu: {
      'zh-CN': { name: '智谱 ChatGLM', description: '智谱 AI 大模型' },
      en: { name: 'Zhipu ChatGLM', description: 'Large language models from Zhipu AI' },
      'zh-TW': { name: '智譜 ChatGLM', description: '智譜 AI 大模型' },
    },
    moonshot: {
      'zh-CN': { name: '月之暗面 Kimi', description: '擅长长文本' },
      en: { name: 'Moonshot Kimi', description: 'Optimized for long-context tasks' },
      'zh-TW': { name: '月之暗面 Kimi', description: '擅長長文本' },
    },
    doubao: {
      'zh-CN': { name: '火山引擎 豆包', description: '字节跳动大模型' },
      en: { name: 'Doubao', description: 'Models from Volcano Engine / ByteDance' },
      'zh-TW': { name: '火山引擎 豆包', description: '字節跳動大模型' },
    },
    minimax: {
      'zh-CN': { name: '稀宇科技 MiniMax', description: 'MiniMax 文本模型' },
      en: { name: 'MiniMax', description: 'MiniMax text models' },
      'zh-TW': { name: '稀宇科技 MiniMax', description: 'MiniMax 文字模型' },
    },
    gemini: {
      'zh-CN': { name: 'Google Gemini', description: 'Google Gemini 系列模型' },
      en: { name: 'Google Gemini', description: 'Google Gemini family models' },
      'zh-TW': { name: 'Google Gemini', description: 'Google Gemini 系列模型' },
    },
    claude: {
      'zh-CN': { name: 'Anthropic Claude', description: 'Anthropic Claude 系列模型' },
      en: { name: 'Anthropic Claude', description: 'Anthropic Claude family models' },
      'zh-TW': { name: 'Anthropic Claude', description: 'Anthropic Claude 系列模型' },
    },
  };

  function getLocalizedProvider(provider) {
    const localized = providerLabels[provider?.id]?.[currentLocale];
    if (!localized) {
      return provider;
    }
    return {
      ...provider,
      name: localized.name,
      description: localized.description,
    };
  }
  $: {
    currentLocale;
    localizedProviders = providers.map(getLocalizedProvider);
  }

  // 提供商默认配置
  function getProviderDefaults(providerId) {
    const provider = localizedProviders.find(p => p.id === providerId);
    return {
      endpoint: provider?.default_endpoint || '',
      model: provider?.default_model || '',
      requiresApiKey: provider?.requires_api_key ?? true
    };
  }

  // 从全局 store 订阅测试状态
  let textTestStatus = null;
  let textTestMessage = '';
  let textConnectionVerified = false;
  let showApiKey = false;
  let fetchedModels = [];
  let modelsLoading = false;
  let modelsError = '';
  let modelsLoaded = 0;
  let showManualInput = false;

  const unsubscribe = aiStore.subscribe(state => {
    textTestStatus = state.textTestStatus;
    textTestMessage = state.textTestMessage;
    textConnectionVerified = state.textConnectionVerified;
  });

  // 是否已配置（必须测试成功）
  $: isTextModelConfigured = textConnectionVerified;
  $: hasTextModelConfig = !!(config?.text_model?.endpoint && config?.text_model?.model);

  // 当前提供商
  $: currentProvider = localizedProviders.find(p => p.id === config?.text_model?.provider) || localizedProviders[0];
  $: requiresApiKey = currentProvider?.requires_api_key ?? true;

  // 是否选择了 AI 增强模式（决定是否展开配置面板）
  $: isAiMode = config.ai_mode === 'summary';

  // 每个 provider 的配置缓存（切换时保留配置）
  let providerConfigs = {};
  let configInitialized = false;

  $: if (config?.text_model?.provider && !configInitialized) {
    providerConfigs[config.text_model.provider] = {
      endpoint: config.text_model.endpoint,
      model: config.text_model.model,
      api_key: config.text_model.api_key || ''
    };
    configInitialized = true;
  }

  function handleProviderChange(e) {
    const providerId = e.target.value;

    // 缓存当前 provider 配置
    if (config.text_model.provider) {
      providerConfigs[config.text_model.provider] = {
        endpoint: config.text_model.endpoint,
        model: config.text_model.model,
        api_key: config.text_model.api_key || ''
      };
    }

    // 恢复缓存或使用默认值
    const defaults = getProviderDefaults(providerId);
    const cached = providerConfigs[providerId];

    config.text_model.provider = providerId;
    config.text_model.endpoint = cached?.endpoint || defaults.endpoint;
    config.text_model.model = cached?.model || defaults.model;
    config.text_model.api_key = cached?.api_key || '';

    // 切换提供商时清空状态
    textTestStatus = null;
    textTestMessage = '';
    textConnectionVerified = false;
    modelsError = '';
    fetchedModels = [];
    modelsLoaded = 0;
    aiStore.reset();
    refreshModels();
    dispatch('change', config);
  }

  function handleChange() {
    if (config.ai_mode === 'summary' && !isTextModelConfigured) {
      aiStore.setError(t('settingsAI.saveRequiresVerifiedModel'));
      return;
    }
    dispatch('change', config);
  }

  function shouldHideRawMessage(message) {
    return currentLocale === 'en' && /[一-鿿]/.test(message);
  }

  async function testTextModel() {
    aiStore.startTesting();
    try {
      const result = await invoke('test_model', {
        modelConfig: {
          provider: config.text_model.provider,
          endpoint: config.text_model.endpoint,
          api_key: config.text_model.api_key,
          model: config.text_model.model,
        }
      });
      if (result.success) {
        aiStore.setSuccess(
          result.response_time_ms
            ? t('settingsAI.saveAfterTestWithLatency', { ms: result.response_time_ms })
            : t('settingsAI.saveAfterTest')
        );
      } else {
        const failureMessage = String(result?.message || '').trim();
        aiStore.setError(
          failureMessage && !shouldHideRawMessage(failureMessage)
            ? failureMessage
            : t('settingsAI.genericTestFailed')
        );
      }
    } catch (e) {
      const failureMessage = String(e || '').trim();
      aiStore.setError(
        failureMessage && !shouldHideRawMessage(failureMessage)
          ? failureMessage
          : t('settingsAI.genericTestFailed')
      );
    }
  }

  async function refreshModels() {
    modelsError = '';
    fetchedModels = [];
    modelsLoaded = 0;

    if (!config?.text_model?.endpoint) return;

    const provider = providers.find(p => p.id === config.text_model.provider);
    const needsApiKey = provider?.requires_api_key ?? true;
    if (needsApiKey && !config.text_model.api_key) return;

    modelsLoading = true;
    try {
      const models = await invoke('fetch_models', {
        provider: config.text_model.provider,
        endpoint: config.text_model.endpoint,
        apiKey: config.text_model.api_key || null,
      });
      fetchedModels = Array.isArray(models) ? models : [];
      modelsLoaded = fetchedModels.length;
      if (fetchedModels.length > 0 && !config.text_model.model?.trim()) {
        config.text_model.model = fetchedModels[0];
        dispatch('change', config);
      }
    } catch (e) {
      fetchedModels = [];
      modelsLoaded = 0;
      const msg = e.toString();
      modelsError = msg;
      aiStore.setError(
        msg && !shouldHideRawMessage(msg)
          ? msg
          : t('settingsAI.genericTestFailed')
      );
    } finally {
      modelsLoading = false;
    }
  }

  function getConfigHash() {
    if (!config?.text_model) return null;
    const { provider, endpoint, model, api_key } = config.text_model;
    return `${provider}|${endpoint}|${model}|${api_key || ''}`;
  }

  onMount(async () => {
    await new Promise(r => setTimeout(r, 200));

    const currentHash = getConfigHash();
    let lastHash = null;
    const unsub = aiStore.subscribe(s => { lastHash = s.lastTestedConfigHash; });
    unsub();

    if (hasTextModelConfig && currentHash !== lastHash) {
      aiStore.setConfigHash(currentHash);
      await testTextModel();
    }

    if (config?.text_model?.endpoint && (!requiresApiKey || config.text_model.api_key)) {
      await refreshModels();
    }
  });

  onDestroy(() => {
    unsubscribe();
  });
</script>

<!-- 日报模式切换 -->
<fieldset class="mb-5" data-locale={currentLocale}>
  <legend class="settings-label mb-2">{t('settingsAI.modeLegend')}</legend>
  <div class="flex gap-2">
    {#each aiModes as mode}
      {@const isSelected = config.ai_mode === mode.value}
      <button
        type="button"
        on:click={() => {
          if (mode.requiresText && !isTextModelConfigured) {
            config.ai_mode = mode.value;
            aiStore.setError(t('settingsAI.switchRequiresVerifiedModel'));
          } else {
            config.ai_mode = mode.value;
            handleChange();
          }
        }}
        class="flex-1 min-h-16 px-3 py-2.5 rounded-lg text-sm font-medium leading-none transition-all duration-150
               {isSelected
                 ? 'settings-segment-active'
                 : 'settings-segment-base'}"
      >
        <div class="flex h-full flex-col items-center justify-center gap-1 text-center">
          <div class="leading-none">{mode.label}</div>
          <div class="text-[10px] leading-none {isSelected ? 'text-white/70' : 'settings-subtle'}">{mode.description}</div>
        </div>
      </button>
    {/each}
  </div>
</fieldset>

<!-- AI 模型配置 -->
{#if isAiMode}
  <div class="settings-block pt-3 border-t border-slate-200 dark:border-slate-700">
    <!-- 提供商 -->
    <div>
      <label for="ai-provider" class="settings-label mb-1.5">{t('settingsAI.provider')}</label>
      <select
        id="ai-provider"
        value={config.text_model?.provider || 'ollama'}
        on:change={handleProviderChange}
        class="control-input"
      >
        {#each localizedProviders as provider}
          <option value={provider.id}>{provider.name}</option>
        {/each}
      </select>
      {#if currentProvider?.description}
        <p class="settings-note mt-1">{currentProvider.description}</p>
      {/if}
    </div>

    <!-- API 地址 -->
    <div>
      <label for="ai-endpoint" class="settings-label mb-1.5">{t('settingsAI.endpoint')}</label>
      <input
        id="ai-endpoint"
        type="text"
        bind:value={config.text_model.endpoint}
        on:change={handleChange}
        class="control-input-mono"
        placeholder={currentProvider?.default_endpoint || 'http://localhost:11434'}
      />
    </div>

    <!-- API 密钥 -->
    {#if requiresApiKey}
      <div>
        <label for="ai-apikey" class="settings-label mb-1.5">{t('settingsAI.apiKey')}</label>
        <div class="relative">
          {#if showApiKey}
            <input
              id="ai-apikey"
              type="text"
              bind:value={config.text_model.api_key}
              on:change={handleChange}
              class="control-input pr-12"
              placeholder="sk-..."
            />
          {:else}
            <input
              id="ai-apikey"
              type="password"
              bind:value={config.text_model.api_key}
              on:change={handleChange}
              class="control-input pr-12"
              placeholder="sk-..."
            />
          {/if}
          <button
            type="button"
            class="absolute inset-y-0 right-3 inline-flex items-center justify-center text-slate-400 transition hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
            aria-label={showApiKey ? t('settingsAI.hideApiKey') : t('settingsAI.showApiKey')}
            title={showApiKey ? t('settingsAI.hideApiKey') : t('settingsAI.showApiKey')}
            on:click={() => showApiKey = !showApiKey}
          >
            <svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9">
              <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12s3.75-6.75 9.75-6.75S21.75 12 21.75 12 18 18.75 12 18.75 2.25 12 2.25 12Z" />
              <circle cx="12" cy="12" r="3.25" />
            </svg>
          </button>
        </div>
      </div>
    {/if}

    <!-- 测试连接 -->
    <button
      on:click={testTextModel}
      disabled={textTestStatus === 'testing' || !hasTextModelConfig}
      class="w-full min-h-10 px-4 py-2 text-sm font-medium rounded-lg leading-none transition-all
             {textTestStatus === 'success'
               ? 'settings-action-success'
               : textTestStatus === 'error'
                 ? 'settings-action-danger'
                 : 'settings-action-secondary'}
             disabled:opacity-40 disabled:cursor-not-allowed"
    >
      {#if textTestStatus === 'testing'}
        <span class="inline-flex items-center gap-1.5">
          <span class="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
          {t('settingsAI.testing')}
        </span>
      {:else if textTestStatus === 'success'}
        ✓ {t('settingsAI.testSuccess')}
      {:else if textTestStatus === 'error'}
        ✗ {t('settingsAI.testFailed')}
      {:else}
        {t('settingsAI.testConnection')}
      {/if}
    </button>

    <!-- 测试结果 -->
    {#if textTestMessage}
      <div class="px-3 py-2 rounded-lg text-xs {textTestStatus === 'success' ? 'settings-tone-success' : 'settings-tone-danger'}">
        {textTestMessage}
      </div>
    {/if}

    <hr class="border-slate-200 dark:border-slate-700" />

    <!-- 模型选择 -->
    <div>
      <div class="flex items-end gap-2">
        <div class="flex-1">
          <label for="ai-model" class="settings-label mb-1.5">{t('settingsAI.model')}</label>
          {#if showManualInput || fetchedModels.length === 0}
            <input
              id="ai-model"
              type="text"
              bind:value={config.text_model.model}
              on:change={handleChange}
              class="control-input"
              placeholder={currentProvider?.default_model || 'qwen2.5'}
            />
          {:else}
            <select
              id="ai-model"
              value={config.text_model.model}
              on:change={(e) => {
                if (e.target.value === '__manual__') {
                  showManualInput = true;
                  config.text_model.model = '';
                  return;
                }
                config.text_model.model = e.target.value;
                handleChange();
              }}
              class="control-input"
            >
              <option value="__manual__">{t('settingsAI.manualModel')}</option>
              {#each fetchedModels as model (model)}
                <option value={model}>{model}</option>
              {/each}
            </select>
          {/if}
        </div>

        <button
          type="button"
          on:click={refreshModels}
          disabled={modelsLoading || !config.text_model.endpoint || (requiresApiKey && !config.text_model.api_key)}
          class="shrink-0 min-h-10 px-3 py-2 text-xs font-medium rounded-lg leading-none transition-all settings-action-secondary disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {#if modelsLoading}
            <span class="inline-flex items-center gap-1">
              <span class="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
            </span>
          {:else}
            {t('settingsAI.refreshModels')}
          {/if}
        </button>
      </div>

      {#if showManualInput && fetchedModels.length > 0}
        <button
          type="button"
          on:click={() => { showManualInput = false; }}
          class="settings-link-action mt-1"
        >
          {t('settingsAI.backToList')}
        </button>
      {/if}

      {#if modelsError}
        <p class="settings-note text-rose-500 dark:text-rose-400">{modelsError}</p>
      {:else if modelsLoaded > 0}
        <p class="settings-note">{t('settingsAI.loadedModels', { count: modelsLoaded })}</p>
      {/if}
    </div>
  </div>
{:else}
  <div class="pt-3 border-t border-slate-200 dark:border-slate-700">
    <p class="settings-empty">{t('settingsAI.aiModeDisabled')}</p>
  </div>
{/if}
