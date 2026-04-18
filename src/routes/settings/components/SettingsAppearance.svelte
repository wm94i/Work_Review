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

  const dispatch = createEventDispatcher();
  $: currentLocale = $locale;

  let avatarSaving = false;
  let avatarScaleSaving = false;
  let avatarOpacitySaving = false;
  let avatarPresetSaving = false;
  let avatarScaleTimer = null;
  let avatarOpacityTimer = null;
  const breakReminderIntervals = [30, 45, 50, 60, 90, 120];
  let blurLabels = [];
  let avatarToggleUi;
  let runtimePlatform = '';
  let linuxSessionSupport = null;
  let permissionStatus = null;
  let gnomeExtensionInstalling = false;

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
  $: avatarInputSupportLabelKey = linuxSessionSupport?.avatarInputSupportLevel === 'full'
    ? 'settingsAppearance.avatarInputFull'
    : linuxSessionSupport?.avatarInputSupportLevel === 'mouse-only'
      ? 'settingsAppearance.avatarInputMouseOnly'
      : 'settingsAppearance.avatarInputUnavailable';
  $: avatarInputSupportHintKey = linuxSessionSupport?.avatarInputSupportLevel === 'full'
    ? 'settingsAppearance.avatarInputFullHint'
    : linuxSessionSupport?.avatarInputSupportLevel === 'mouse-only'
      ? 'settingsAppearance.avatarInputMouseOnlyHint'
      : 'settingsAppearance.avatarInputUnavailableHint';
  $: macPermissionItems = permissionStatus
    ? [
        {
          labelKey: 'settingsAppearance.avatarScreenCapturePermission',
          granted: permissionStatus.screenCapture,
        },
        {
          labelKey: 'settingsAppearance.avatarAccessibilityPermission',
          granted: permissionStatus.accessibility,
        },
        {
          labelKey: 'settingsAppearance.avatarInputMonitoringPermission',
          granted: permissionStatus.inputMonitoring,
        },
      ]
    : [];
  $: showGnomeExtensionInstaller = runtimePlatform === 'linux'
    && linuxSessionSupport?.desktopEnvironment === 'gnome'
    && !linuxSessionSupport?.gnomeAvatarExtensionEnabled;
  $: showGnomeExtensionRelogin = runtimePlatform === 'linux'
    && linuxSessionSupport?.desktopEnvironment === 'gnome'
    && linuxSessionSupport?.gnomeAvatarExtensionNeedsRelogin;

  function normalizePermissionStatus(rawStatus) {
    if (!rawStatus || typeof rawStatus !== 'object') {
      return null;
    }

    return {
      screenCapture: Boolean(rawStatus.screen_capture),
      accessibility: Boolean(rawStatus.accessibility),
      inputMonitoring: Boolean(rawStatus.input_monitoring),
      screenshotSupported: Boolean(rawStatus.screenshot_supported),
      avatarInputSupported: Boolean(rawStatus.avatar_input_supported),
      allGranted: Boolean(rawStatus.all_granted),
    };
  }

  async function refreshPlatformSupport() {
    runtimePlatform = await invoke('get_runtime_platform');
    permissionStatus = normalizePermissionStatus(await invoke('check_permissions'));

    if (runtimePlatform === 'linux') {
      linuxSessionSupport = await invoke('get_linux_session_support');
    } else {
      linuxSessionSupport = null;
    }
  }

  onMount(async () => {
    try {
      const b64 = await invoke('get_background_image');
      if (b64) bgPreview = `data:image/jpeg;base64,${b64}`;
    } catch (e) { /* ignore */ }

    try {
      await refreshPlatformSupport();
    } catch (e) {
      console.error('读取平台桌宠能力失败:', e);
      linuxSessionSupport = null;
      permissionStatus = null;
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

  function toggleBreakReminder() {
    if (!config.avatar_enabled) {
      return;
    }

    config.break_reminder_enabled = !config.break_reminder_enabled;
    dispatch('change', config);
  }

  function handleBreakReminderIntervalChange() {
    dispatch('change', config);
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
    } catch (e) {
      console.error('清除背景图失败:', e);
      showToast(t('settingsAppearance.clearFailed', { error: e }), 'error');
    }
  }

  function updateBgOpacity(val) {
    config.background_opacity = parseFloat(val);
    dispatch('change', config);
    dispatchBgEvent(bgPreview);
  }

  function updateBgBlur(val) {
    config.background_blur = parseInt(val);
    dispatch('change', config);
    dispatchBgEvent(bgPreview);
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

  async function installGnomeAvatarExtension() {
    if (gnomeExtensionInstalling) {
      return;
    }

    gnomeExtensionInstalling = true;
    try {
      const result = await invoke('install_gnome_avatar_extension');
      showToast(
        result.message,
        result.requiresRelogin ? 'warning' : result.enabled ? 'success' : 'info'
      );
      await refreshPlatformSupport();
    } catch (e) {
      console.error('自动安装 GNOME 桌宠扩展失败:', e);
      showToast(t('settingsAppearance.avatarGnomeExtensionInstallFailed', { error: e }), 'error');
    } finally {
      gnomeExtensionInstalling = false;
    }
  }
</script>

<div class="settings-card" data-locale={currentLocale}>
  <div class="settings-section">
    <div class="flex items-center justify-between gap-4">
      <div>
        <div class="flex items-center gap-2">
          <div class="settings-text">{t('settingsAppearance.avatar')}</div>
          <span class="rounded-full border border-amber-200 bg-amber-50 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.08em] text-amber-700 dark:border-amber-400/30 dark:bg-amber-500/10 dark:text-amber-200">
            Beta
          </span>
        </div>
        <div class="settings-muted mt-0.5">{t('settingsAppearance.avatarDesc')}</div>
        <div class="settings-muted mt-1 text-[12px]">{t('settingsAppearance.avatarBetaHint')}</div>
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

    {#if runtimePlatform === 'linux' && linuxSessionSupport}
      <div class="settings-block rounded-xl border border-slate-200 bg-slate-50/80 p-4 dark:border-slate-700 dark:bg-slate-900/40">
        <div class="flex items-center justify-between gap-3">
          <div>
            <div class="settings-text">{t('settingsAppearance.avatarLinuxSupportTitle')}</div>
            <div class="settings-muted mt-0.5">{t('settingsAppearance.avatarLinuxSupportHint')}</div>
          </div>
          <div class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500 dark:text-slate-400">
            {linuxSessionSupport.sessionType} / {linuxSessionSupport.desktopEnvironment}
          </div>
        </div>

        <div class="mt-3 flex items-center justify-between gap-3 text-sm">
          <div class="settings-subtle">{t('settingsAppearance.avatarScreenshotSupportTitle')}</div>
          <div class="font-semibold text-slate-700 dark:text-slate-200">
            {linuxSessionSupport.screenshotSupported
              ? t('settingsAppearance.avatarPermissionGranted')
              : t('settingsAppearance.avatarPermissionMissing')}
          </div>
        </div>

        <div class="mt-3 flex items-center justify-between gap-3 text-sm">
          <div class="settings-subtle">{t('settingsAppearance.avatarInputSupportTitle')}</div>
          <div class="font-semibold text-slate-700 dark:text-slate-200">
            {t(avatarInputSupportLabelKey)}
          </div>
        </div>

        <div class="mt-2 flex items-center justify-between gap-3 text-sm">
          <div class="settings-subtle">{t('settingsAppearance.avatarInputProviderLabel')}</div>
          <div class="font-mono text-xs text-slate-500 dark:text-slate-400">
            {linuxSessionSupport.avatarInputProvider}
          </div>
        </div>

        <div class="mt-2 text-[12px] text-slate-500 dark:text-slate-400">
          {t(avatarInputSupportHintKey)}
        </div>

        {#if linuxSessionSupport.desktopEnvironment === 'gnome'}
          <div class="mt-3 flex items-center justify-between gap-3 text-sm">
            <div class="settings-subtle">{t('settingsAppearance.avatarGnomeExtensionTitle')}</div>
            <div class="font-semibold text-slate-700 dark:text-slate-200">
              {linuxSessionSupport.gnomeAvatarExtensionNeedsRelogin
                ? t('settingsAppearance.avatarGnomeExtensionRelogin')
                : linuxSessionSupport.gnomeAvatarExtensionEnabled
                ? t('settingsAppearance.avatarGnomeExtensionReady')
                : linuxSessionSupport.gnomeAvatarExtensionInstalled
                  ? t('settingsAppearance.avatarGnomeExtensionInstalled')
                  : t('settingsAppearance.avatarGnomeExtensionMissing')}
            </div>
          </div>

          {#if showGnomeExtensionRelogin}
            <div class="mt-2 text-[12px] text-amber-700 dark:text-amber-300">
              {t('settingsAppearance.avatarGnomeExtensionReloginHint')}
            </div>
          {/if}

          {#if showGnomeExtensionInstaller}
            <div class="mt-3 flex items-center justify-between gap-3">
              <div class="text-[12px] text-slate-500 dark:text-slate-400">
                {t('settingsAppearance.avatarGnomeExtensionHint')}
              </div>
              <button
                type="button"
                on:click={installGnomeAvatarExtension}
                class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm font-medium text-slate-700 transition hover:border-slate-400 hover:bg-white disabled:cursor-wait disabled:opacity-60 dark:border-slate-600 dark:text-slate-200 dark:hover:border-slate-500 dark:hover:bg-slate-800"
                disabled={gnomeExtensionInstalling}
              >
                {gnomeExtensionInstalling
                  ? t('settingsAppearance.avatarGnomeExtensionInstalling')
                  : t('settingsAppearance.avatarGnomeExtensionInstall')}
              </button>
            </div>
          {/if}
        {/if}
      </div>
    {/if}

    {#if runtimePlatform === 'macos' && permissionStatus}
      <div class="settings-block rounded-xl border border-slate-200 bg-slate-50/80 p-4 dark:border-slate-700 dark:bg-slate-900/40">
        <div class="flex items-center justify-between gap-3">
          <div>
            <div class="settings-text">{t('settingsAppearance.avatarMacPermissionsTitle')}</div>
            <div class="settings-muted mt-0.5">{t('settingsAppearance.avatarMacPermissionsHint')}</div>
          </div>
          <div class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500 dark:text-slate-400">
            macOS
          </div>
        </div>

        <div class="mt-3 flex items-center justify-between gap-3 text-sm">
          <div class="settings-subtle">{t('settingsAppearance.avatarScreenshotSupportTitle')}</div>
          <div class="font-semibold text-slate-700 dark:text-slate-200">
            {permissionStatus.screenshotSupported
              ? t('settingsAppearance.avatarPermissionGranted')
              : t('settingsAppearance.avatarPermissionMissing')}
          </div>
        </div>

        <div class="mt-2 flex items-center justify-between gap-3 text-sm">
          <div class="settings-subtle">{t('settingsAppearance.avatarInputSupportTitle')}</div>
          <div class="font-semibold text-slate-700 dark:text-slate-200">
            {permissionStatus.avatarInputSupported
              ? t('settingsAppearance.avatarPermissionGranted')
              : t('settingsAppearance.avatarPermissionMissing')}
          </div>
        </div>

        {#each macPermissionItems as item}
          <div class="mt-2 flex items-center justify-between gap-3 text-sm">
            <div class="settings-subtle">{t(item.labelKey)}</div>
            <div class="font-semibold text-slate-700 dark:text-slate-200">
              {item.granted
                ? t('settingsAppearance.avatarPermissionGranted')
                : t('settingsAppearance.avatarPermissionMissing')}
            </div>
          </div>
        {/each}

        {#if !permissionStatus.allGranted}
          <div class="mt-2 text-[12px] text-amber-700 dark:text-amber-300">
            {t('settingsAppearance.avatarPermissionMissingHint')}
          </div>
        {/if}
      </div>
    {/if}

    {#if runtimePlatform === 'windows' && permissionStatus}
      <div class="settings-block rounded-xl border border-slate-200 bg-slate-50/80 p-4 dark:border-slate-700 dark:bg-slate-900/40">
        <div class="flex items-center justify-between gap-3">
          <div>
            <div class="settings-text">{t('settingsAppearance.avatarWindowsSupportTitle')}</div>
            <div class="settings-muted mt-0.5">{t('settingsAppearance.avatarWindowsSupportHint')}</div>
          </div>
          <div class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500 dark:text-slate-400">
            Windows
          </div>
        </div>

        <div class="mt-3 flex items-center justify-between gap-3 text-sm">
          <div class="settings-subtle">{t('settingsAppearance.avatarScreenshotSupportTitle')}</div>
          <div class="font-semibold text-slate-700 dark:text-slate-200">
            {permissionStatus.screenshotSupported
              ? t('settingsAppearance.avatarPermissionGranted')
              : t('settingsAppearance.avatarPermissionMissing')}
          </div>
        </div>

        <div class="mt-2 flex items-center justify-between gap-3 text-sm">
          <div class="settings-subtle">{t('settingsAppearance.avatarInputSupportTitle')}</div>
          <div class="font-semibold text-slate-700 dark:text-slate-200">
            {permissionStatus.avatarInputSupported
              ? t('settingsAppearance.avatarPermissionGranted')
              : t('settingsAppearance.avatarPermissionMissing')}
          </div>
        </div>
      </div>
    {/if}

    <hr class="border-slate-200 dark:border-slate-700" />

    <div class="settings-block pt-1">
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="settings-text">{t('settingsAppearance.avatarSize')}</div>
          <div class="settings-muted mt-0.5">{t('settingsAppearance.avatarSizeHint')}</div>
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
          <div class="settings-muted mt-0.5">{t('settingsAppearance.avatarOpacityHint')}</div>
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
          <div class="settings-text">{t('settingsAppearance.avatarPreset')}</div>
          <div class="settings-muted mt-0.5">{t('settingsAppearance.avatarPresetHint')}</div>
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
        <div class="settings-muted mt-0.5">{t('settingsAppearance.breakReminderDescription')}</div>
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
        <p class="settings-note">{t('settingsAppearance.breakReminderHint')}</p>
      </div>
    {/if}
  </div>
</div>

<!-- 背景图片 -->
<div class="settings-card" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsAppearance.backgroundImage')}</h3>
  <p class="settings-card-desc">{t('settingsAppearance.backgroundImageDesc')}</p>

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
