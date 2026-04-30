<script>
  import { onMount, onDestroy } from 'svelte';
  import { createEventDispatcher } from 'svelte';
  import { Live2DRenderer } from './live2dRenderer.js';
  import {
    getAvatarPresetDefinition,
    normalizeAvatarPresetId,
  } from './avatarPresetRegistry.js';
  import { getAvatarIdleMotionMeta, getAvatarModeMeta } from './avatarStateMeta.js';

  const dispatch = createEventDispatcher();
  const SCENE_WIDTH = 612;
  const SCENE_HEIGHT = 354;

  const MODEL_PATHS = {
    'original-standard': '/live2d-models/default/cat.model3.json',
    'keyboard-focus': '/live2d-models/default/cat.model3.json',
    'minimal-office': '/live2d-models/default/cat.model3.json',
  };

  // Left-hand keys trigger CatParamLeftHandDown
  const LEFT_HAND_KEYS = new Set([
    'KeyQ', 'KeyW', 'KeyE', 'KeyR', 'KeyT',
    'KeyA', 'KeyS', 'KeyD', 'KeyF', 'KeyG',
    'KeyZ', 'KeyX', 'KeyC', 'KeyV',
    'Digit1', 'Digit2', 'Digit3', 'Digit4', 'Digit5',
    'Digit6', 'Digit7', 'Digit8', 'Digit9', 'Digit0',
    'Backquote', 'Tab', 'CapsLock', 'ShiftLeft', 'ShiftRight',
    'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight',
    'MetaLeft', 'MetaRight', 'Space', 'Backspace', 'Delete', 'Escape',
  ]);

  export let state = {
    mode: 'idle',
    appName: 'Work Review',
    contextLabel: '待命中',
    hint: '',
    isIdle: true,
    isGeneratingReport: false,
    avatarOpacity: 0.82,
    avatarPreset: 'original-standard',
  };
  export let inputActivity = {
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
  export let transitionClass = '';
  export let motionBeat = 0;

  let canvasEl = null;
  let renderer = null;
  let modelReady = false;
  let smoothAngleX = 0;
  let smoothAngleY = 0;
  let smoothEyeX = 0;
  let smoothEyeY = 0;
  let lastFrameTime = 0;
  let animFrameId = 0;

  $: preset = getAvatarPresetDefinition(normalizeAvatarPresetId(state.avatarPreset));
  $: modelPath = MODEL_PATHS[state.avatarPreset] || MODEL_PATHS['original-standard'];
  $: modeMeta = getAvatarModeMeta(state.mode, state.contextLabel);
  $: idleMotionMeta = getAvatarIdleMotionMeta(state.mode, state.contextLabel, motionBeat);
  $: shellClass = ['avatar-shell', modeMeta.shellClass, idleMotionMeta.shellClass, transitionClass]
    .filter(Boolean)
    .join(' ');
  $: shellStyle = `--avatar-shell-opacity:${state.avatarOpacity ?? 0.82};`;

  // Key-cap overlay: reuse imported images from the preset registry
  $: keyboardVisualKey = inputActivity.keyboardVisualKey || '';
  $: keyboardActive = !!inputActivity.keyboardActive;
  $: mouseActive = !!inputActivity.mouseActive;
  $: mouseGroup = inputActivity.mouseGroup || 'idle';

  $: keyCapSrc = keyboardActive && preset.keyboardVisualLayers
    ? preset.keyboardVisualLayers[keyboardVisualKey] ?? null
    : null;

  function updateParameters() {
    if (!renderer || !modelReady) {
      animFrameId = requestAnimationFrame(updateParameters);
      return;
    }

    const now = performance.now();
    const deltaMs = lastFrameTime ? now - lastFrameTime : 16.67;
    lastFrameTime = now;

    // Exponential damping for smooth tracking
    const alpha = 1 - Math.pow(0.75, deltaMs / 16.67);

    // Head angle: map cursorRatio (0..1) to angle range (-30..30)
    const targetAngleX = ((inputActivity.cursorRatioX ?? 0.5) - 0.5) * 60;
    const targetAngleY = -((inputActivity.cursorRatioY ?? 0.5) - 0.5) * 40;
    smoothAngleX += (targetAngleX - smoothAngleX) * alpha;
    smoothAngleY += (targetAngleY - smoothAngleY) * alpha;
    renderer.setParameterValue('ParamAngleX', smoothAngleX);
    renderer.setParameterValue('ParamAngleY', smoothAngleY);
    renderer.setParameterValue('ParamAngleZ', smoothAngleX * 0.15);

    // Eye tracking
    const targetEyeX = ((inputActivity.cursorRatioX ?? 0.5) - 0.5) * 2;
    const targetEyeY = -((inputActivity.cursorRatioY ?? 0.5) - 0.5) * 2;
    smoothEyeX += (targetEyeX - smoothEyeX) * alpha * 1.4;
    smoothEyeY += (targetEyeY - smoothEyeY) * alpha * 1.4;
    renderer.setParameterValue('ParamEyeBallX', smoothEyeX);
    renderer.setParameterValue('ParamEyeBallY', smoothEyeY);

    // Hand press
    const isLeftKey = LEFT_HAND_KEYS.has(keyboardVisualKey);
    renderer.setParameterValue('CatParamLeftHandDown', keyboardActive && isLeftKey ? 1 : 0);

    const isMouseAction = mouseActive && (mouseGroup === 'mouse-left' || mouseGroup === 'mouse-right' || mouseGroup === 'mouse-side');
    const isRightKey = keyboardActive && !isLeftKey;
    renderer.setParameterValue('CatParamRightHandDown', isMouseAction || isRightKey ? 1 : 0);

    animFrameId = requestAnimationFrame(updateParameters);
  }

  let prevPresetId = state.avatarPreset;
  async function switchModel(presetId) {
    if (!renderer || presetId === prevPresetId) return;
    prevPresetId = presetId;
    modelReady = false;
    try {
      const path = MODEL_PATHS[presetId] || MODEL_PATHS['original-standard'];
      await renderer.loadModel(path);
      modelReady = true;
    } catch (err) {
      console.error('[Live2D] model load failed:', err);
    }
  }

  $: {
    if (state.avatarPreset !== prevPresetId && renderer) {
      switchModel(state.avatarPreset);
    }
  }

  onMount(async () => {
    if (!canvasEl) return;
    renderer = new Live2DRenderer();

    try {
      await renderer.init(canvasEl);
      await renderer.loadModel(modelPath);
      modelReady = true;
    } catch (err) {
      console.error('[Live2D] init failed:', err);
      dispatch('live2dfallback');
      return;
    }

    lastFrameTime = performance.now();
    animFrameId = requestAnimationFrame(updateParameters);
  });

  onDestroy(() => {
    if (animFrameId) cancelAnimationFrame(animFrameId);
    if (renderer) renderer.destroy();
    renderer = null;
    modelReady = false;
  });

  function handlePointerDown(event) {
    dispatch('avatarpointerdown', { originalEvent: event });
  }

  function handleActivate(event) {
    dispatch('avataractivate', { originalEvent: event });
  }
</script>

<div class="relative flex h-full w-full items-end overflow-visible select-none">
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class={shellClass}
    style={shellStyle}
    on:mousedown={handlePointerDown}
    on:dblclick={handleActivate}
  >
    <div class="scene-frame">
      <div class="scene-canvas-wrap">
        <canvas bind:this={canvasEl} class="live2d-canvas" />
        {#if keyCapSrc}
          <img src={keyCapSrc} class="key-cap-overlay" alt="" />
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .avatar-shell {
    display: flex;
    height: 100%;
    width: 100%;
    align-items: flex-end;
    justify-content: center;
    opacity: var(--avatar-shell-opacity);
    cursor: grab;
    transform-origin: center bottom;
    animation: avatar-float 3.1s ease-in-out infinite;
  }

  .avatar-shell:active {
    cursor: grabbing;
  }

  .scene-frame {
    position: relative;
    height: 100%;
    width: 100%;
  }

  .scene-canvas-wrap {
    position: relative;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .live2d-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .key-cap-overlay {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    pointer-events: none;
    mix-blend-mode: normal;
    transition: opacity 80ms linear;
  }

  .idle-breathe {
    animation: avatar-float 3.1s ease-in-out infinite, idle-breathe 3.6s ease-in-out infinite;
  }

  .idle-sway {
    animation: avatar-float 3.1s ease-in-out infinite, idle-sway 4.2s ease-in-out infinite;
  }

  .idle-observe {
    animation: avatar-float 3.1s ease-in-out infinite, idle-observe 4.8s ease-in-out infinite;
  }

  .idle-groove {
    animation: avatar-float 3.1s ease-in-out infinite, idle-groove 2.8s ease-in-out infinite;
  }

  .idle-focus-pulse {
    animation: avatar-float 3.1s ease-in-out infinite, idle-focus-pulse 3.1s ease-in-out infinite;
  }

  .transition-alert {
    animation: avatar-float 3.1s ease-in-out infinite, transition-alert 0.72s ease-out;
  }

  .transition-settle {
    animation: avatar-float 3.1s ease-in-out infinite, transition-settle 0.68s ease-out;
  }

  .transition-snap-back {
    animation: avatar-float 3.1s ease-in-out infinite, transition-snap-back 0.76s ease-out;
  }

  .transition-focus-shift {
    animation: avatar-float 3.1s ease-in-out infinite, transition-focus-shift 0.64s ease-out;
  }

  .transition-glide {
    animation: avatar-float 3.1s ease-in-out infinite, transition-glide 0.62s ease-out;
  }

  .transition-lift {
    animation: avatar-float 3.1s ease-in-out infinite, transition-lift 0.66s ease-out;
  }

  @keyframes avatar-float {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-2.4px); }
  }

  @keyframes idle-breathe {
    0%, 100% { transform: translateY(0) scale(1); }
    50% { transform: translateY(-1.6px) scale(1.006); }
  }

  @keyframes idle-sway {
    0%, 100% { transform: translateX(0) translateY(0); }
    50% { transform: translateX(-1.8px) translateY(-1.2px); }
  }

  @keyframes idle-observe {
    0%, 100% { transform: translateY(0); }
    30% { transform: translate(-1px, -1.4px); }
    65% { transform: translate(1.4px, -0.6px); }
  }

  @keyframes idle-groove {
    0%, 100% { transform: translateX(0) translateY(0) rotate(0deg); }
    25% { transform: translateX(-1px) translateY(-1.6px) rotate(-0.6deg); }
    75% { transform: translateX(1.2px) translateY(-1.1px) rotate(0.8deg); }
  }

  @keyframes idle-focus-pulse {
    0%, 100% { transform: translateY(0) scale(1); }
    50% { transform: translateY(-1.8px) scale(1.01); }
  }

  @keyframes transition-alert {
    0% { transform: translateY(8px) scale(0.98); opacity: 0.76; }
    60% { transform: translateY(-3px) scale(1.015); opacity: 1; }
    100% { transform: translateY(0) scale(1); opacity: 1; }
  }

  @keyframes transition-settle {
    0% { transform: translateY(-4px) scale(1.015); }
    100% { transform: translateY(0) scale(1); }
  }

  @keyframes transition-snap-back {
    0% { transform: translateX(6px) scale(0.985); }
    45% { transform: translateX(-4px) scale(1.01); }
    100% { transform: translateX(0) scale(1); }
  }

  @keyframes transition-focus-shift {
    0% { transform: translateY(4px) scale(0.992); }
    55% { transform: translateY(-3px) scale(1.012); }
    100% { transform: translateY(0) scale(1); }
  }

  @keyframes transition-glide {
    0% { transform: translateX(-5px) translateY(2px); opacity: 0.84; }
    100% { transform: translateX(0) translateY(0); opacity: 1; }
  }

  @keyframes transition-lift {
    0% { transform: translateY(8px) scale(0.985); }
    100% { transform: translateY(0) scale(1); }
  }
</style>
