<script>
  import { createEventDispatcher, onMount } from 'svelte';
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
  
  const unsubscribe = aiStore.subscribe(state => {
    textTestStatus = state.textTestStatus;
    textTestMessage = state.textTestMessage;
    textConnectionVerified = state.textConnectionVerified;
  });

  // 是否已配置（必须测试成功）
  $: isTextModelConfigured = textConnectionVerified;
  $: hasTextModelConfig = !!(config?.text_model?.endpoint && config?.text_model?.model);

  // 模式可用性
  $: modeAvailability = aiModes.reduce((acc, mode) => {
    acc[mode.value] = mode.requiresText ? isTextModelConfigured : true;
    return acc;
  }, {});

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
</script>

<!-- 日报模式切换：紧凑的分段控制 -->
<!-- 模式选择与连接状态解耦，用户可先选模式再配置模型 -->
<div class="mb-5">
  <label class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-2">日报模式</label>
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
        class="flex-1 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-150
               {isSelected
                 ? 'bg-primary-500 text-white shadow-sm' 
                 : 'bg-slate-100 dark:bg-slate-700/50 text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-700'}"
      >
        <div>{mode.label}</div>
        <div class="text-[10px] mt-0.5 {isSelected ? 'text-white/70' : 'text-slate-400 dark:text-slate-500'}">{mode.description}</div>
      </button>
    {/each}
  </div>
</div>

<!-- AI 模型配置：仅在 AI 增强模式或云端模式下展开 -->
{#if isAiMode}
  <div class="space-y-3 pt-3 border-t border-slate-200 dark:border-slate-700">
    <!-- 提供商 + 测试按钮 -->
    <div class="flex items-end gap-2">
      <div class="flex-1">
        <label for="ai-provider" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1.5">提供商</label>
        <select
          id="ai-provider"
          value={config.text_model?.provider || 'ollama'}
          on:change={handleProviderChange}
          class="w-full px-3 py-2 text-sm rounded-lg bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-600 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
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
        class="shrink-0 px-3 py-2 text-xs font-medium rounded-lg transition-all
               {textTestStatus === 'success' 
                 ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-400' 
                 : textTestStatus === 'error' 
                   ? 'bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400' 
                   : 'bg-slate-100 hover:bg-slate-200 dark:bg-slate-700 dark:hover:bg-slate-600 text-slate-700 dark:text-slate-300'}
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
      <div class="px-3 py-2 rounded-lg text-xs {textTestStatus === 'success' ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-900/20 dark:text-emerald-400' : 'bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-400'}">
        {textTestMessage}
      </div>
    {/if}

    <!-- API 地址 -->
    <div>
      <label for="ai-endpoint" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1.5">API 地址</label>
      <input
        id="ai-endpoint"
        type="text"
        bind:value={config.text_model.endpoint}
        on:change={handleChange}
        class="w-full px-3 py-2 text-sm font-mono rounded-lg bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-600 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
        placeholder={currentProvider?.default_endpoint || 'http://localhost:11434'}
      />
    </div>

    <!-- API 密钥（按需显示） -->
    {#if requiresApiKey}
      <div>
        <label for="ai-apikey" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1.5">API 密钥</label>
        <input
          id="ai-apikey"
          type="password"
          bind:value={config.text_model.api_key}
          on:change={handleChange}
          class="w-full px-3 py-2 text-sm rounded-lg bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-600 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          placeholder="sk-..."
        />
      </div>
    {/if}

    <!-- 模型名称 -->
    <div>
      <label for="ai-model" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1.5">模型名称</label>
      <input
        id="ai-model"
        type="text"
        bind:value={config.text_model.model}
        on:change={handleChange}
        class="w-full px-3 py-2 text-sm rounded-lg bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-600 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
        placeholder={currentProvider?.default_model || 'qwen2.5'}
      />
      {#if currentProvider?.description}
        <p class="mt-1 text-xs text-slate-400">{currentProvider.description}</p>
      {/if}
    </div>
  </div>
{:else}
  <!-- 未启用 AI 模式时的提示 -->
  <div class="pt-3 border-t border-slate-200 dark:border-slate-700">
    <p class="text-xs text-slate-400 dark:text-slate-500 text-center py-2">切换到「AI 增强」模式后可配置 AI 模型</p>
  </div>
{/if}
