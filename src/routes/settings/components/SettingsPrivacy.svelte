<script>
  import { createEventDispatcher } from 'svelte';
  
  export let config;
  export let runningApps = [];
  export let recentApps = [];
  
  const dispatch = createEventDispatcher();
  
  // 内联输入状态
  let showAppInput = false;
  let selectedApp = '';
  let selectedLevel = 'ignored';
  let showKeywordInput = false;
  let newKeyword = '';
  let showDomainInput = false;
  let newDomain = '';

  // 隐私级别定义 - 使用文字标签避免 emoji 渲染异常
  const privacyLevels = [
    { value: 'full', label: '完全记录', color: 'text-emerald-600', bg: 'bg-emerald-50 dark:bg-emerald-900/20', desc: '记录截图、活动和OCR文字' },
    { value: 'anonymized', label: '仅统计时长', color: 'text-amber-600', bg: 'bg-amber-50 dark:bg-amber-900/20', desc: '只统计使用时长，不截图不做OCR' },
    { value: 'ignored', label: '完全忽略', color: 'text-red-600', bg: 'bg-red-50 dark:bg-red-900/20', desc: '不记录、不统计、不显示' },
  ];

  function addAppRule() {
    if (!selectedApp) return;
    // 检查是否已存在同名规则
    const existingIndex = config.privacy.app_rules.findIndex(r => r.app_name === selectedApp);
    if (existingIndex >= 0) {
      // 更新已有规则
      config.privacy.app_rules[existingIndex].level = selectedLevel;
      config.privacy.app_rules = [...config.privacy.app_rules];
    } else {
      config.privacy.app_rules = [
        ...config.privacy.app_rules,
        { app_name: selectedApp, level: selectedLevel }
      ];
    }
    showAppInput = false;
    selectedApp = '';
    dispatch('change', config);
  }

  function removeAppRule(index) {
    const rules = [...config.privacy.app_rules];
    rules.splice(index, 1);
    config.privacy.app_rules = rules;
    dispatch('change', config);
  }

  function addKeyword() {
    if (!newKeyword.trim()) return;
    // 避免重复添加
    if (config.privacy.sensitive_keywords.includes(newKeyword.trim())) {
      newKeyword = '';
      return;
    }
    config.privacy.sensitive_keywords = [
      ...config.privacy.sensitive_keywords,
      newKeyword.trim()
    ];
    newKeyword = '';
    showKeywordInput = false;
    dispatch('change', config);
  }

  function removeKeyword(index) {
    const keywords = [...config.privacy.sensitive_keywords];
    keywords.splice(index, 1);
    config.privacy.sensitive_keywords = keywords;
    dispatch('change', config);
  }

  // 域名黑名单管理
  function addDomain() {
    if (!newDomain.trim()) return;
    const domains = config.privacy.excluded_domains || [];
    // 避免重复
    if (domains.includes(newDomain.trim())) {
      newDomain = '';
      return;
    }
    config.privacy.excluded_domains = [...domains, newDomain.trim()];
    newDomain = '';
    showDomainInput = false;
    dispatch('change', config);
  }

  function removeDomain(index) {
    const domains = [...(config.privacy.excluded_domains || [])];
    domains.splice(index, 1);
    config.privacy.excluded_domains = domains;
    dispatch('change', config);
  }

  // 快捷选择应用
  function selectApp(appName) {
    selectedApp = appName;
  }
</script>

<div class="card p-5 mb-5">
  <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-200 mb-1">隐私设置</h3>
  <p class="text-xs text-slate-400 dark:text-slate-500 mb-4">所有数据仅存储在本地，不会上传到任何服务器</p>
  
  <div class="space-y-6">
    <!-- 应用规则 -->
    <div>
      <div class="flex items-center justify-between mb-1">
        <span class="block text-sm font-medium text-slate-700 dark:text-slate-300">
          应用规则
        </span>
        <button
          on:click={() => showAppInput = !showAppInput}
          class="text-sm text-primary-600 hover:text-primary-700 transition-colors"
        >
          {showAppInput ? '收起' : '+ 添加规则'}
        </button>
      </div>
      <p class="text-xs text-slate-500 mb-3">指定应用的记录策略</p>

      {#if showAppInput}
        <div class="p-4 bg-slate-50 dark:bg-slate-700/30 rounded-xl mb-3 animate-fadeIn">
          <!-- 应用名称输入 -->
          <div class="mb-3">
            <label for="app-name-input" class="block text-xs text-slate-500 mb-2">输入应用名称或点击下方选择</label>
            <input
              id="app-name-input"
              type="text"
              bind:value={selectedApp}
              class="input w-full"
              placeholder="如: Chrome, 1Password, 微信"
            />
          </div>
          <!-- 策略选择：分段按钮 -->
          <div class="mb-3">
            <span class="block text-xs text-slate-500 mb-2">记录策略</span>
            <div class="flex rounded-lg overflow-hidden border border-slate-200 dark:border-slate-600">
              {#each privacyLevels as level}
                <button
                  on:click={() => selectedLevel = level.value}
                  class="flex-1 py-2 text-xs font-medium transition-all
                         {selectedLevel === level.value
                           ? level.value === 'full' ? 'bg-emerald-500 text-white'
                             : level.value === 'anonymized' ? 'bg-amber-500 text-white'
                             : 'bg-red-500 text-white'
                           : 'bg-white dark:bg-slate-700 text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-600'}"
                >
                  {level.label}
                </button>
              {/each}
            </div>
            <p class="text-xs mt-1.5 {privacyLevels.find(l => l.value === selectedLevel)?.color || 'text-slate-400'}">
              {privacyLevels.find(l => l.value === selectedLevel)?.desc || ''}
            </p>
          </div>
          
          <!-- 快捷选择 -->
          {#if recentApps.length > 0 || runningApps.length > 0}
          <div class="space-y-3">
            {#if recentApps.length > 0}
            <div>
              <span class="block text-xs text-slate-400 mb-1.5">📁 历史应用</span>
              <div class="flex flex-wrap gap-1.5">
                {#each recentApps.slice(0, 12) as app}
                  <button
                    on:click={() => selectApp(app)}
                    class="px-2.5 py-1 text-xs rounded-lg transition-all
                           {selectedApp === app 
                             ? 'bg-primary-500 text-white' 
                             : 'bg-white dark:bg-slate-600 text-slate-600 dark:text-slate-300 hover:bg-primary-50 dark:hover:bg-slate-500 border border-slate-200 dark:border-slate-500'}"
                  >
                    {app}
                  </button>
                {/each}
              </div>
            </div>
            {/if}
            
            {#if runningApps.length > 0}
            <div>
              <span class="block text-xs text-slate-400 mb-1.5">🟢 运行中</span>
              <div class="flex flex-wrap gap-1.5">
                {#each runningApps.slice(0, 8) as app}
                  <button
                    on:click={() => selectApp(app)}
                    class="px-2.5 py-1 text-xs rounded-lg transition-all
                           {selectedApp === app 
                             ? 'bg-primary-500 text-white' 
                             : 'bg-white dark:bg-slate-600 text-slate-600 dark:text-slate-300 hover:bg-primary-50 dark:hover:bg-slate-500 border border-slate-200 dark:border-slate-500'}"
                  >
                    {app}
                  </button>
                {/each}
              </div>
            </div>
            {/if}
          </div>
          {/if}

          <!-- 操作按钮 -->
          <div class="flex justify-end gap-2 mt-4 pt-3 border-t border-slate-200 dark:border-slate-600">
            <button
              on:click={() => { showAppInput = false; selectedApp = ''; }}
              class="px-4 py-1.5 text-sm text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 rounded-lg"
            >
              取消
            </button>
            <button
              on:click={addAppRule}
              class="px-4 py-1.5 text-sm bg-indigo-500 text-white rounded-lg hover:bg-indigo-600 disabled:opacity-50 transition-colors"
              disabled={!selectedApp}
            >
              添加规则
            </button>
          </div>
        </div>
      {/if}

      <!-- 已有规则列表 -->
      <div class="space-y-2">
        {#each config.privacy.app_rules as rule, i}
          <div class="flex items-center justify-between p-3 bg-slate-50 dark:bg-slate-700/30 rounded-lg group">
            <div class="flex items-center gap-3">
              <span class="text-sm font-medium text-slate-800 dark:text-white">{rule.app_name}</span>
              {#if rule.level === 'full'}
                <span class="text-xs px-2 py-0.5 rounded-full bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-400">完全记录</span>
              {:else if rule.level === 'anonymized'}
                <span class="text-xs px-2 py-0.5 rounded-full bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400">仅统计时长</span>
              {:else}
                <span class="text-xs px-2 py-0.5 rounded-full bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400">完全忽略</span>
              {/if}
            </div>
            <button
              on:click={() => removeAppRule(i)}
              class="text-xs text-slate-400 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-all"
            >
              删除
            </button>
          </div>
        {/each}
        {#if config.privacy.app_rules.length === 0}
          <p class="text-sm text-slate-400 text-center py-3">暂无特殊规则，所有应用默认完全记录</p>
        {/if}
      </div>
    </div>

    <hr class="border-slate-200 dark:border-slate-700" />

    <!-- 内容过滤（合并敏感词 + 域名黑名单） -->
    <div>
      <span class="block text-sm font-medium text-slate-700 dark:text-slate-300 mb-1">内容过滤</span>
      <p class="text-xs text-slate-500 mb-4">OCR 识别到的文字中包含敏感词时，该段文字不会被保存。黑名单域名的浏览活动不会被记录。</p>
      
      <!-- 敏感词 -->
      <div class="mb-4">
        <div class="flex items-center justify-between mb-2">
          <span class="text-xs font-medium text-slate-600 dark:text-slate-400">🔤 敏感词</span>
          <button
            on:click={() => showKeywordInput = !showKeywordInput}
            class="text-xs text-primary-600 hover:text-primary-700"
          >
            {showKeywordInput ? '收起' : '+ 添加'}
          </button>
        </div>
        
        {#if showKeywordInput}
          <div class="flex gap-2 mb-2 animate-fadeIn">
            <input
              type="text"
              bind:value={newKeyword}
              class="input flex-1 text-sm"
              placeholder="输入敏感词..."
              on:keydown={(e) => e.key === 'Enter' && addKeyword()}
            />
            <button
              on:click={addKeyword}
              class="px-3 py-1.5 text-sm bg-primary-500 text-white rounded-lg hover:bg-primary-600"
            >
              添加
            </button>
          </div>
        {/if}

        <div class="flex flex-wrap gap-1.5">
          {#each config.privacy.sensitive_keywords as keyword, i}
            <div class="inline-flex items-center px-2.5 py-1 rounded-full bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-300 text-xs group">
              <span>{keyword}</span>
              <button
                on:click={() => removeKeyword(i)}
                class="ml-1.5 text-slate-400 hover:text-red-500 opacity-50 group-hover:opacity-100 transition-opacity"
              >
                ×
              </button>
            </div>
          {/each}
          {#if config.privacy.sensitive_keywords.length === 0}
            <span class="text-xs text-slate-400">暂无敏感词</span>
          {/if}
        </div>
        <!-- 敏感词匹配说明 -->
        <p class="text-xs text-slate-400 mt-2">OCR 识别内容包含该词时自动过滤，不区分大小写</p>
      </div>

      <!-- 域名黑名单 -->
      <div>
        <div class="flex items-center justify-between mb-2">
          <span class="text-xs font-medium text-slate-600 dark:text-slate-400">🌐 域名黑名单</span>
          <button
            on:click={() => showDomainInput = !showDomainInput}
            class="text-xs text-primary-600 hover:text-primary-700"
          >
            {showDomainInput ? '收起' : '+ 添加'}
          </button>
        </div>
        
        {#if showDomainInput}
          <div class="flex gap-2 mb-2 animate-fadeIn">
            <input
              type="text"
              bind:value={newDomain}
              class="input flex-1 text-sm"
              placeholder="例如: example.com"
              on:keydown={(e) => e.key === 'Enter' && addDomain()}
            />
            <button
              on:click={addDomain}
              class="px-3 py-1.5 text-sm bg-primary-500 text-white rounded-lg hover:bg-primary-600"
            >
              添加
            </button>
          </div>
        {/if}

        <div class="flex flex-wrap gap-1.5">
          {#each (config.privacy.excluded_domains || []) as domain, i}
            <div class="inline-flex items-center px-2.5 py-1 rounded-full bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-xs group">
              <span>{domain}</span>
              <button
                on:click={() => removeDomain(i)}
                class="ml-1.5 text-red-400 hover:text-red-600 opacity-50 group-hover:opacity-100 transition-opacity"
              >
                ×
              </button>
            </div>
          {/each}
          {#if (config.privacy.excluded_domains || []).length === 0}
            <span class="text-xs text-slate-400">暂无黑名单域名</span>
          {/if}
        </div>
        <!-- 域名黑名单格式说明 -->
        <p class="text-xs text-slate-400 mt-2">填写完整域名如 <code class="bg-slate-100 dark:bg-slate-700 px-1 rounded">example.com</code>，该域名下所有页面均不会被记录</p>
      </div>
    </div>
  </div>
</div>
