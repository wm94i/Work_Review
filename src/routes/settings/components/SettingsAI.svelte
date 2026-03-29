<script>
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { aiStore } from '$lib/stores/ai.js';
  
  export let config;
  export let providers = [];
  
  const dispatch = createEventDispatcher();
  
  // 日报生成模式：基础模板 vs AI 增强
  const aiModes = [
    { 
      value: 'local', 
      label: '基础模板', 
      description: '固定格式统计报告',
      requiresText: false
    },
    { 
      value: 'summary', 
      label: 'AI 增强', 
      description: '调用 AI 生成智能总结',
      requiresText: true
    },
  ];

  // 提供商默认配置
  function getProviderDefaults(providerId) {
    const provider = providers.find(p => p.id === providerId);
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

  // 模型列表获取状态
  let fetchingModels = false;
  let modelList = [];
  let fetchModelsError = '';
  
  const unsubscribe = aiStore.subscribe(state => {
    textTestStatus = state.textTestStatus;
    textTestMessage = state.textTestMessage;
    textConnectionVerified = state.textConnectionVerified;
  });

  // 是否已配置（必须测试成功）
  $: isTextModelConfigured = textConnectionVerified;
  $: hasTextModelConfig = !!(config?.text_model?.endpoint && config?.text_model?.model);

  // 当前提供商
  $: currentProvider = providers.find(p => p.id === config?.text_model?.provider) || providers[0];
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
    
    modelList = [];
    fetchModelsError = '';
    aiStore.reset();
    dispatch('change', config);
  }

  function handleChange() {
    // 阻止派发含有未验证文本模型的配置
    if (config.ai_mode === 'summary' && !isTextModelConfigured) {
      aiStore.setError("必须先完成 API 连接测试才能保存");
      return; 
    }
    dispatch('change', config);
  }

  async function fetchModelList() {
    fetchingModels = true;
    fetchModelsError = '';
    modelList = [];
    try {
      const result = await invoke('fetch_models', {
        modelConfig: {
          provider: config.text_model.provider,
          endpoint: config.text_model.endpoint,
          api_key: config.text_model.api_key || null,
          model: '',
        }
      });
      modelList = result || [];
      if (modelList.length === 0) {
        fetchModelsError = '未获取到模型列表，请检查网络连接或 API 配置';
      }
    } catch (e) {
      fetchModelsError = e.toString();
    } finally {
      fetchingModels = false;
    }
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
        aiStore.setSuccess(result.message + (result.response_time_ms ? ` (${result.response_time_ms}ms)` : '') + '，请点击右上角保存设置');
      } else {
        aiStore.setError(result.message);
      }
    } catch (e) {
      aiStore.setError(e.toString());
    }
  }

  function getConfigHash() {
    if (!config?.text_model) return null;
    const { provider, endpoint, model, api_key } = config.text_model;
    return `${provider}|${endpoint}|${model}|${api_key || ''}`;
  }

  // 挂载时只在配置变化时自动测试
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
  });

  onDestroy(() => {
    unsubscribe();
  });
</script>

<!-- 日报模式切换：紧凑的分段控制 -->
<!-- 模式选择与连接状态解耦，用户可先选模式再配置模型 -->
<fieldset class="mb-5">
  <legend class="settings-label mb-2">日报模式</legend>
  <div class="flex gap-2">
    {#each aiModes as mode}
      {@const isSelected = config.ai_mode === mode.value}
      <button 
        type="button"
        on:click={() => { 
          // 仅当切换需要文字模型且未配置或测试失败时，给提示并阻止向父组件发送 change（避免自动保存未验证状态）
          if (mode.requiresText && !isTextModelConfigured) {
            config.ai_mode = mode.value; // 允许 UI 切换展开面板
            aiStore.setError("请先配置并测试 AI 模型连接");
            // 不触发 handleChange()，防止父组件认为配置已完备
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

<!-- AI 模型配置：仅在 AI 增强模式或云端模式下展开 -->
{#if isAiMode}
  <div class="settings-block pt-3 border-t border-slate-200 dark:border-slate-700">
    <!-- 提供商 + 测试按钮 -->
    <div class="flex items-end gap-2">
      <div class="flex-1">
        <label for="ai-provider" class="settings-label mb-1.5">提供商</label>
        <select
          id="ai-provider"
          value={config.text_model?.provider || 'ollama'}
          on:change={handleProviderChange}
          class="control-input"
        >
          {#each providers as provider}
            <option value={provider.id}>{provider.name}</option>
          {/each}
        </select>
      </div>
      
      <!-- 测试按钮紧跟提供商选择 -->
      <button
        on:click={testTextModel}
        disabled={textTestStatus === 'testing' || !hasTextModelConfig}
        class="shrink-0 min-h-10 px-3 py-2 text-xs font-medium rounded-lg leading-none transition-all
               {textTestStatus === 'success' 
                 ? 'settings-action-success' 
                 : textTestStatus === 'error' 
                   ? 'settings-action-danger' 
                   : 'settings-action-secondary'}
               disabled:opacity-40 disabled:cursor-not-allowed"
      >
        {#if textTestStatus === 'testing'}
          <span class="inline-flex items-center gap-1">
            <span class="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
            测试中
          </span>
        {:else if textTestStatus === 'success'}
          ✓ 连接成功
        {:else if textTestStatus === 'error'}
          ✗ 连接失败
        {:else}
          测试连接
        {/if}
      </button>
    </div>
    
    <!-- 测试结果消息 -->
    {#if textTestMessage}
      <div class="px-3 py-2 rounded-lg text-xs {textTestStatus === 'success' ? 'settings-tone-success' : 'settings-tone-danger'}">
        {textTestMessage}
      </div>
    {/if}

    <!-- API 地址 -->
    <div>
      <label for="ai-endpoint" class="settings-label mb-1.5">API 地址</label>
      <input
        id="ai-endpoint"
        type="text"
        bind:value={config.text_model.endpoint}
        on:change={handleChange}
        class="control-input-mono"
        placeholder={currentProvider?.default_endpoint || 'http://localhost:11434'}
      />
    </div>

    <!-- API 密钥（按需显示） -->
    {#if requiresApiKey}
      <div>
        <label for="ai-apikey" class="settings-label mb-1.5">API 密钥</label>
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
            aria-label={showApiKey ? '隐藏 API 密钥' : '显示 API 密钥'}
            title={showApiKey ? '隐藏 API 密钥' : '显示 API 密钥'}
            on:click={() => {
              showApiKey = !showApiKey;
            }}
          >
            <svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9">
              <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12s3.75-6.75 9.75-6.75S21.75 12 21.75 12 18 18.75 12 18.75 2.25 12 2.25 12Z" />
              <circle cx="12" cy="12" r="3.25" />
            </svg>
          </button>
        </div>
      </div>
    {/if}

    <!-- 模型名称 -->
    <div>
      <div class="flex items-end gap-2">
        <div class="flex-1">
          <label for="ai-model" class="settings-label mb-1.5">模型名称</label>
          <input
            id="ai-model"
            type="text"
            bind:value={config.text_model.model}
            on:change={handleChange}
            class="control-input"
            placeholder={currentProvider?.default_model || 'qwen2.5'}
          />
        </div>
        <button
          type="button"
          on:click={fetchModelList}
          disabled={fetchingModels || !config.text_model.endpoint}
          class="shrink-0 min-h-10 px-3 py-2 text-xs font-medium rounded-lg leading-none transition-all
                 settings-action-secondary
                 disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {#if fetchingModels}
            <span class="inline-flex items-center gap-1">
              <span class="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
              获取中
            </span>
          {:else}
            获取列表
          {/if}
        </button>
      </div>
      {#if fetchModelsError}
        <p class="text-xs text-red-500 dark:text-red-400 mt-1.5">{fetchModelsError}</p>
      {/if}
      {#if modelList.length > 0}
        <select
          class="control-input mt-2"
          value={config.text_model.model}
          on:change={(e) => {
            config.text_model.model = e.target.value;
            aiStore.reset();
            dispatch('change', config);
          }}
        >
          <option value="" disabled>选择模型...</option>
          {#each modelList as m}
            <option value={m}>{m}</option>
          {/each}
        </select>
        <p class="settings-note">共获取到 {modelList.length} 个可用模型</p>
      {/if}
      {#if currentProvider?.description}
        <p class="settings-note">{currentProvider.description}</p>
      {/if}
    </div>
  </div>
{:else}
  <!-- 未启用 AI 模式时的提示 -->
  <div class="pt-3 border-t border-slate-200 dark:border-slate-700">
    <p class="settings-empty">切换到「AI 增强」模式后可配置 AI 模型</p>
  </div>
{/if}
