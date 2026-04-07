<script>
  import { onDestroy, onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { marked } from 'marked';
  import { assistantStore, BASIC_ASSISTANT_MODEL_ID } from '../../lib/stores/assistant.js';
  import { formatDurationLocalized, locale, t, tm } from '$lib/i18n/index.js';

  marked.use({
    gfm: true,
    breaks: true,
  });

  let input = '';
  let sending = false;
  let error = null;
  let chatBody;
  let composer;
  let bottomAnchor;
  let messages = [];
  let unsubscribeAssistant = () => {};
  let destroyed = false;
  let stickToBottom = true;
  $: currentLocale = $locale;
  $: starterPrompts = tm('ask.starterPrompts') || [];

  // 模型选择器
  let modelProfiles = [];
  let selectedModelId = BASIC_ASSISTANT_MODEL_ID;

  const providerDisplayNames = {
    ollama: {
      'zh-CN': 'Ollama (本地)',
      en: 'Ollama (Local)',
      'zh-TW': 'Ollama（本機）',
    },
    openai: {
      'zh-CN': 'OpenAI / 兼容API',
      en: 'OpenAI / Compatible API',
      'zh-TW': 'OpenAI / 相容 API',
    },
    siliconflow: {
      'zh-CN': '硅基流动 SiliconFlow',
      en: 'SiliconFlow',
      'zh-TW': '矽基流動 SiliconFlow',
    },
    deepseek: {
      'zh-CN': 'DeepSeek',
      en: 'DeepSeek',
      'zh-TW': 'DeepSeek',
    },
    qwen: {
      'zh-CN': '通义千问 Qwen',
      en: 'Qwen',
      'zh-TW': '通義千問 Qwen',
    },
    zhipu: {
      'zh-CN': '智谱 ChatGLM',
      en: 'Zhipu ChatGLM',
      'zh-TW': '智譜 ChatGLM',
    },
    moonshot: {
      'zh-CN': '月之暗面 Kimi',
      en: 'Moonshot Kimi',
      'zh-TW': '月之暗面 Kimi',
    },
    doubao: {
      'zh-CN': '火山引擎 豆包',
      en: 'Doubao',
      'zh-TW': '火山引擎 豆包',
    },
    minimax: {
      'zh-CN': '稀宇科技 MiniMax',
      en: 'MiniMax',
      'zh-TW': '稀宇科技 MiniMax',
    },
    gemini: {
      'zh-CN': 'Google Gemini',
      en: 'Google Gemini',
      'zh-TW': 'Google Gemini',
    },
    claude: {
      'zh-CN': 'Anthropic Claude',
      en: 'Anthropic Claude',
      'zh-TW': 'Anthropic Claude',
    },
  };

  function localizedProviderName(providerId) {
    return providerDisplayNames[providerId]?.[currentLocale] || providerId || '';
  }

  function displayModelProfileName(profile) {
    if (!profile) return '';
    const localizedProvider = localizedProviderName(profile.model_config?.provider);
    const modelName = profile.model_config?.model?.trim();
    if (localizedProvider && modelName) {
      return `${localizedProvider} · ${modelName}`;
    }
    if (modelName) {
      return modelName;
    }
    return profile.name || '';
  }

  onMount(async () => {
    unsubscribeAssistant = assistantStore.subscribe((state) => {
      const nextMessages = state.messages || [];
      const previousCount = messages.length;
      const messageCountIncreased = nextMessages.length > previousCount;
      const latestMessage = nextMessages[nextMessages.length - 1];

      messages = nextMessages;
      selectedModelId = state.selectedModelId || BASIC_ASSISTANT_MODEL_ID;

      if (!nextMessages.length) {
        stickToBottom = true;
        return;
      }

      if (previousCount === 0) {
        void scrollToBottom('auto', 3);
        return;
      }

      if (messageCountIncreased && (stickToBottom || latestMessage?.role === 'user')) {
        void scrollToBottom(latestMessage?.role === 'assistant' ? 'smooth' : 'auto', 2);
      }
    });

    // 加载模型档案
    try {
      const config = await invoke('get_config');
      modelProfiles = config.text_model_profiles || [];
      if (
        selectedModelId !== BASIC_ASSISTANT_MODEL_ID &&
        !modelProfiles.some((profile) => profile.id === selectedModelId)
      ) {
        selectedModelId = BASIC_ASSISTANT_MODEL_ID;
        assistantStore.setSelectedModelId(BASIC_ASSISTANT_MODEL_ID);
      }
    } catch (e) {
      console.warn('加载模型配置失败:', e);
    }

    resizeComposer();
    await scrollToBottom('auto', 3);
    composer?.focus();
  });

  onDestroy(() => {
    destroyed = true;
    unsubscribeAssistant();
  });

  function sourceLabel(sourceType) {
    const labels = {
      activity: t('ask.referenceTypes.activity'),
      hourly_summary: t('ask.referenceTypes.hourly_summary'),
      daily_report: t('ask.referenceTypes.daily_report'),
    };
    return labels[sourceType] || sourceType;
  }

  // 已知的段落标题——后端模板和 AI 模型都可能输出这些词作为独立行
  const SECTION_TITLES = new Set([
    '结论', '依据', '关键发现', '本期概览', '重点工作',
    '核心观察', '风险与提醒', '下阶段建议', '工作复盘',
    '主要意图', '主要工作', '待跟进事项', '代表性 Session',
    '相关记录依据',
  ]);

  function normalizeAssistantContent(content) {
    const text = (content || '').replace(/\r\n/g, '\n').trim();
    if (!text) return '';

    const lines = text.split('\n');

    // ——— 第 1 步：去掉模板自引用句 ———
    const filtered = [];
    let inCodeBlock = false;
    for (const line of lines) {
      const t = line.trim();
      if (t.startsWith('```')) inCodeBlock = !inCodeBlock;
      if (!inCodeBlock && (
        t.includes('我基于周报复盘') ||
        t.includes('我基于意图识别') ||
        t.includes('我基于 Session 聚合') ||
        t.includes('我基于记忆检索')
      )) continue;
      filtered.push(line);
    }

    // ——— 第 2 步：逐行补全 markdown 格式（兼容已有部分格式的内容）———
    const result = [];
    inCodeBlock = false;

    for (let i = 0; i < filtered.length; i++) {
      const raw = filtered[i];
      const t = raw.trim();

      // 空行保留（段落分隔）
      if (!t) { result.push(''); continue; }

      // 代码块原样透传
      if (t.startsWith('```')) { inCodeBlock = !inCodeBlock; result.push(raw); continue; }
      if (inCodeBlock) { result.push(raw); continue; }

      // 已有 markdown 标题 → 保留
      if (/^#{1,6}\s/.test(t)) {
        result.push(raw);
        continue;
      }

      // 已有列表/引用标记 → 保留
      if (/^[-*+]\s/.test(t) || /^\d+\.\s/.test(t) || /^>\s/.test(t)) {
        result.push(raw);
        continue;
      }

      // 已知段落标题（无 # 前缀的纯文本）→ ## 标题
      if (SECTION_TITLES.has(t)) {
        result.push('', `## ${t}`, '');
        continue;
      }

      // "标题（说明）" 格式 → ### 副标题
      if (/^[^（）()。，！？]{2,20}[（(].+[）)]$/.test(t) && !t.includes('。')) {
        result.push('', `### ${t}`, '');
        continue;
      }

      // 短 key：value 数据行（key ≤ 8 字符，无句号结尾，总长 < 40）→ 列表项
      if (/^[^：。！？，]{1,8}：/.test(t) && !/[。！？]$/.test(t) && t.length < 40) {
        result.push(`- ${t}`);
        continue;
      }

      // 普通文本
      result.push(t);
    }

    return result.join('\n');
  }

  function renderMarkdown(content) {
    return marked.parse(normalizeAssistantContent(content));
  }

  function resizeComposer() {
    if (!composer) return;
    composer.style.height = '0px';
    composer.style.height = `${Math.min(composer.scrollHeight, 220)}px`;
  }

  function isNearBottom(threshold = 120) {
    if (!chatBody) return true;
    return chatBody.scrollHeight - chatBody.scrollTop - chatBody.clientHeight <= threshold;
  }

  function syncStickToBottom() {
    stickToBottom = isNearBottom();
  }

  async function scrollToBottom(behavior = 'smooth', attempts = 1) {
    await tick();
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      await new Promise((resolve) => requestAnimationFrame(resolve));
      if (bottomAnchor?.scrollIntoView) {
        bottomAnchor.scrollIntoView({ block: 'end', behavior });
      } else if (chatBody) {
        chatBody.scrollTop = chatBody.scrollHeight;
      }
    }
    stickToBottom = true;
  }

  function buildHistoryPayload() {
    return messages
      .filter((message) => message.role === 'user' || message.role === 'assistant')
      .slice(-8)
      .map((message) => ({
        role: message.role,
        content: message.content,
      }));
  }

  function getSelectedModelConfig() {
    if (selectedModelId === BASIC_ASSISTANT_MODEL_ID) {
      return null;
    }
    const profile = modelProfiles.find((p) => p.id === selectedModelId);
    return profile ? profile.model_config : null;
  }

  function handleModelChange(event) {
    selectedModelId = event.currentTarget.value;
    assistantStore.setSelectedModelId(selectedModelId);
  }

  async function clearConversation() {
    assistantStore.clearMessages();
    error = null;
    await tick();
    await scrollToBottom('auto', 2);
    composer?.focus();
  }

  async function submitQuestion(question = input) {
    const trimmed = question.trim();
    if (!trimmed || sending) return;

    error = null;
    sending = true;

    const history = buildHistoryPayload();

    assistantStore.appendMessage({
      role: 'user',
      content: trimmed,
    });

    input = '';
    resizeComposer();
    await tick();
    await scrollToBottom('auto', 2);

    try {
      const answer = await invoke('chat_work_assistant', {
        question: trimmed,
        history,
        modelConfig: getSelectedModelConfig(),
        locale: currentLocale,
      });

      // 写入全局 store（即使组件已销毁也安全）
      assistantStore.appendMessage({
        role: 'assistant',
        content: answer.answer,
        usedAi: answer.usedAi,
        modelName: answer.modelName,
        toolLabels: answer.toolLabels || [],
        references: answer.references || [],
      });
      if (!destroyed) {
        await scrollToBottom();
      }
    } catch (e) {
      if (!destroyed) {
        error = e.toString();
      }
    } finally {
      if (destroyed) return;
      sending = false;
      await tick();
      resizeComposer();
      composer?.focus();
    }
  }

  function handleComposerKeydown(event) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      submitQuestion();
    }
  }

  $: hasConversation = messages.length > 0;
  $: input, resizeComposer();
</script>

<div class="page-shell ask-workbench-shell h-full" data-locale={currentLocale}>
  <div class="ask-workbench-frame flex min-h-[calc(100vh-7rem)] flex-col">
    <div bind:this={chatBody} class="flex-1 overflow-y-auto px-4 pb-40 pt-10" on:scroll={syncStickToBottom}>
      {#if !hasConversation}
        <div class="ask-welcome-panel mx-auto flex min-h-full max-w-4xl flex-col items-center justify-center text-center">
          <span class="ask-kicker">{t('ask.title')}</span>
          <h1 class="mb-2 text-2xl font-semibold tracking-tight text-slate-800 dark:text-slate-100">{t('ask.title')}</h1>
          <p class="mb-10 text-sm text-slate-500 dark:text-slate-400">{t('ask.subtitle')}</p>
          <div class="ask-starter-grid grid w-full max-w-3xl gap-3 sm:grid-cols-2">
            {#each starterPrompts as prompt}
              <button
                class="ask-starter-card rounded-[28px] bg-[linear-gradient(180deg,rgba(255,255,255,0.92),rgba(248,250,252,0.88))] px-5 py-4 text-left text-sm font-medium leading-6 text-slate-700 ring-1 ring-inset ring-slate-200/80 shadow-[0_8px_24px_rgba(15,23,42,0.04)] transition hover:-translate-y-0.5 hover:text-slate-900 hover:ring-slate-300 dark:bg-[linear-gradient(180deg,rgba(15,23,42,0.78),rgba(15,23,42,0.62))] dark:text-slate-200 dark:ring-slate-700/80 dark:shadow-none dark:hover:text-white dark:hover:ring-slate-600"
                on:click={() => submitQuestion(prompt)}
                disabled={sending}
              >
                {prompt}
              </button>
            {/each}
          </div>
        </div>
      {:else}
        <div class="ask-thread-shell mx-auto flex min-h-full max-w-4xl flex-col gap-10">
          {#each messages as message}
            <div class={message.role === 'user' ? 'flex justify-end' : 'flex justify-start'}>
              <div
                class={message.role === 'user'
                  ? 'ask-message-card ask-message-card-user max-w-[78%] rounded-[28px] rounded-br-lg bg-gradient-to-br from-slate-100 to-slate-50 px-5 py-4 text-slate-800 ring-1 ring-inset ring-slate-200/60 shadow-sm dark:from-slate-800 dark:to-slate-900 dark:text-slate-100 dark:ring-slate-700/60'
                  : 'ask-message-card ask-message-card-assistant w-full max-w-[90%] px-1 py-1 text-slate-800 dark:text-slate-100'}
              >
                {#if message.role === 'assistant'}
                  <div class="markdown-body assistant-markdown max-w-none">
                    {@html renderMarkdown(message.content)}
                  </div>

                  {#if message.references?.length}
                    <details class="mt-6 rounded-[24px] bg-slate-50/74 px-4 py-3 ring-1 ring-inset ring-slate-200/60 dark:bg-slate-950/34 dark:ring-slate-800/70">
                      <summary class="cursor-pointer list-none text-sm font-medium text-slate-500 dark:text-slate-400">
                        {t('ask.references', { count: message.references.length })}
                      </summary>

                      <div class="mt-3 space-y-2">
                        {#each message.references as item}
                          <div class="ask-reference-card rounded-[20px] bg-white/88 px-3 py-3 ring-1 ring-inset ring-slate-200/70 dark:bg-slate-900/80 dark:ring-slate-800">
                            <div class="flex flex-wrap items-center gap-2 text-xs text-slate-400 dark:text-slate-500">
                              <span>{sourceLabel(item.sourceType)}</span>
                              <span>{item.date}</span>
                              {#if item.appName}
                                <span>{item.appName}</span>
                              {/if}
                              {#if item.duration}
                                <span>{formatDurationLocalized(item.duration)}</span>
                              {/if}
                            </div>
                            <div class="mt-1 text-sm font-medium text-slate-800 dark:text-slate-100">{item.title}</div>
                            {#if item.excerpt}
                              <div class="mt-1 text-sm leading-6 text-slate-500 dark:text-slate-400">{item.excerpt}</div>
                            {/if}
                          </div>
                        {/each}
                      </div>
                    </details>
                  {/if}
                {:else}
                  <p class="whitespace-pre-wrap text-[16px] font-medium leading-7 tracking-[0.01em]">{message.content}</p>
                {/if}
              </div>
            </div>
          {/each}

          <!-- Loading bubble -->
          {#if sending}
            <div class="flex justify-start">
              <div class="rounded-[24px] bg-slate-50/80 px-5 py-4 ring-1 ring-inset ring-slate-200/50 dark:bg-slate-800/60 dark:ring-slate-700/50">
                <div class="flex items-center gap-1.5">
                  <span class="inline-block h-2 w-2 animate-pulse rounded-full bg-slate-400 dark:bg-slate-500"></span>
                  <span class="inline-block h-2 w-2 animate-pulse rounded-full bg-slate-400 dark:bg-slate-500" style="animation-delay: 0.2s"></span>
                  <span class="inline-block h-2 w-2 animate-pulse rounded-full bg-slate-400 dark:bg-slate-500" style="animation-delay: 0.4s"></span>
                </div>
              </div>
            </div>
          {/if}

          <!-- Error callout -->
          {#if error}
            <div class="flex items-start gap-3 rounded-[24px] border border-rose-200 bg-rose-50/80 px-5 py-4 dark:border-rose-900/60 dark:bg-rose-950/30">
              <svg class="mt-0.5 h-5 w-5 shrink-0 text-rose-500 dark:text-rose-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
              </svg>
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium text-rose-700 dark:text-rose-300">{t('ask.requestFailed')}</p>
                <p class="mt-1 text-sm text-rose-600 dark:text-rose-400">{error}</p>
              </div>
            </div>
          {/if}

          <div bind:this={bottomAnchor} class="h-px w-full"></div>
        </div>
      {/if}
    </div>

    <div class="pointer-events-none sticky bottom-0 bg-gradient-to-t from-slate-50 via-slate-50/90 to-transparent px-4 pb-4 pt-8 dark:from-slate-950 dark:via-slate-950/84">
      <div class="pointer-events-auto mx-auto max-w-4xl">
        <div class="ask-composer-shell rounded-[30px] border border-slate-200/70 bg-white/94 px-4 py-3 shadow-[0_12px_32px_rgba(15,23,42,0.08)] backdrop-blur dark:border-slate-700/70 dark:bg-slate-900/88 dark:shadow-[0_12px_32px_rgba(2,6,23,0.32)]">
          <textarea
            bind:this={composer}
            bind:value={input}
            rows="1"
            class="max-h-[220px] min-h-[26px] w-full resize-none bg-transparent text-[15px] leading-7 text-slate-800 outline-none placeholder:text-slate-400 dark:text-slate-100 dark:placeholder:text-slate-500"
            placeholder={t('ask.placeholder')}
            on:input={resizeComposer}
            on:keydown={handleComposerKeydown}
          />

          <div class="mt-3 flex items-center justify-between gap-3 border-t border-slate-200/60 pt-2.5 dark:border-slate-700/60">
            <div class="flex min-w-0 flex-1 items-center gap-2">
              <select
                bind:value={selectedModelId}
                on:change={handleModelChange}
                class="h-8 min-w-[122px] max-w-[176px] cursor-pointer appearance-none rounded-full border border-slate-200/80 bg-slate-100/90 px-3 pr-8 text-[11px] font-medium text-slate-600 shadow-[inset_0_1px_0_rgba(255,255,255,0.5)] outline-none transition hover:bg-slate-200/70 focus:ring-2 focus:ring-slate-300 dark:border-slate-700/80 dark:bg-slate-800/70 dark:text-slate-300 dark:hover:bg-slate-700/80 dark:focus:ring-slate-600"
                style="background-image: url(&quot;data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E&quot;); background-repeat: no-repeat; background-position: right 10px center;"
                aria-label={t('ask.modelSelector')}
              >
                <option value={BASIC_ASSISTANT_MODEL_ID}>{t('ask.basicTemplate')}</option>
                {#each modelProfiles as profile}
                  <option value={profile.id}>{displayModelProfileName(profile) || t('ask.aiEnhanced')}</option>
                {/each}
              </select>

              <span class="mx-0.5 h-4 w-px shrink-0 bg-slate-200/60 dark:bg-slate-700/60"></span>

              {#if sending}
                <span class="shrink-0 text-[11px] text-slate-400 dark:text-slate-500">{t('ask.thinking')}</span>
              {:else}
                <button
                  type="button"
                  class="shrink-0 rounded-full px-2.5 py-1 text-[11px] text-slate-400 transition hover:bg-slate-100/80 hover:text-slate-600 dark:text-slate-500 dark:hover:bg-slate-800/70 dark:hover:text-slate-300"
                  on:click={clearConversation}
                  disabled={!hasConversation}
                >
                  {t('ask.clearing')}
                </button>
              {/if}
            </div>

            <button
              class="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-slate-900 text-white transition hover:scale-[1.02] hover:bg-slate-800 disabled:cursor-not-allowed disabled:bg-slate-300 dark:bg-slate-100 dark:text-slate-900 dark:hover:bg-white dark:disabled:bg-slate-700 dark:disabled:text-slate-400"
              on:click={() => submitQuestion()}
              disabled={sending || !input.trim()}
              aria-label={sending ? t('ask.sending') : t('ask.sendMessage')}
              title={sending ? t('ask.sending') : t('ask.sendMessage')}
            >
              {#if sending}
                <svg class="h-4 w-4 animate-spin" viewBox="0 0 24 24" fill="none">
                  <circle class="opacity-25" cx="12" cy="12" r="9" stroke="currentColor" stroke-width="2.5"></circle>
                  <path class="opacity-90" d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"></path>
                </svg>
              {:else}
                <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none">
                  <path d="M12 17V7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"></path>
                  <path d="M8 11L12 7L16 11" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"></path>
                </svg>
              {/if}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
