<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { emitTo, listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import AvatarCanvas from '../../lib/components/Avatar/AvatarCanvas.svelte';
  import AvatarFollowupCard from '../../lib/components/Avatar/AvatarFollowupCard.svelte';
  import AvatarPopover from '../../lib/components/Avatar/AvatarPopover.svelte';
  import { applyLocaleToDocument, initializeLocale, locale, t } from '$lib/i18n/index.js';
  import {
    getAvatarMotionStepDelay,
    getAvatarStateBubble,
    getAvatarTransitionMeta,
  } from '../../lib/components/Avatar/avatarStateMeta.js';

  const appWindow = getCurrentWebviewWindow();
  const nativeWindow = getCurrentWindow();

  let state = {
    mode: 'idle',
    appName: 'Work Review',
    contextLabel: '待命中',
    hint: '准备陪你开始工作',
    isIdle: true,
    isGeneratingReport: false,
    avatarOpacity: 0.82,
    avatarPreset: 'original-standard',
    avatarPersona: 'assistant',
  };
  let inputActivity = {
    keyboardActive: false,
    mouseActive: false,
    keyboardGroup: 'idle',
    keyboardVisualKey: '',
    mouseGroup: 'idle',
    cursorRatioX: 0.5,
    cursorRatioY: 0.5,
    lastKeyboardInputAtMs: 0,
    lastMouseInputAtMs: 0,
  };
  let bubbleSource = null;
  let bubble = null;
  let bubbleTimer = null;
  let followup = null;
  let focusSession = null;
  let focusTimer = null;
  let focusNowMs = 0;
  let lastStateBubbleAt = 0;
  let transitionClass = '';
  let transitionTimer = null;
  let motionBeat = 0;
  let motionTimer = null;
  let positionSaveTimer = null;
  let lastSavedPositionKey = null;
  let unsubscribeLocale = () => {};
  let handleVisibilityChange = null;
  let handleContextMenu = null;
  let handleKeydown = null;
  let avatarExpanded = null;
  $: currentLocale = $locale;

  const RUNTIME_BUBBLE_MESSAGES = {
    __avatar_nudge_switch_companion__: {
      'zh-CN': '你切得有点快，我陪你回主线。',
      'zh-TW': '你切得有點快，我陪你回主線。',
      en: 'You are switching fast. Let us get back to the main thread.',
    },
    __avatar_nudge_switch_assistant__: {
      'zh-CN': '切换有点频繁，建议先回到当前主线。',
      'zh-TW': '切換有點頻繁，建議先回到目前主線。',
      en: 'Lots of switching. It may help to return to the current thread first.',
    },
    __avatar_nudge_switch_coach__: {
      'zh-CN': '别再切了，先把手上这段收住。',
      'zh-TW': '別再切了，先把手上這段收住。',
      en: 'Enough switching. Close this stretch before moving on.',
    },
    '先放松一下，待会再继续推进。': {
      'zh-CN': '先放松一下，待会再继续推进。',
      'zh-TW': '先放鬆一下，待會再繼續推進。',
      en: 'Take a short break, then continue when you are ready.',
    },
    '该休息一下了，起来活动活动吧。': {
      'zh-CN': '该休息一下了，起来活动活动吧。',
      'zh-TW': '該休息一下了，起來活動活動吧。',
      en: 'Time for a break. Stand up and stretch a bit.',
    },
    '开始整理日报，稍等我一下。': {
      'zh-CN': '开始整理日报，稍等我一下。',
      'zh-TW': '開始整理日報，稍等我一下。',
      en: "I'm preparing your daily report. Give me a moment.",
    },
    '日报整理好了，可以回来看看。': {
      'zh-CN': '日报整理好了，可以回来看看。',
      'zh-TW': '日報整理好了，可以回來看看。',
      en: 'Your daily report is ready. You can check it now.',
    },
    '这次日报整理失败了，稍后可以再试。': {
      'zh-CN': '这次日报整理失败了，稍后可以再试。',
      'zh-TW': '這次日報整理失敗了，稍後可以再試。',
      en: 'This report run failed. Please try again later.',
    },
  };

  const FOLLOWUP_PERSONA_LABEL_KEY = {
    companion: 'settingsAppearance.avatarPersonaCompanionTitle',
    assistant: 'settingsAppearance.avatarPersonaAssistantTitle',
    coach: 'settingsAppearance.avatarPersonaCoachTitle',
  };

  const FOLLOWUP_LEAD_KEY = {
    companion: 'settingsAppearance.avatarFollowupCompanionLead',
    assistant: 'settingsAppearance.avatarFollowupAssistantLead',
    coach: 'settingsAppearance.avatarFollowupCoachLead',
  };

  const FOLLOWUP_PERSONA_THEME = {
    companion: {
      badgeClass: 'bg-emerald-500/12 text-emerald-700',
      primaryClass: 'bg-emerald-500 hover:bg-emerald-600 text-white',
      surfaceClass: 'border-emerald-200/95 bg-[linear-gradient(180deg,rgba(255,255,255,0.98),rgba(236,253,245,0.98))]',
      strategyKey: 'settingsAppearance.avatarFollowupCompanionStrategy',
      focusKey: 'settingsAppearance.avatarFollowupFocusCompanion',
      focusFullKey: 'settingsAppearance.avatarFollowupFocusFullCompanion',
      rememberKey: 'settingsAppearance.avatarFollowupRememberCompanion',
      rememberFullKey: 'settingsAppearance.avatarFollowupRememberFullCompanion',
      snoozeKey: 'settingsAppearance.avatarFollowupSnoozeCompanion',
      snoozeFullKey: 'settingsAppearance.avatarFollowupSnoozeFullCompanion',
      timelineOpeningKey: 'settingsAppearance.avatarFollowupTimelineOpeningCompanion',
      rememberedKey: 'settingsAppearance.avatarFollowupRememberedCompanion',
      snoozedKey: 'settingsAppearance.avatarFollowupSnoozedCompanion',
      focusStartedKey: 'settingsAppearance.avatarFollowupFocusStartedCompanion',
      focusStoppedKey: 'settingsAppearance.avatarFollowupFocusStoppedCompanion',
      focusFinishedKey: 'settingsAppearance.avatarFollowupFocusFinishedCompanion',
    },
    assistant: {
      badgeClass: 'bg-sky-500/12 text-sky-700',
      primaryClass: 'bg-sky-500 hover:bg-sky-600 text-white',
      surfaceClass: 'border-sky-200/95 bg-[linear-gradient(180deg,rgba(255,255,255,0.98),rgba(239,246,255,0.98))]',
      strategyKey: 'settingsAppearance.avatarFollowupAssistantStrategy',
      focusKey: 'settingsAppearance.avatarFollowupFocus',
      focusFullKey: 'settingsAppearance.avatarFollowupFocusFull',
      rememberKey: 'settingsAppearance.avatarFollowupRemember',
      rememberFullKey: 'settingsAppearance.avatarFollowupRememberFull',
      snoozeKey: 'settingsAppearance.avatarFollowupSnooze',
      snoozeFullKey: 'settingsAppearance.avatarFollowupSnoozeFull',
      timelineOpeningKey: 'settingsAppearance.avatarFollowupTimelineOpening',
      rememberedKey: 'settingsAppearance.avatarFollowupRemembered',
      snoozedKey: 'settingsAppearance.avatarFollowupSnoozed',
      focusStartedKey: 'settingsAppearance.avatarFollowupFocusStarted',
      focusStoppedKey: 'settingsAppearance.avatarFollowupFocusStopped',
      focusFinishedKey: 'settingsAppearance.avatarFollowupFocusFinished',
    },
    coach: {
      badgeClass: 'bg-amber-500/14 text-amber-800',
      primaryClass: 'bg-amber-500 hover:bg-amber-600 text-slate-950',
      surfaceClass: 'border-amber-200/95 bg-[linear-gradient(180deg,rgba(255,255,255,0.98),rgba(255,251,235,0.98))]',
      strategyKey: 'settingsAppearance.avatarFollowupCoachStrategy',
      focusKey: 'settingsAppearance.avatarFollowupFocusCoach',
      focusFullKey: 'settingsAppearance.avatarFollowupFocusFullCoach',
      rememberKey: 'settingsAppearance.avatarFollowupRememberCoach',
      rememberFullKey: 'settingsAppearance.avatarFollowupRememberFullCoach',
      snoozeKey: 'settingsAppearance.avatarFollowupSnoozeCoach',
      snoozeFullKey: 'settingsAppearance.avatarFollowupSnoozeFullCoach',
      timelineOpeningKey: 'settingsAppearance.avatarFollowupTimelineOpeningCoach',
      rememberedKey: 'settingsAppearance.avatarFollowupRememberedCoach',
      snoozedKey: 'settingsAppearance.avatarFollowupSnoozedCoach',
      focusStartedKey: 'settingsAppearance.avatarFollowupFocusStartedCoach',
      focusStoppedKey: 'settingsAppearance.avatarFollowupFocusStoppedCoach',
      focusFinishedKey: 'settingsAppearance.avatarFollowupFocusFinishedCoach',
    },
  };

  function localizeBacklogNudgeMessage(message, nextLocale) {
    if (!message?.startsWith('__avatar_backlog_nudge__:')) {
      return null;
    }

    const [, persona = 'assistant', countRaw = '0'] = message.split(':');
    const count = Number(countRaw) || 0;
    const key =
      persona === 'companion'
        ? 'settingsAppearance.avatarNudgeBacklogCompanion'
        : persona === 'coach'
          ? 'settingsAppearance.avatarNudgeBacklogCoach'
          : 'settingsAppearance.avatarNudgeBacklogAssistant';

    return t(key, { count, locale: nextLocale });
  }

  function localizeBubblePayload(payload, nextLocale = currentLocale) {
    if (!payload) {
      return null;
    }

    if (payload.clear) {
      return payload;
    }

    const localizedMessage =
      localizeBacklogNudgeMessage(payload.message, nextLocale)
      || RUNTIME_BUBBLE_MESSAGES[payload.message]?.[nextLocale]
      || payload.message;

    return {
      ...payload,
      message: localizedMessage,
    };
  }

  function formatFocusCountdown(ms) {
    const safeMs = Math.max(0, ms);
    const totalSeconds = Math.ceil(safeMs / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  }

  function buildFocusBubblePayload(session) {
    if (!session) {
      return null;
    }

    const countdown = formatFocusCountdown(session.endsAtMs - focusNowMs);
    return {
      message: t('settingsAppearance.avatarFollowupFocusActive', {
        countdown,
      }),
      persistent: true,
      tone: 'success',
    };
  }

  $: focusBubble = buildFocusBubblePayload(focusSession);
  $: bubble = localizeBubblePayload(focusBubble || bubbleSource, currentLocale);
  $: followupCopy = buildFollowupCopy(followup);
  $: syncAvatarExpansion(followup != null);

  async function syncAvatarExpansion(expanded) {
    if (avatarExpanded === expanded) {
      return;
    }
    const previous = avatarExpanded;
    avatarExpanded = expanded;
    try {
      await invoke('set_avatar_window_expanded', { expanded });
    } catch (e) {
      avatarExpanded = previous;
      console.error('更新桌宠窗口尺寸失败:', e);
    }
  }

  function clearBubble() {
    bubbleSource = null;
    clearTimeout(bubbleTimer);
    bubbleTimer = null;
  }

  function showBubble(payload) {
    if (payload?.clear) {
      clearBubble();
      return;
    }

    if (focusSession && !payload?.persistent) {
      return;
    }

    bubbleSource = payload;
    clearTimeout(bubbleTimer);

    if (!payload?.persistent) {
      bubbleTimer = setTimeout(() => {
        bubbleSource = null;
        bubbleTimer = null;
      }, payload?.durationMs ?? payload?.duration ?? 4200);
    }
  }

  function dismissBubble() {
    if (focusSession) {
      stopFocusSession(true);
      return;
    }
    clearBubble();
  }

  function getFollowupPersonaLabelKey(persona) {
    return FOLLOWUP_PERSONA_LABEL_KEY[persona] || FOLLOWUP_PERSONA_LABEL_KEY.assistant;
  }

  function getFollowupLeadKey(persona) {
    return FOLLOWUP_LEAD_KEY[persona] || FOLLOWUP_LEAD_KEY.assistant;
  }

  function formatFollowupAge(hours) {
    const normalizedHours = Number(hours) || 0;
    if (normalizedHours <= 1) {
      return t('settingsAppearance.avatarFollowupAgeRecent');
    }
    return t('settingsAppearance.avatarFollowupAgeHours', { count: normalizedHours });
  }

  function truncateFollowupTitle(title, maxLength = 34) {
    const normalized = typeof title === 'string' ? title.trim() : '';
    if (!normalized) {
      return '';
    }
    if (normalized.length <= maxLength) {
      return normalized;
    }
    return `${normalized.slice(0, Math.max(1, maxLength - 1)).trimEnd()}…`;
  }

  function buildFollowupCopy(payload) {
    if (!payload) {
      return null;
    }

    const theme =
      FOLLOWUP_PERSONA_THEME[payload.persona] || FOLLOWUP_PERSONA_THEME.assistant;

    return {
      title: t('settingsAppearance.avatarFollowupTitle'),
      personaLabel: t(getFollowupPersonaLabelKey(payload.persona)),
      summary: t(getFollowupLeadKey(payload.persona), {
        title: truncateFollowupTitle(payload.title),
      }),
      strategy: t(theme.strategyKey),
      meta: [
        payload.sourceApp,
        payload.intentLabel,
        formatFollowupAge(payload.sessionAgeHours),
      ].filter(Boolean).join(' · '),
      openTimeline: t('settingsAppearance.avatarFollowupOpenTimeline'),
      focus: t(theme.focusKey),
      focusFull: t(theme.focusFullKey),
      remember: t(theme.rememberKey),
      rememberFull: t(theme.rememberFullKey),
      snooze: t(theme.snoozeKey),
      snoozeFull: t(theme.snoozeFullKey),
      dismissLabel: t('settingsAppearance.avatarFollowupDismiss'),
      badgeClass: theme.badgeClass,
      primaryClass: theme.primaryClass,
      surfaceClass: theme.surfaceClass,
    };
  }

  function getFollowupTheme(persona) {
    return FOLLOWUP_PERSONA_THEME[persona] || FOLLOWUP_PERSONA_THEME.assistant;
  }

  function buildFollowupActionInput(action) {
    if (!followup) {
      return null;
    }

    return {
      action,
      projectKey: followup.projectKey,
      title: followup.title,
      date: followup.date,
      sourceApp: followup.sourceApp,
      sourceTitle: followup.sourceTitle,
      persona: followup.persona,
    };
  }

  async function submitFollowupAction(action) {
    const input = buildFollowupActionInput(action);
    if (!input) {
      return null;
    }

    const snapshot = { ...followup };
    await invoke('handle_avatar_followup_action', { input });
    return snapshot;
  }

  function clearFollowup() {
    followup = null;
  }

  function clearFocusTimer() {
    clearInterval(focusTimer);
    focusTimer = null;
  }

  function finishFocusSession() {
    const completedSession = focusSession;
    focusSession = null;
    focusNowMs = 0;
    clearFocusTimer();
    if (!completedSession) {
      return;
    }
    const theme = getFollowupTheme(state.avatarPersona);
    showBubble({
      message: t(theme.focusFinishedKey),
      tone: 'success',
      persistent: true,
    });
  }

  function ensureFocusTicking() {
    clearFocusTimer();
    if (!focusSession) {
      return;
    }
    focusNowMs = Date.now();
    focusTimer = setInterval(() => {
      focusNowMs = Date.now();
      if (focusSession && focusNowMs >= focusSession.endsAtMs) {
        finishFocusSession();
      }
    }, 1000);
  }

  function stopFocusSession(showEndedBubble = false) {
    if (!focusSession) {
      return;
    }
    focusSession = null;
    focusNowMs = 0;
    clearFocusTimer();
    if (showEndedBubble) {
      clearBubble();
      const theme = getFollowupTheme(state.avatarPersona);
      showBubble({
        message: t(theme.focusStoppedKey),
        tone: 'success',
      });
    }
  }

  async function startFollowupFocus() {
    try {
      const payload = await submitFollowupAction('focus');
      if (!payload) {
        return;
      }

      clearFollowup();
      focusSession = {
        projectKey: payload.projectKey,
        title: payload.title,
        endsAtMs: Date.now() + 25 * 60 * 1000,
      };
      clearBubble();
      ensureFocusTicking();
      const theme = getFollowupTheme(payload.persona);
      showBubble({
        message: t(theme.focusStartedKey),
        tone: 'success',
      });
    } catch (e) {
      console.error('桌宠开始专注失败:', e);
      showBubble({
        message: t('settingsAppearance.avatarFollowupActionFailed', { error: e }),
        persistent: true,
      });
    }
  }

  async function openFollowupTimeline() {
    try {
      const payload = await submitFollowupAction('timeline');
      if (!payload) {
        return;
      }

      clearFollowup();
      const theme = getFollowupTheme(payload.persona);
      showBubble({
        message: t(theme.timelineOpeningKey),
        tone: 'success',
      });
      await invoke('show_main_window', { sourceWindowLabel: appWindow.label });
      await emitTo('main', 'avatar-open-timeline', {
        date: payload.date,
        projectKey: payload.projectKey,
        title: payload.title,
      });
    } catch (e) {
      console.error('桌宠打开时间线失败:', e);
      showBubble({
        message: t('settingsAppearance.avatarFollowupActionFailed', { error: e }),
        persistent: true,
      });
    }
  }

  async function rememberFollowup() {
    try {
      const payload = await submitFollowupAction('remember');
      if (!payload) {
        return;
      }

      clearFollowup();
      const theme = getFollowupTheme(payload.persona);
      showBubble({
        message: t(theme.rememberedKey),
        tone: 'success',
      });
    } catch (e) {
      console.error('桌宠记为待跟进失败:', e);
      showBubble({
        message: t('settingsAppearance.avatarFollowupActionFailed', { error: e }),
        persistent: true,
      });
    }
  }

  async function snoozeFollowup() {
    try {
      const payload = await submitFollowupAction('snooze');
      if (!payload) {
        return;
      }

      clearFollowup();
      const theme = getFollowupTheme(payload.persona);
      showBubble({
        message: t(theme.snoozedKey),
        tone: 'success',
      });
    } catch (e) {
      console.error('桌宠稍后提醒失败:', e);
      showBubble({
        message: t('settingsAppearance.avatarFollowupActionFailed', { error: e }),
        persistent: true,
      });
    }
  }

  async function dismissFollowup() {
    try {
      await submitFollowupAction('dismiss');
    } catch (e) {
      console.error('关闭桌宠继续提醒失败:', e);
    } finally {
      clearFollowup();
    }
  }

  async function openMainWindow() {
    try {
      await invoke('show_main_window', { sourceWindowLabel: appWindow.label });
    } catch (e) {
      console.error('显示主窗口失败:', e);
    }
  }

  async function startAvatarDrag(event) {
    const originalEvent = event.detail?.originalEvent ?? event;

    if (originalEvent.button !== 0) {
      return;
    }

    originalEvent.preventDefault?.();

    try {
      await nativeWindow.startDragging();
    } catch (e) {
      console.error('拖动桌宠失败:', e);
    }
  }

  function scheduleAvatarPositionSave(position) {
    const nextX = Math.round(position.x);
    const nextY = Math.round(position.y);
    const nextKey = `${nextX},${nextY}`;

    clearTimeout(positionSaveTimer);
    positionSaveTimer = setTimeout(async () => {
      if (nextKey === lastSavedPositionKey) {
        return;
      }

      try {
        await invoke('save_avatar_position', { x: nextX, y: nextY });
        lastSavedPositionKey = nextKey;
      } catch (e) {
        console.error('保存桌宠位置失败:', e);
      }
    }, 240);
  }

  function scheduleNextMotionStep() {
    clearTimeout(motionTimer);
    if (document.hidden) {
      motionTimer = null;
      return;
    }
    const delay = getAvatarMotionStepDelay(state.mode, state.contextLabel, motionBeat);
    motionTimer = setTimeout(() => {
      motionBeat = (motionBeat + 1) % 96;
      scheduleNextMotionStep();
    }, delay);
  }

  onMount(() => {
    let unlistenState = () => {};
    let unlistenBubble = () => {};
    let unlistenFollowup = () => {};
    let unlistenInput = () => {};
    let unlistenMoved = () => {};
    let unlistenLocaleChanged = () => {};
    initializeLocale();
    unsubscribeLocale = locale.subscribe((nextLocale) => {
      applyLocaleToDocument(nextLocale);
    });
    if (!document.hidden) {
      scheduleNextMotionStep();
    }

    handleVisibilityChange = () => {
      if (document.hidden) {
        clearTimeout(motionTimer);
        motionTimer = null;
      } else {
        scheduleNextMotionStep();
      }
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);

    // 桌宠窗口不需要浏览器原生右键菜单和打印能力，避免误触后弹出系统界面。
    handleContextMenu = (event) => {
      event.preventDefault();
      event.stopPropagation();
    };
    handleKeydown = (event) => {
      if (
        (event.metaKey || event.ctrlKey) &&
        (event.key === 'p' || event.key === 'P')
      ) {
        event.preventDefault();
        event.stopPropagation();
      }
    };
    document.addEventListener('contextmenu', handleContextMenu);
    document.addEventListener('keydown', handleKeydown, true);

    (async () => {
      try {
        state = await invoke('get_avatar_state');
      } catch (e) {
        console.error('获取桌宠状态失败:', e);
      }

      unlistenState = await appWindow.listen('avatar-state-changed', (event) => {
        const nextState = event.payload;
        const stateChanged =
          nextState.mode !== state.mode || nextState.contextLabel !== state.contextLabel;
        const stateBubble = getAvatarStateBubble(
          nextState.mode,
          currentLocale,
          nextState.contextLabel,
          nextState.avatarPersona,
        );
        const transition = getAvatarTransitionMeta(
          state.mode,
          nextState.mode,
          state.contextLabel,
          nextState.contextLabel,
        );

        if (
          stateBubble &&
          stateChanged &&
          Date.now() - lastStateBubbleAt > 900
        ) {
          lastStateBubbleAt = Date.now();
          showBubble(stateBubble);
        }

        if (
          transition.className &&
          (
            nextState.mode !== state.mode ||
            nextState.contextLabel !== state.contextLabel
          )
        ) {
          transitionClass = transition.className;
          clearTimeout(transitionTimer);
          transitionTimer = setTimeout(() => {
            transitionClass = '';
            transitionTimer = null;
          }, transition.durationMs);
        }

        state = nextState;
        scheduleNextMotionStep();
      });

      unlistenBubble = await appWindow.listen('avatar-bubble', (event) => {
        showBubble(event.payload);
      });

      unlistenFollowup = await appWindow.listen('avatar-followup-suggestion', (event) => {
        followup = event.payload;
      });

      unlistenInput = await appWindow.listen('avatar-input-changed', (event) => {
        const payload = event.payload ?? {};
        inputActivity = {
          keyboardActive: !!payload.keyboardActive,
          mouseActive: !!payload.mouseActive,
          keyboardGroup: payload.keyboardGroup ?? 'idle',
          keyboardVisualKey: payload.keyboardVisualKey ?? '',
          mouseGroup: payload.mouseGroup ?? 'idle',
          cursorRatioX: payload.cursorRatioX ?? 0.5,
          cursorRatioY: payload.cursorRatioY ?? 0.5,
          lastKeyboardInputAtMs: payload.lastKeyboardInputAtMs ?? 0,
          lastMouseInputAtMs: payload.lastMouseInputAtMs ?? 0,
        };

        if (inputActivity.keyboardActive || inputActivity.mouseActive) {
          motionBeat = (motionBeat + 1) % 96;
          scheduleNextMotionStep();
        }
      });

      unlistenLocaleChanged = await listen('locale-changed', (event) => {
        const nextLocale = event.payload;
        if (typeof nextLocale === 'string' && nextLocale) {
          initializeLocale(nextLocale);
        }
      });

      unlistenMoved = await nativeWindow.onMoved(({ payload: position }) => {
        scheduleAvatarPositionSave(position);
      });
    })();

    return () => {
      clearTimeout(bubbleTimer);
      clearTimeout(transitionTimer);
      clearTimeout(positionSaveTimer);
      clearTimeout(motionTimer);
      clearFocusTimer();
      if (handleVisibilityChange) document.removeEventListener('visibilitychange', handleVisibilityChange);
      if (handleContextMenu) document.removeEventListener('contextmenu', handleContextMenu);
      if (handleKeydown) document.removeEventListener('keydown', handleKeydown, true);
      unsubscribeLocale();
      unlistenState();
      unlistenBubble();
      unlistenFollowup();
      unlistenInput();
      unlistenLocaleChanged();
      unlistenMoved();
    };
  });
</script>

<div class="relative h-screen w-screen overflow-visible bg-transparent select-none">
  <div class="absolute inset-x-0 top-0 h-[86px] overflow-visible">
    <AvatarPopover {bubble} onClose={dismissBubble} />
  </div>

  <AvatarFollowupCard
    followup={followup}
    copy={followupCopy}
    onTimeline={openFollowupTimeline}
    onFocus={startFollowupFocus}
    onRemember={rememberFollowup}
    onSnooze={snoozeFollowup}
    onDismiss={dismissFollowup}
  />

  <div class="absolute inset-x-0 bottom-0 top-[78px] flex items-end justify-center overflow-visible">
    <div class="h-full w-[82%]">
      <AvatarCanvas
        {state}
        {inputActivity}
        {transitionClass}
        {motionBeat}
        on:avatarpointerdown={startAvatarDrag}
        on:avataractivate={openMainWindow}
      />
    </div>
  </div>
</div>

<style>
  :global(:root),
  :global(html),
  :global(body) {
    background: transparent !important;
  }

  :global(body) {
    margin: 0;
    overflow: hidden;
  }
</style>
