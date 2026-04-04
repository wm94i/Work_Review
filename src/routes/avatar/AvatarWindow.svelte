<script>
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import AvatarCanvas from '../../lib/components/Avatar/AvatarCanvas.svelte';
  import AvatarPopover from '../../lib/components/Avatar/AvatarPopover.svelte';
  import { applyLocaleToDocument, initializeLocale, locale } from '$lib/i18n/index.js';
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
  };
  let bubbleSource = null;
  let bubble = null;
  let bubbleTimer = null;
  let lastStateBubbleAt = 0;
  let transitionClass = '';
  let transitionTimer = null;
  let motionBeat = 0;
  let motionTimer = null;
  let positionSaveTimer = null;
  let lastSavedPositionKey = null;
  let unsubscribeLocale = () => {};
  let unlistenLocaleChanged = () => {};
  let handleVisibilityChange = null;
  let handleContextMenu = null;
  let handleKeydown = null;
  $: currentLocale = $locale;

  const RUNTIME_BUBBLE_MESSAGES = {
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

  function localizeBubblePayload(payload, nextLocale = currentLocale) {
    if (!payload) {
      return null;
    }

    if (payload.clear) {
      return payload;
    }

    const localizedMessage =
      RUNTIME_BUBBLE_MESSAGES[payload.message]?.[nextLocale]
      || payload.message;

    return {
      ...payload,
      message: localizedMessage,
    };
  }

  $: bubble = localizeBubblePayload(bubbleSource, currentLocale);

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
    clearBubble();
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
    let unlistenMoved = () => {};
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
        const stateBubble = getAvatarStateBubble(nextState.mode, currentLocale);
        const transition = getAvatarTransitionMeta(
          state.mode,
          nextState.mode,
          state.contextLabel,
          nextState.contextLabel,
        );

        if (
          stateBubble &&
          nextState.mode !== state.mode &&
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

      unlistenLocaleChanged = await listen('locale-changed', (event) => {
        initializeLocale(event.payload);
      }, {
        target: { kind: 'WebviewWindow', label: appWindow.label }
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
      if (handleVisibilityChange) document.removeEventListener('visibilitychange', handleVisibilityChange);
      if (handleContextMenu) document.removeEventListener('contextmenu', handleContextMenu);
      if (handleKeydown) document.removeEventListener('keydown', handleKeydown, true);
      unsubscribeLocale();
      unlistenLocaleChanged();
      unlistenState();
      unlistenBubble();
      unlistenMoved();
    };
  });
</script>

<div class="relative h-screen w-screen overflow-visible bg-transparent select-none">
  <AvatarPopover {bubble} onClose={dismissBubble} />

  <div class="h-full w-[54%]">
    <AvatarCanvas
      {state}
      {transitionClass}
      {motionBeat}
      on:avatarpointerdown={startAvatarDrag}
      on:avataractivate={openMainWindow}
    />
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
