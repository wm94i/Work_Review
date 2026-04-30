<script>
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { showToast } from '$lib/stores/toast.js';
  import { locale, t } from '$lib/i18n/index.js';
  import {
    AVATAR_OPACITY_DEFAULT,
    AVATAR_SCALE_DEFAULT,
    clampAvatarOpacity,
    clampAvatarScale,
    formatAvatarOpacityLabel,
    formatAvatarScaleLabel,
    getAvatarToggleToast,
    getAvatarToggleUiState,
    toggleAvatarSetting,
    updateAvatarOpacitySetting,
    updateAvatarScaleSetting,
  } from '$lib/utils/avatarToggle.js';
  import { AVATAR_PRESET_OPTIONS } from '$lib/components/Avatar/avatarPresetRegistry.js';
  import AvatarPresetPreview from '$lib/components/Avatar/AvatarPresetPreview.svelte';

  export let config;
  export let mode = 'full';

  const dispatch = createEventDispatcher();
  $: currentLocale = $locale;
  $: showAvatarControls = mode === 'full' || mode === 'avatar-only';
  $: showBackgroundSettings = mode === 'full' || mode === 'background-only';

  let avatarSaving = false;
  let avatarScaleSaving = false;
  let avatarOpacitySaving = false;
  let avatarPersonaSaving = false;
  let avatarPresetSaving = false;
  let avatarScaleTimer = null;
  let avatarOpacityTimer = null;
  const breakReminderIntervals = [30, 45, 50, 60, 90, 120];
  const AVATAR_PERSONA_OPTIONS = [
    {
      id: 'companion',
      titleKey: 'settingsAppearance.avatarPersonaCompanionTitle',
      descriptionKey: 'settingsAppearance.avatarPersonaCompanionDesc',
    },
    {
      id: 'assistant',
      titleKey: 'settingsAppearance.avatarPersonaAssistantTitle',
      descriptionKey: 'settingsAppearance.avatarPersonaAssistantDesc',
    },
    {
      id: 'coach',
      titleKey: 'settingsAppearance.avatarPersonaCoachTitle',
      descriptionKey: 'settingsAppearance.avatarPersonaCoachDesc',
    },
  ];
  let blurLabels = [];
  let avatarToggleUi;
  // === 背景图片 ===
  let bgPreview = null;
  let bgUploading = false;
  let appearanceDestroyed = false;

  $: {
    currentLocale;
    blurLabels = [
      t('settingsAppearance.blurClear'),
      t('settingsAppearance.blurLight'),
      t('settingsAppearance.blurMedium'),
    ];
  }
  $: {
    currentLocale;
    avatarToggleUi = getAvatarToggleUiState(Boolean(config.avatar_enabled), avatarSaving);
  }
  $: avatarScale = clampAvatarScale(config.avatar_scale ?? AVATAR_SCALE_DEFAULT);
  $: avatarScaleLabel = formatAvatarScaleLabel(avatarScale);
  $: avatarOpacity = clampAvatarOpacity(config.avatar_opacity ?? AVATAR_OPACITY_DEFAULT);
  $: avatarOpacityLabel = formatAvatarOpacityLabel(avatarOpacity);
  onMount(async () => {
    if (showBackgroundSettings) {
      try {
        const b64 = await invoke('get_background_image');
        if (b64) bgPreview = `data:image/jpeg;base64,${b64}`;
      } catch (e) { /* ignore */ }
    }
  });

  onDestroy(() => {
    appearanceDestroyed = true;
    clearTimeout(avatarScaleTimer);
    clearTimeout(avatarOpacityTimer);
  });

  async function toggleAvatarMode() {
    if (avatarSaving) {
      return;
    }

    avatarSaving = true;

    try {
      if (config.avatar_enabled) {
        try {
          await invoke('persist_avatar_position');
        } catch (persistError) {
          console.warn('关闭桌面助手前持久化位置失败:', persistError);
        }
      }

      const enabled = await toggleAvatarSetting(config, async (nextConfig) => {
        await invoke('save_config', { config: nextConfig });
      });

      dispatch('change', config);
      showToast(getAvatarToggleToast(enabled), enabled ? 'success' : 'info');
    } catch (e) {
      console.error('设置桌宠失败:', e);
      showToast(t('settingsAppearance.avatarToggleFailed', { error: e }), 'error');
    } finally {
      avatarSaving = false;
    }
  }

  function queueAvatarScaleSave(nextScale) {
    clearTimeout(avatarScaleTimer);
    avatarScaleTimer = setTimeout(async () => {
      avatarScaleSaving = true;

      try {
        const savedScale = await updateAvatarScaleSetting(config, nextScale, async (nextConfig) => {
          await invoke('save_config', { config: nextConfig });
        });
        config.avatar_scale = savedScale;
        dispatch('change', config);
      } catch (e) {
        console.error('保存桌宠缩放失败:', e);
        showToast(t('settingsAppearance.avatarScaleSaveFailed', { error: e }), 'error');
      } finally {
        avatarScaleSaving = false;
      }
    }, 120);
  }

  function handleAvatarScaleInput(event) {
    const nextScale = clampAvatarScale(Number(event.currentTarget.value));
    config.avatar_scale = nextScale;
    dispatch('change', config);
    queueAvatarScaleSave(nextScale);
  }

  function queueAvatarOpacitySave(nextOpacity) {
    clearTimeout(avatarOpacityTimer);
    avatarOpacityTimer = setTimeout(async () => {
      avatarOpacitySaving = true;

      try {
        const savedOpacity = await updateAvatarOpacitySetting(
          config,
          nextOpacity,
          async (nextConfig) => {
            await invoke('save_config', { config: nextConfig });
          }
        );
        config.avatar_opacity = savedOpacity;
        dispatch('change', config);
      } catch (e) {
        console.error('保存桌宠透明度失败:', e);
        showToast(t('settingsAppearance.avatarOpacitySaveFailed', { error: e }), 'error');
      } finally {
        avatarOpacitySaving = false;
      }
    }, 120);
  }

  function handleAvatarOpacityInput(event) {
    const nextOpacity = clampAvatarOpacity(Number(event.currentTarget.value));
    config.avatar_opacity = nextOpacity;
    dispatch('change', config);
    queueAvatarOpacitySave(nextOpacity);
  }

  async function selectAvatarPreset(presetId) {
    if (avatarPresetSaving || config.avatar_preset === presetId) {
      return;
    }

    avatarPresetSaving = true;
    const previousPreset = config.avatar_preset;
    config.avatar_preset = presetId;
    dispatch('change', config);

    try {
      await invoke('save_config', { config });
    } catch (e) {
      config.avatar_preset = previousPreset;
      dispatch('change', config);
      console.error('保存桌宠预设失败:', e);
      showToast(t('settingsAppearance.avatarPresetSaveFailed', { error: e }), 'error');
    } finally {
      avatarPresetSaving = false;
    }
  }

  async function selectAvatarPersona(personaId) {
    if (avatarPersonaSaving || config.avatar_persona === personaId) {
      return;
    }

    avatarPersonaSaving = true;
    const previousPersona = config.avatar_persona;
    config.avatar_persona = personaId;
    dispatch('change', config);

    try {
      await invoke('save_config', { config });
    } catch (e) {
      config.avatar_persona = previousPersona;
      dispatch('change', config);
      console.error('保存桌宠互动风格失败:', e);
      showToast(t('settingsAppearance.avatarPersonaSaveFailed', { error: e }), 'error');
    } finally {
      avatarPersonaSaving = false;
    }
  }

  function toggleBreakReminder() {
    if (!config.avatar_enabled) {
      return;
    }

    config.break_reminder_enabled = !config.break_reminder_enabled;
    dispatch('change', config);
    saveConfigQuietly();
  }

  function handleBreakReminderIntervalChange() {
    dispatch('change', config);
    saveConfigQuietly();
  }

  function handleBgFileSelect(event) {
    const file = event.target.files?.[0];
    if (!file) return;
    if (!file.type.startsWith('image/')) return;
    if (file.size > 10 * 1024 * 1024) {
      showToast(t('settingsAppearance.imageTooLarge'), 'warning');
      return;
    }

    bgUploading = true;
    const reader = new FileReader();
    reader.onload = async () => {
      if (appearanceDestroyed) return;

      try {
        const b64Data = typeof reader.result === 'string' ? reader.result.split(',')[1] : null;
        if (!b64Data) {
          throw new Error(t('settingsAppearance.imageReadFailed'));
        }
        await invoke('save_background_image', { data: b64Data });
        if (appearanceDestroyed) return;
        config.background_image = 'background.jpg';
        await invoke('save_config', { config });
        const freshB64 = await invoke('get_background_image');
        if (appearanceDestroyed) return;
        const imageUrl = freshB64 ? `data:image/jpeg;base64,${freshB64}` : null;
        bgPreview = imageUrl;
        dispatchBgEvent(imageUrl);
      } catch (e) {
        if (appearanceDestroyed) return;
        console.error('上传背景图失败:', e);
        showToast(t('settingsAppearance.uploadFailed', { error: e }), 'error');
      } finally {
        if (!appearanceDestroyed) {
          bgUploading = false;
        }
      }
    };
    reader.readAsDataURL(file);
  }

  async function clearBg() {
    try {
      await invoke('clear_background_image');
      bgPreview = null;
      config.background_image = null;
      dispatchBgEvent(null);
      await invoke('save_config', { config });
    } catch (e) {
      console.error('清除背景图失败:', e);
      showToast(t('settingsAppearance.clearFailed', { error: e }), 'error');
    }
  }

  function updateBgOpacity(val) {
    config.background_opacity = parseFloat(val);
    dispatch('change', config);
    dispatchBgEvent(bgPreview);
    saveConfigQuietly();
  }

  function updateBgBlur(val) {
    config.background_blur = parseInt(val);
    dispatch('change', config);
    dispatchBgEvent(bgPreview);
    saveConfigQuietly();
  }

  function dispatchBgEvent(image) {
    window.dispatchEvent(new CustomEvent('background-changed', {
      detail: {
        image,
        opacity: config.background_opacity ?? 0.25,
        blur: config.background_blur ?? 1,
      }
    }));
  }

  async function saveConfigQuietly() {
    try {
      await invoke('save_config', { config });
    } catch (e) {
      console.error('自动保存配置失败:', e);
    }
  }
</script>

{#if showAvatarControls}
<div class="settings-card" data-locale={currentLocale}>
  <div class="settings-section">
    <div class="flex items-center justify-between gap-4">
      <div>
        <div class="flex items-center gap-2">
          <div class="settings-text">{t('settingsAppearance.avatar')}</div>
        </div>
        <div class="settings-muted mt-0.5">{t('settingsAppearance.avatarDesc')}</div>
      </div>
      <button
        type="button"
        on:click={toggleAvatarMode}
        class="switch-track {avatarToggleUi.trackClass} {avatarToggleUi.buttonClass}"
        disabled={avatarSaving}
        aria-label={avatarToggleUi.ariaLabel}
        aria-pressed={config.avatar_enabled}
      >
        <span class="switch-thumb {avatarToggleUi.thumbClass}"></span>
      </button>
    </div>
    <div class="settings-block pt-1">
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="settings-text">{t('settingsAppearance.avatarSize')}</div>
        </div>
        <div class="text-sm font-semibold text-slate-700 dark:text-slate-200">
          {avatarScaleLabel}
          {#if avatarScaleSaving}
            <span class="ml-2 text-xs font-normal text-slate-400 dark:text-slate-500">{t('settingsAppearance.syncing')}</span>
          {/if}
        </div>
      </div>

      <input
        type="range"
        min="0.7"
        max="1.3"
        step="0.05"
        value={avatarScale}
        on:input={handleAvatarScaleInput}
        class="mt-3 w-full accent-primary-500"
        aria-label={t('settingsAppearance.avatarSizeAria')}
      />
      <div class="mt-2 flex justify-between text-[11px] text-slate-400 dark:text-slate-500">
        <span>{t('settingsAppearance.smaller')}</span>
        <span>{t('settingsAppearance.default90')}</span>
        <span>{t('settingsAppearance.larger')}</span>
      </div>
    </div>

    <div class="settings-block pt-1">
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="settings-text">{t('settingsAppearance.avatarOpacity')}</div>
        </div>
        <div class="text-sm font-semibold text-slate-700 dark:text-slate-200">
          {avatarOpacityLabel}
          {#if avatarOpacitySaving}
            <span class="ml-2 text-xs font-normal text-slate-400 dark:text-slate-500">{t('settingsAppearance.syncing')}</span>
          {/if}
        </div>
      </div>

      <input
        type="range"
        min="0.45"
        max="1"
        step="0.05"
        value={avatarOpacity}
        on:input={handleAvatarOpacityInput}
        class="mt-3 w-full accent-primary-500"
        aria-label={t('settingsAppearance.avatarOpacityAria')}
      />
      <div class="mt-2 flex justify-between text-[11px] text-slate-400 dark:text-slate-500">
        <span>{t('settingsAppearance.moreTransparent')}</span>
        <span>{t('settingsAppearance.default82')}</span>
        <span>{t('settingsAppearance.moreSolid')}</span>
      </div>
    </div>

    <div class="settings-block pt-1">
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="settings-text">{t('settingsAppearance.avatarPersona')}</div>
        </div>
        {#if avatarPersonaSaving}
          <div class="text-xs text-slate-400 dark:text-slate-500">{t('settingsAppearance.syncing')}</div>
        {/if}
      </div>

      <div class="mt-3 grid gap-3 md:grid-cols-3">
        {#each AVATAR_PERSONA_OPTIONS as persona}
          <button
            type="button"
            class="rounded-lg border p-3 text-left transition {config.avatar_persona === persona.id ? 'border-emerald-400 bg-emerald-50/80 shadow-sm dark:border-emerald-400/70 dark:bg-emerald-500/10' : 'border-slate-200 bg-white hover:border-slate-300 dark:border-slate-700 dark:bg-slate-900/60 dark:hover:border-slate-600'}"
            on:click={() => selectAvatarPersona(persona.id)}
            aria-pressed={config.avatar_persona === persona.id}
          >
            <div class="flex items-center justify-between gap-3">
              <div class="text-sm font-semibold text-slate-800 dark:text-slate-100">
                {t(persona.titleKey)}
              </div>
              {#if config.avatar_persona === persona.id}
                <span class="rounded-full bg-emerald-500/12 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.08em] text-emerald-700 dark:text-emerald-300">
                  {t('settingsAppearance.avatarPersonaCurrent')}
                </span>
              {/if}
            </div>
            <div class="mt-2 text-xs leading-5 text-slate-500 dark:text-slate-400">
              {t(persona.descriptionKey)}
            </div>
          </button>
        {/each}
      </div>
    </div>

    <div class="settings-block pt-1">
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="settings-text">{t('settingsAppearance.avatarPreset')}</div>
        </div>
        {#if avatarPresetSaving}
          <div class="text-xs text-slate-400 dark:text-slate-500">{t('settingsAppearance.syncing')}</div>
        {/if}
      </div>

      <div class="mt-3 grid gap-3 md:grid-cols-3">
        {#each AVATAR_PRESET_OPTIONS as preset}
          <button
            type="button"
            class="rounded-2xl border p-3 text-left transition {config.avatar_preset === preset.id ? 'border-primary-500 bg-primary-50/70 shadow-sm dark:border-primary-400 dark:bg-primary-500/10' : 'border-slate-200 bg-white hover:border-slate-300 dark:border-slate-700 dark:bg-slate-900/60 dark:hover:border-slate-600'}"
            on:click={() => selectAvatarPreset(preset.id)}
            aria-pressed={config.avatar_preset === preset.id}
          >
            <div class="h-24 w-full rounded-xl border border-slate-200 bg-slate-50 dark:border-slate-700 dark:bg-slate-950/70">
              <AvatarPresetPreview presetId={preset.id} selected={config.avatar_preset === preset.id} />
            </div>
            <div class="mt-3 text-sm font-semibold text-slate-800 dark:text-slate-100">
              {t(preset.titleKey)}
            </div>
            <div class="mt-1 text-xs leading-5 text-slate-500 dark:text-slate-400">
              {t(preset.descriptionKey)}
            </div>
          </button>
        {/each}
      </div>
    </div>

    <hr class="border-slate-200 dark:border-slate-700" />

    <div class="flex items-center justify-between gap-4">
      <div>
        <div class="settings-text">{t('settingsAppearance.breakReminder')}</div>
        {#if !config.avatar_enabled}
          <div class="settings-muted mt-1 text-[12px]">{t('settingsAppearance.breakReminderRequiresAvatar')}</div>
        {/if}
      </div>
      <button
        type="button"
        on:click={toggleBreakReminder}
        class="switch-track {config.break_reminder_enabled && config.avatar_enabled ? 'bg-primary-500' : 'bg-slate-300 dark:bg-slate-600'} {!config.avatar_enabled ? 'cursor-not-allowed opacity-50' : ''}"
        disabled={!config.avatar_enabled}
        aria-pressed={config.break_reminder_enabled}
      >
        <span class="switch-thumb {config.break_reminder_enabled && config.avatar_enabled ? 'translate-x-5' : 'translate-x-0'}"></span>
      </button>
    </div>

    {#if config.break_reminder_enabled}
      <div class="settings-block pt-3 border-t border-slate-200 dark:border-slate-700">
        <label for="break-reminder-interval" class="settings-label mb-1.5">
          {t('settingsAppearance.breakReminderInterval')}
        </label>
        <select
          id="break-reminder-interval"
          bind:value={config.break_reminder_interval_minutes}
          on:change={handleBreakReminderIntervalChange}
          class="control-input"
          disabled={!config.avatar_enabled}
        >
          {#each breakReminderIntervals as interval}
            <option value={interval}>{interval} 分钟</option>
          {/each}
        </select>
      </div>
    {/if}
  </div>
</div>
{/if}

<!-- 背景图片 -->
{#if showBackgroundSettings}
<div class="settings-card" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsAppearance.backgroundImage')}</h3>

  <div class="settings-section">
    <!-- 预览 + 上传 -->
    <div class="flex items-start gap-4">
      {#if bgPreview}
        <div class="w-32 h-20 rounded-lg overflow-hidden border border-slate-200 dark:border-slate-700 flex-shrink-0">
          <img src={bgPreview} alt={t('settingsAppearance.bgPreviewAlt')} class="w-full h-full object-cover" />
        </div>
      {:else}
        <div class="w-32 h-20 rounded-lg border-2 border-dashed border-slate-200 dark:border-slate-700 flex items-center justify-center flex-shrink-0">
          <span class="settings-subtle">{t('settingsAppearance.noBackground')}</span>
        </div>
      {/if}

      <div class="flex-1 settings-field">
        <label class="settings-action-secondary cursor-pointer">
          {#if bgUploading}
            <div class="animate-spin rounded-full h-3 w-3 border-2 border-slate-500 border-t-transparent"></div>
            {t('common.processing')}
          {:else}
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" /></svg>
            {t('settingsAppearance.chooseImage')}
          {/if}
          <input type="file" accept="image/*" class="hidden" on:change={handleBgFileSelect} disabled={bgUploading} />
        </label>
        {#if bgPreview}
          <button
            on:click={clearBg}
            class="settings-link-danger"
          >
            {t('settingsAppearance.clearBackground')}
          </button>
        {/if}
        <p class="settings-muted">{t('settingsAppearance.bgSupport')}</p>
      </div>
    </div>

    {#if bgPreview || config.background_image}
      <hr class="border-slate-200 dark:border-slate-700" />

      <!-- 显示强度 -->
      <div class="settings-block">
        <div class="flex items-center justify-between">
          <span class="settings-text">{t('settingsAppearance.bgStrength')}</span>
          <span class="settings-value">{Math.round((config.background_opacity ?? 0.25) * 100)}%</span>
        </div>
        <input
          type="range"
          min="0.05"
          max="0.60"
          step="0.01"
          value={config.background_opacity ?? 0.25}
          on:input={(e) => updateBgOpacity(e.target.value)}
          class="range-input"
        />
        <div class="flex justify-between text-[10px] settings-subtle">
          <span>{t('settingsAppearance.bgLight')}</span>
          <span>{t('settingsAppearance.bgStrong')}</span>
        </div>
      </div>

      <!-- 模糊度 -->
      <div class="settings-block">
        <div class="flex items-center justify-between">
          <span class="settings-text">{t('settingsAppearance.bgBlur')}</span>
          <span class="settings-muted">{blurLabels[config.background_blur ?? 1]}</span>
        </div>
        <div class="flex gap-2">
          {#each [0, 1, 2] as level}
            <button
              on:click={() => updateBgBlur(level)}
              class="segment-btn
                {(config.background_blur ?? 1) === level
                  ? 'settings-segment-active'
                  : 'settings-segment-base'}"
            >
              {blurLabels[level]}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>
{/if}
