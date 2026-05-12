<script>
  import { createEventDispatcher } from 'svelte';
  import { locale, t } from '$lib/i18n/index.js';
  
  export let config;
  export let runningApps = [];
  export let recentApps = [];
  
  const dispatch = createEventDispatcher();
  $: currentLocale = $locale;
  let privacyLevels = [];
  
  // 内联输入状态
  let showAppInput = false;
  let selectedApp = '';
  let selectedLevel = 'ignored';
  let batchSelectedApps = new Set();
  let appSearchQuery = '';
  let showAllRecentApps = false;
  let showAllRunningApps = false;
  let showKeywordInput = false;
  let newKeyword = '';
  let showDomainInput = false;
  let newDomain = '';

  // 隐私级别定义 - 使用文字标签避免 emoji 渲染异常
  const privacyLevelConfigs = [
    { value: 'full', labelKey: 'settingsPrivacy.full', descKey: 'settingsPrivacy.fullDesc', textClass: 'settings-text-success', chipClass: 'settings-chip-success', activeClass: 'settings-segment-success' },
    { value: 'anonymized', labelKey: 'settingsPrivacy.anonymized', descKey: 'settingsPrivacy.anonymizedDesc', textClass: 'settings-text-warn', chipClass: 'settings-chip-warn', activeClass: 'settings-segment-warn' },
    { value: 'ignored', labelKey: 'settingsPrivacy.ignored', descKey: 'settingsPrivacy.ignoredDesc', textClass: 'settings-text-danger', chipClass: 'settings-chip-danger', activeClass: 'settings-segment-danger' },
  ];
  $: {
    currentLocale;
    privacyLevels = privacyLevelConfigs.map((level) => ({
      ...level,
      label: t(level.labelKey),
      desc: t(level.descKey),
    }));
  }

  function addAppRule() {
    const appsToAdd = new Set(batchSelectedApps);
    if (selectedApp) appsToAdd.add(selectedApp);
    if (appsToAdd.size === 0) return;
    let rules = [...config.privacy.app_rules];
    for (const appName of appsToAdd) {
      const existingIndex = rules.findIndex(r => r.app_name === appName);
      if (existingIndex >= 0) {
        rules[existingIndex].level = selectedLevel;
      } else {
        rules.push({ app_name: appName, level: selectedLevel });
      }
    }
    config.privacy.app_rules = rules;
    showAppInput = false;
    selectedApp = '';
    batchSelectedApps = new Set();
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
    if (config.privacy.excluded_keywords.includes(newKeyword.trim())) {
      newKeyword = '';
      return;
    }
    config.privacy.excluded_keywords = [
      ...config.privacy.excluded_keywords,
      newKeyword.trim()
    ];
    newKeyword = '';
    showKeywordInput = false;
    dispatch('change', config);
  }

  function removeKeyword(index) {
    const keywords = [...config.privacy.excluded_keywords];
    keywords.splice(index, 1);
    config.privacy.excluded_keywords = keywords;
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

  // 快捷选择应用（多选切换）
  function toggleBatchApp(appName) {
    if (batchSelectedApps.has(appName)) {
      batchSelectedApps.delete(appName);
    } else {
      batchSelectedApps.add(appName);
    }
    batchSelectedApps = batchSelectedApps; // 触发响应式更新
  }
</script>

<div class="settings-card mb-5" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsPrivacy.title')}</h3>
  <p class="settings-card-desc">{t('settingsPrivacy.description')}</p>
  
  <div class="settings-section">
    <!-- 应用规则 -->
    <div>
      <div class="flex items-center justify-between mb-1">
        <span class="settings-text">
          {t('settingsPrivacy.appRules')}
        </span>
        <button
          on:click={() => { const opening = !showAppInput; showAppInput = opening; if (opening) dispatch('refresh-apps'); appSearchQuery = ''; showAllRecentApps = false; showAllRunningApps = false; } }
          class="settings-link-action text-sm"
        >
          {showAppInput ? t('settingsPrivacy.collapse') : t('settingsPrivacy.addRule')}
        </button>
      </div>
      <p class="settings-muted mb-3">{t('settingsPrivacy.appRulesHint')}</p>

      {#if showAppInput}
        <div class="settings-panel mb-3 animate-fadeIn">
          <!-- 应用名称输入 -->
          <div class="settings-field mb-3">
            <label for="app-name-input" class="settings-label">{t('settingsPrivacy.appInputLabel')}</label>
            <input
              id="app-name-input"
              type="text"
              bind:value={selectedApp}
              class="control-input"
              placeholder={t('settingsPrivacy.appPlaceholder')}
            />
          </div>
          <!-- 策略选择：分段按钮 -->
          <div class="settings-field mb-3">
            <span class="settings-label">{t('settingsPrivacy.strategy')}</span>
            <div class="flex gap-2">
              {#each privacyLevels as level}
                <button
                  on:click={() => selectedLevel = level.value}
                  class="segment-btn border border-slate-200 dark:border-slate-600
                         {selectedLevel === level.value
                           ? level.activeClass
                           : 'settings-segment-idle'}"
                >
                  {level.label}
                </button>
              {/each}
            </div>
            <p class="text-xs mt-1.5 {privacyLevels.find(l => l.value === selectedLevel)?.textClass || 'settings-subtle'}">
              {privacyLevels.find(l => l.value === selectedLevel)?.desc || ''}
            </p>
          </div>
          
          <!-- 快捷选择 -->
          {#if recentApps.length > 0 || runningApps.length > 0}
          <div class="settings-block">
            <!-- 搜索过滤 -->
            {#if recentApps.length + runningApps.length > 10}
            <div class="mb-2">
              <input
                type="text"
                bind:value={appSearchQuery}
                class="control-input text-xs"
                placeholder={t('settingsPrivacy.searchApps') || '搜索应用...'}
              />
            </div>
            {/if}

            {#if runningApps.length > 0}
            <div class="mb-2">
              <span class="settings-subtle block mb-1.5">{t('settingsPrivacy.runningApps')}</span>
              <div class="flex flex-wrap gap-1.5">
                {#each runningApps
                  .filter(app => !appSearchQuery || app.toLowerCase().includes(appSearchQuery.toLowerCase()))
                  .slice(0, showAllRunningApps ? undefined : 8) as app}
                  <button
                    on:click={() => toggleBatchApp(app)}
                    class="settings-chip-button
                           {batchSelectedApps.has(app)
                             ? 'settings-chip-button-active'
                             : ''}"
                  >
                    {batchSelectedApps.has(app) ? '✓ ' : ''}{app}
                  </button>
                {/each}
              </div>
              {#if runningApps.length > 8 && !appSearchQuery}
                <button
                  on:click={() => showAllRunningApps = !showAllRunningApps}
                  class="text-xs text-primary-500 hover:text-primary-600 mt-1"
                >
                  {showAllRunningApps
                    ? t('settingsPrivacy.collapse') || '收起'
                    : `+${runningApps.length - 8} ${t('settingsPrivacy.moreApps') || '更多'}`}
                </button>
              {/if}
            </div>
            {/if}

            {#if recentApps.length > 0}
            <div>
              <span class="settings-subtle block mb-1.5">{t('settingsPrivacy.historyApps')}</span>
              <div class="flex flex-wrap gap-1.5">
                {#each recentApps
                  .filter(app => !appSearchQuery || app.toLowerCase().includes(appSearchQuery.toLowerCase()))
                  .slice(0, showAllRecentApps ? undefined : 12) as app}
                  <button
                    on:click={() => toggleBatchApp(app)}
                    class="settings-chip-button
                           {batchSelectedApps.has(app)
                             ? 'settings-chip-button-active'
                             : ''}"
                  >
                    {batchSelectedApps.has(app) ? '✓ ' : ''}{app}
                  </button>
                {/each}
              </div>
              {#if recentApps.length > 12 && !appSearchQuery}
                <button
                  on:click={() => showAllRecentApps = !showAllRecentApps}
                  class="text-xs text-primary-500 hover:text-primary-600 mt-1"
                >
                  {showAllRecentApps
                    ? t('settingsPrivacy.collapse') || '收起'
                    : `+${recentApps.length - 12} ${t('settingsPrivacy.moreApps') || '更多'}`}
                </button>
              {/if}
            </div>
            {/if}
          </div>
          {/if}

          <!-- 操作按钮 -->
          <div class="settings-actions mt-4">
            <button
              on:click={() => { showAppInput = false; selectedApp = ''; }}
              class="settings-action-secondary"
            >
              {t('common.cancel')}
            </button>
            <button
              on:click={addAppRule}
              class="settings-action-primary"
              disabled={!selectedApp && batchSelectedApps.size === 0}
            >
              {batchSelectedApps.size > 1
                ? t('settingsPrivacy.batchAdd', { count: batchSelectedApps.size })
                : t('settingsPrivacy.addRuleAction')}
            </button>
          </div>
        </div>
      {/if}

      <!-- 已有规则列表：按隐私级别分组，chip 紧凑布局 -->
      {#if config.privacy.app_rules.length > 0}
        <div class="space-y-3">
          {#each privacyLevels as level}
            {@const groupRules = config.privacy.app_rules
              .map((r, i) => ({ ...r, _idx: i }))
              .filter(r => r.level === level.value)}
            {#if groupRules.length > 0}
              <div>
                <span class="text-xs font-medium {level.textClass} mb-1 block">{level.label}</span>
                <div class="flex flex-wrap gap-1.5">
                  {#each groupRules as rule}
                    <div class="settings-chip-neutral group">
                      <span>{rule.app_name}</span>
                      <button
                        on:click={() => removeAppRule(rule._idx)}
                        class="ml-1.5 text-slate-400 hover:text-red-500 opacity-50 group-hover:opacity-100 transition-opacity"
                      >
                        ×
                      </button>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          {/each}
        </div>
      {:else}
        <p class="settings-empty">{t('settingsPrivacy.noRules')}</p>
      {/if}
    </div>

    <hr class="border-slate-200 dark:border-slate-700" />

    <!-- 内容过滤（合并敏感词 + 域名黑名单） -->
    <div>
      <span class="settings-text block mb-1">{t('settingsPrivacy.contentFilter')}</span>
      <p class="settings-muted mb-4">{t('settingsPrivacy.contentFilterDesc')}</p>
      
      <!-- 敏感词 -->
      <div class="mb-4">
        <div class="flex items-center justify-between mb-2">
          <span class="settings-label">{t('settingsPrivacy.keywords')}</span>
          <button
            on:click={() => showKeywordInput = !showKeywordInput}
            class="settings-link-action"
          >
            {showKeywordInput ? t('settingsPrivacy.collapse') : t('settingsPrivacy.add')}
          </button>
        </div>
        
        {#if showKeywordInput}
          <div class="flex gap-2 mb-2 animate-fadeIn">
            <input
              type="text"
              bind:value={newKeyword}
              class="control-input flex-1"
              placeholder={t('settingsPrivacy.keywordPlaceholder')}
              on:keydown={(e) => e.key === 'Enter' && addKeyword()}
            />
            <button
              on:click={addKeyword}
              class="settings-action-primary"
            >
              {t('common.add')}
            </button>
          </div>
        {/if}

        <div class="flex flex-wrap gap-1.5">
          {#each config.privacy.excluded_keywords as keyword, i}
            <div class="settings-chip-neutral group">
              <span>{keyword}</span>
              <button
                on:click={() => removeKeyword(i)}
                class="ml-1.5 text-slate-400 hover:text-red-500 opacity-50 group-hover:opacity-100 transition-opacity"
              >
                ×
              </button>
            </div>
          {/each}
          {#if config.privacy.excluded_keywords.length === 0}
            <span class="settings-subtle">{t('settingsPrivacy.noKeywords')}</span>
          {/if}
        </div>
        <!-- 敏感词匹配说明 -->
        <p class="settings-note">{t('settingsPrivacy.keywordHint')}</p>
      </div>

      <!-- 域名黑名单 -->
      <div>
        <div class="flex items-center justify-between mb-2">
          <span class="settings-label">{t('settingsPrivacy.domainBlacklist')}</span>
          <button
            on:click={() => showDomainInput = !showDomainInput}
            class="settings-link-action"
          >
            {showDomainInput ? t('settingsPrivacy.collapse') : t('settingsPrivacy.add')}
          </button>
        </div>
        
        {#if showDomainInput}
          <div class="flex gap-2 mb-2 animate-fadeIn">
            <input
              type="text"
              bind:value={newDomain}
              class="control-input flex-1"
              placeholder={t('settingsPrivacy.domainPlaceholder')}
              on:keydown={(e) => e.key === 'Enter' && addDomain()}
            />
            <button
              on:click={addDomain}
              class="settings-action-primary"
            >
              {t('common.add')}
            </button>
          </div>
        {/if}

        <div class="flex flex-wrap gap-1.5">
          {#each (config.privacy.excluded_domains || []) as domain, i}
            <div class="settings-chip-danger group">
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
            <span class="settings-subtle">{t('settingsPrivacy.noDomains')}</span>
          {/if}
        </div>
        <!-- 域名黑名单格式说明 -->
        <p class="settings-note">{t('settingsPrivacy.domainHint')}</p>
      </div>
    </div>
  </div>
</div>
