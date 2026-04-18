import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

test('桌宠窗口应强制网页根节点透明，避免轮廓外出现白底', () => {
  const source = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.match(source, /:global\(:root\)/);
  assert.match(source, /:global\(html\)/);
  assert.match(source, /:global\(body\)/);
  assert.match(source, /background:\s*transparent !important/);
});

test('设置页应提供桌宠连续缩放滑杆', () => {
  const source = readFileSync(new URL('../../../routes/settings/components/SettingsAppearance.svelte', import.meta.url), 'utf8');

  assert.match(source, /avatar_scale/);
  assert.match(source, /type="range"/);
  assert.match(source, /min="0\.7"/);
  assert.match(source, /max="1\.3"/);
  assert.match(source, /step="0\.05"/);
});

test('设置页应将桌面化身标记为 Beta 并提示其处于实验阶段', () => {
  const source = readFileSync(new URL('../../../routes/settings/components/SettingsAppearance.svelte', import.meta.url), 'utf8');

  assert.match(source, /settingsAppearance\.avatar/);
  assert.match(source, />\s*Beta\s*</);
  assert.match(source, /settingsAppearance\.avatarBetaHint/);
});

test('桌宠控制项应迁移到外观页独立区域，并提供猫体透明度滑杆', () => {
  const appearanceSource = readFileSync(new URL('../../../routes/settings/components/SettingsAppearance.svelte', import.meta.url), 'utf8');
  const generalSource = readFileSync(new URL('../../../routes/settings/components/SettingsGeneral.svelte', import.meta.url), 'utf8');

  assert.match(appearanceSource, /settingsAppearance\.avatarOpacity/);
  assert.match(appearanceSource, /settingsAppearance\.avatarOpacityHint/);
  assert.match(appearanceSource, /avatar_opacity/);
  assert.match(appearanceSource, /min="0\.45"/);
  assert.match(appearanceSource, /max="1"/);
  assert.match(appearanceSource, /settingsAppearance\.avatarOpacityAria/);
  assert.doesNotMatch(generalSource, /settingsAppearance\.avatarOpacity/);
  assert.doesNotMatch(generalSource, /settingsAppearance\.avatar/);
});

test('常规设置页应提供关闭主界面后释放 Webview 的轻量模式开关', () => {
  const source = readFileSync(new URL('../../../routes/settings/components/SettingsGeneral.svelte', import.meta.url), 'utf8');

  assert.match(source, /settingsGeneral\.lightweightMode/);
  assert.match(source, /settingsGeneral\.lightweightModeDescription/);
  assert.match(source, /config\.lightweight_mode/);
});

test('开启开机自启动后应出现主界面启动模式二级选项', () => {
  const source = readFileSync(new URL('../../../routes/settings/components/SettingsGeneral.svelte', import.meta.url), 'utf8');
  const i18nSource = readFileSync(new URL('../../../lib/i18n/index.js', import.meta.url), 'utf8');

  assert.match(source, /\{#if autoStartEnabled\}/);
  assert.match(source, /config\.auto_start_silent/);
  assert.match(source, /settingsGeneral\.autoStartLaunchMode/);
  assert.match(source, /settingsGeneral\.autoStartLaunchShow/);
  assert.match(source, /settingsGeneral\.autoStartLaunchSilent/);
  assert.match(i18nSource, /autoStartLaunchMode:\s*'启动后显示'/);
  assert.match(i18nSource, /autoStartLaunchSilent:\s*'启动时静默驻留'/);
});

test('休息提醒应放在桌宠外观设置中，并依赖桌面化身开关', () => {
  const source = readFileSync(new URL('../../../routes/settings/components/SettingsAppearance.svelte', import.meta.url), 'utf8');
  const generalSource = readFileSync(new URL('../../../routes/settings/components/SettingsGeneral.svelte', import.meta.url), 'utf8');
  const i18nSource = readFileSync(new URL('../../../lib/i18n/index.js', import.meta.url), 'utf8');

  assert.match(source, /config\.break_reminder_enabled/);
  assert.match(source, /config\.break_reminder_interval_minutes/);
  assert.match(source, /settingsAppearance\.breakReminder/);
  assert.match(source, /settingsAppearance\.breakReminderDescription/);
  assert.match(source, /settingsAppearance\.breakReminderInterval/);
  assert.match(source, /disabled=\{!config\.avatar_enabled\}/);
  assert.match(source, /settingsAppearance\.breakReminderRequiresAvatar/);
  assert.match(source, /\{#if config\.break_reminder_enabled\}/);
  assert.doesNotMatch(generalSource, /break_reminder_enabled/);
  assert.match(i18nSource, /settingsAppearance:\s*\{/);
  assert.match(i18nSource, /breakReminderInterval:\s*'提醒间隔'/);
});

test('桌宠窗口重新同步时应优先保持当前窗口位置，避免尺寸调整后跳回默认点位', () => {
  const source = readFileSync(new URL('../../../../src-tauri/src/avatar_engine.rs', import.meta.url), 'utf8');

  assert.match(source, /window\.outer_position\(\)/);
  assert.match(source, /current_position\.or\(saved_position\)/);
});

test('桌宠窗口在不可见时应暂停动作节拍，重新可见后再恢复', () => {
  const source = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.match(source, /document\.addEventListener\('visibilitychange'/);
  assert.match(
    source,
    /if\s*\(document\.hidden\)[\s\S]*clearTimeout\(motionTimer\)[\s\S]*motionTimer\s*=\s*null/
  );
  assert.match(
    source,
    /else\s*\{[\s\S]*scheduleNextMotionStep\(\)/
  );
  assert.match(source, /document\.removeEventListener\('visibilitychange'/);
});

test('桌宠窗口应监听原生输入事件，并将键鼠活跃状态喂给桌宠渲染层', () => {
  const windowSource = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');
  const engineSource = readFileSync(new URL('../../../../src-tauri/src/avatar_engine.rs', import.meta.url), 'utf8');
  const mainSource = readFileSync(new URL('../../../../src-tauri/src/main.rs', import.meta.url), 'utf8');
  const inputSource = readFileSync(new URL('../../../../src-tauri/src/avatar_input.rs', import.meta.url), 'utf8');

  assert.match(windowSource, /avatar-input-changed/);
  assert.match(windowSource, /inputActivity/);
  assert.match(windowSource, /keyboardActive/);
  assert.match(windowSource, /mouseActive/);
  assert.match(windowSource, /keyboardGroup/);
  assert.match(windowSource, /cursorRatioX/);
  assert.match(windowSource, /cursorRatioY/);
  assert.match(windowSource, /lastKeyboardInputAtMs/);
  assert.match(windowSource, /lastMouseInputAtMs/);
  assert.match(windowSource, /<AvatarCanvas[\s\S]*\{inputActivity\}/);
  assert.match(engineSource, /AVATAR_INPUT_EVENT/);
  assert.match(engineSource, /AvatarInputPayload/);
  assert.match(engineSource, /keyboard_group/);
  assert.match(engineSource, /emit_avatar_input/);
  assert.match(mainSource, /mod avatar_input/);
  assert.match(mainSource, /start_avatar_input_monitor/);
  assert.match(mainSource, /spawn_avatar_input_bridge/);
  assert.match(inputSource, /cfg\(target_os = "windows"\)/);
  assert.match(inputSource, /SetWindowsHookExW/);
  assert.match(inputSource, /WH_KEYBOARD_LL/);
  assert.match(inputSource, /WH_MOUSE_LL/);
  assert.match(inputSource, /GetCursorPos/);
});

test('桌宠键盘模式应按原版 BongoCat 键区分组选择不同高亮层', () => {
  const source = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');
  const inputSource = readFileSync(new URL('../../../../src-tauri/src/avatar_input.rs', import.meta.url), 'utf8');

  assert.match(registrySource, /standard-keyboard-7\.png/);
  assert.match(registrySource, /standard-keyboard-8\.png/);
  assert.match(registrySource, /standard-keyboard-9\.png/);
  assert.match(registrySource, /standard-keyboard-10\.png/);
  assert.match(registrySource, /standard-keyboard-11\.png/);
  assert.match(registrySource, /standard-keyboard-12\.png/);
  assert.match(registrySource, /standard-keyboard-13\.png/);
  assert.match(registrySource, /standard-keyboard-14\.png/);
  assert.match(registrySource, /standard-keyboard-1\.png/);
  assert.match(registrySource, /standard-keyboard-2\.png/);
  assert.match(registrySource, /standard-keyboard-3\.png/);
  assert.match(registrySource, /standard-keyboard-4\.png/);
  assert.match(registrySource, /standard-keyboard-5\.png/);
  assert.match(registrySource, /standard-keyboard-6\.png/);
  assert.match(registrySource, /STANDARD_KEYBOARD_FRAME_BY_GROUP/);
  assert.match(source, /inputActivity\.keyboardGroup/);
  assert.match(source, /keyOverlaySrc = keyboardActive/);
  assert.match(source, /preset\.keyboardFrames\[keyboardGroup\]/);
  assert.match(inputSource, /standard_keyboard_group_from_key_code/);
  assert.match(inputSource, /digit-1/);
  assert.match(inputSource, /digit-7/);
  assert.match(inputSource, /key-q/);
  assert.match(inputSource, /key-e/);
  assert.match(inputSource, /key-r/);
  assert.match(inputSource, /space/);
  assert.match(inputSource, /key-a/);
  assert.match(inputSource, /key-d/);
  assert.match(inputSource, /key-s/);
  assert.match(inputSource, /key-w/);
});

test('桌宠键盘模式应导入原版左右手多帧资源并按键区切换手部帧', () => {
  const source = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.match(registrySource, /standard-hand-4\.png/);
  assert.match(registrySource, /standard-hand-5\.png/);
  assert.match(registrySource, /standard-hand-6\.png/);
  assert.match(registrySource, /standard-hand-7\.png/);
  assert.match(registrySource, /standard-hand-8\.png/);
  assert.match(registrySource, /standard-hand-9\.png/);
  assert.match(registrySource, /standard-hand-10\.png/);
  assert.match(registrySource, /standard-hand-11\.png/);
  assert.match(registrySource, /standard-hand-12\.png/);
  assert.match(registrySource, /standard-hand-13\.png/);
  assert.match(registrySource, /standard-hand-14\.png/);
  assert.match(registrySource, /standard-hand-0\.png/);
  assert.match(registrySource, /standard-hand-1\.png/);
  assert.match(registrySource, /standard-hand-2\.png/);
  assert.match(registrySource, /standard-hand-3\.png/);
  assert.match(registrySource, /standard-up\.png/);
  assert.match(registrySource, /STANDARD_HAND_FRAME_BY_GROUP/);
  assert.match(source, /standardHandSrc = keyboardActive/);
  assert.match(source, /standard-hand-layer/);
});

test('桌宠鼠标模式应按上游标准模式生成动态左手，并叠加鼠标设备状态层', () => {
  const source = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');
  const inputSource = readFileSync(new URL('../../../../src-tauri/src/avatar_input.rs', import.meta.url), 'utf8');

  assert.match(registrySource, /mouse-bg\.png/);
  assert.match(registrySource, /mouse\.png/);
  assert.match(registrySource, /mouse_left\.png/);
  assert.match(registrySource, /mouse_right\.png/);
  assert.match(registrySource, /mouse_side\.png/);
  assert.match(source, /computeStandardMouseGeometry/);
  assert.match(source, /mouseArmPoints/);
  assert.match(source, /mouseDeviceX/);
  assert.match(source, /mouseDeviceY/);
  assert.match(source, /mouse-arm-fill/);
  assert.match(source, /mouse-arm-shadow/);
  assert.match(source, /mouse-arm-stroke/);
  assert.match(registrySource, /MOUSE_DEVICE_OVERLAY_BY_GROUP/);
  assert.match(source, /mouseGroup/);
  assert.match(source, /cursorRatioX/);
  assert.match(source, /cursorRatioY/);
  assert.match(inputSource, /mouse_group_from_event_type/);
  assert.match(inputSource, /record_cursor_ratio/);
  assert.match(inputSource, /current_cursor_ratio/);
  assert.match(inputSource, /mouse-left/);
  assert.match(inputSource, /mouse-right/);
  assert.match(inputSource, /mouse-side/);
  assert.match(inputSource, /mouse-move/);
});

test('Linux 外观设置页应展示桌宠联动能力，并区分完整联动与仅鼠标联动', () => {
  const appearanceSource = readFileSync(new URL('../../../routes/settings/components/SettingsAppearance.svelte', import.meta.url), 'utf8');
  const i18nSource = readFileSync(new URL('../../../lib/i18n/index.js', import.meta.url), 'utf8');
  const commandsSource = readFileSync(new URL('../../../../src-tauri/src/commands.rs', import.meta.url), 'utf8');
  const inputSource = readFileSync(new URL('../../../../src-tauri/src/avatar_input.rs', import.meta.url), 'utf8');

  assert.match(appearanceSource, /get_linux_session_support/);
  assert.match(appearanceSource, /avatarLinuxSupportTitle/);
  assert.match(appearanceSource, /avatarInputSupportTitle/);
  assert.match(appearanceSource, /avatarInputProviderLabel/);
  assert.match(appearanceSource, /avatarInputMouseOnly/);
  assert.match(appearanceSource, /avatarInputUnavailable/);
  assert.match(i18nSource, /avatarLinuxSupportTitle/);
  assert.match(i18nSource, /avatarInputMouseOnly/);
  assert.match(i18nSource, /avatarInputUnavailable/);
  assert.match(commandsSource, /avatar_input_support_level/);
  assert.match(commandsSource, /avatar_mouse_supported/);
  assert.match(commandsSource, /avatar_keyboard_supported/);
  assert.match(commandsSource, /gnome_avatar_extension_installed/);
  assert.match(commandsSource, /gnome_avatar_extension_enabled/);
  assert.match(commandsSource, /gnome_avatar_extension_needs_relogin/);
  assert.match(commandsSource, /install_gnome_avatar_extension/);
  assert.match(commandsSource, /requires_relogin/);
  assert.match(inputSource, /gnome-shell-dbus/);
  assert.match(inputSource, /kdotool-mouselocation/);
  assert.match(inputSource, /hyprctl-cursorpos/);
  assert.match(inputSource, /WorkReviewAvatarInput\.GetInput/);
  assert.match(appearanceSource, /installGnomeAvatarExtension/);
  assert.match(appearanceSource, /avatarGnomeExtensionInstall/);
  assert.match(appearanceSource, /avatarGnomeExtensionInstalling/);
  assert.match(appearanceSource, /avatarGnomeExtensionReady/);
  assert.match(appearanceSource, /avatarGnomeExtensionRelogin/);
  assert.match(appearanceSource, /avatarGnomeExtensionReloginHint/);
  assert.match(i18nSource, /avatarGnomeExtensionInstall/);
  assert.match(i18nSource, /avatarGnomeExtensionInstalling/);
  assert.match(i18nSource, /avatarGnomeExtensionReady/);
  assert.match(i18nSource, /avatarGnomeExtensionRelogin/);
  assert.match(i18nSource, /avatarGnomeExtensionReloginHint/);
});

test('外观设置页应同时展示三端桌宠能力与 mac 权限状态', () => {
  const appearanceSource = readFileSync(new URL('../../../routes/settings/components/SettingsAppearance.svelte', import.meta.url), 'utf8');
  const i18nSource = readFileSync(new URL('../../../lib/i18n/index.js', import.meta.url), 'utf8');
  const commandsSource = readFileSync(new URL('../../../../src-tauri/src/commands.rs', import.meta.url), 'utf8');
  const screenshotSource = readFileSync(new URL('../../../../src-tauri/src/screenshot.rs', import.meta.url), 'utf8');

  assert.match(appearanceSource, /invoke\('check_permissions'\)/);
  assert.match(appearanceSource, /runtimePlatform === 'macos'/);
  assert.match(appearanceSource, /runtimePlatform === 'windows'/);
  assert.match(appearanceSource, /avatarMacPermissionsTitle/);
  assert.match(appearanceSource, /avatarWindowsSupportTitle/);
  assert.match(appearanceSource, /avatarScreenshotSupportTitle/);
  assert.match(appearanceSource, /avatarPermissionGranted/);
  assert.match(appearanceSource, /avatarPermissionMissing/);
  assert.match(appearanceSource, /inputMonitoring/);
  assert.match(commandsSource, /"input_monitoring"/);
  assert.match(screenshotSource, /CGPreflightListenEventAccess/);
  assert.match(screenshotSource, /CGRequestListenEventAccess/);
  assert.match(i18nSource, /avatarMacPermissionsTitle/);
  assert.match(i18nSource, /avatarWindowsSupportTitle/);
  assert.match(i18nSource, /avatarScreenshotSupportTitle/);
  assert.match(i18nSource, /avatarInputMonitoringPermission/);
  assert.match(i18nSource, /avatarPermissionMissingHint/);
});

test('桌宠输入桥应以更高刷新率推送输入状态，减少鼠标联动卡顿', () => {
  const inputSource = readFileSync(new URL('../../../../src-tauri/src/avatar_input.rs', import.meta.url), 'utf8');

  assert.match(inputSource, /Duration::from_millis\(16\)/);
});

test('桌宠窗口应拦截右键菜单和打印快捷键，避免弹出无法消除的原生界面', () => {
  const source = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.match(source, /document\.addEventListener\('contextmenu'/);
  assert.match(source, /event\.preventDefault\(\)/);
  assert.match(source, /document\.addEventListener\('keydown'/);
  assert.match(source, /event\.key\s*===\s*'p'/);
  assert.match(source, /event\.metaKey\s*\|\|\s*event\.ctrlKey/);
  assert.match(source, /document\.removeEventListener\('contextmenu'/);
  assert.match(source, /document\.removeEventListener\('keydown'/);
});

test('桌宠窗口应只保留 SVG 桌宠实现，不再引用 Live2D 分支', () => {
  const source = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /AvatarLive2D/);
  assert.doesNotMatch(source, /live2dLoadFailed/);
  assert.doesNotMatch(source, /avatarType/);
});

test('外观设置页不应再提供桌宠风格切换', () => {
  const source = readFileSync(new URL('../../../routes/settings/components/SettingsAppearance.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /handleAvatarTypeChange/);
  assert.doesNotMatch(source, /settingsAppearance\.avatarType/);
  assert.doesNotMatch(source, />\s*SVG Cat\s*</);
  assert.doesNotMatch(source, />\s*Live2D\s*</);
});

test('桌宠配置应支持官方预设字段，并在设置页仅展示当前可用的原版标准卡片', () => {
  const configSource = readFileSync(new URL('../../../../src-tauri/src/config.rs', import.meta.url), 'utf8');
  const commandsSource = readFileSync(new URL('../../../../src-tauri/src/commands.rs', import.meta.url), 'utf8');
  const appearanceSource = readFileSync(new URL('../../../routes/settings/components/SettingsAppearance.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');
  const i18nSource = readFileSync(new URL('../../../lib/i18n/index.js', import.meta.url), 'utf8');

  assert.match(configSource, /pub avatar_preset: String/);
  assert.match(configSource, /default_avatar_preset/);
  assert.match(commandsSource, /previous_avatar_preset/);
  assert.match(commandsSource, /config\.avatar_preset/);
  assert.match(appearanceSource, /config\.avatar_preset/);
  assert.match(appearanceSource, /AVATAR_PRESET_OPTIONS/);
  assert.match(registrySource, /original-standard/);
  assert.match(registrySource, /AVAILABLE_AVATAR_PRESET_IDS/);
  assert.match(registrySource, /new Set/);
  assert.match(i18nSource, /avatarPreset:/);
  assert.match(i18nSource, /avatarPresetOriginalTitle/);
});

test('桌宠渲染层应通过官方预设注册表解析资源，而不是继续硬编码单套资源', () => {
  const canvasSource = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const windowSource = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.match(canvasSource, /getAvatarPresetDefinition/);
  assert.match(canvasSource, /normalizeAvatarPresetId/);
  assert.match(canvasSource, /\$:\s*preset = getAvatarPresetDefinition/);
  assert.match(canvasSource, /preset\.sceneSrc/);
  assert.match(canvasSource, /preset\.showKeyboardOverlay/);
  assert.match(canvasSource, /preset\.showMouseDevice/);
  assert.match(canvasSource, /preset\.showMouseArm/);
  assert.match(windowSource, /avatarPreset/);
  assert.match(registrySource, /showKeyboardOverlay/);
  assert.match(registrySource, /showMouseDevice/);
  assert.match(registrySource, /showMouseArm/);
});

test('官方预设卡片应使用真实桌宠预览，并把已下线预设回退到原版标准', () => {
  const appearanceSource = readFileSync(new URL('../../../routes/settings/components/SettingsAppearance.svelte', import.meta.url), 'utf8');
  const previewSource = readFileSync(new URL('./AvatarPresetPreview.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.match(appearanceSource, /AvatarPresetPreview/);
  assert.doesNotMatch(appearanceSource, /<img[\s\S]*preset\.preview/);
  assert.match(previewSource, /AvatarCanvas/);
  assert.match(previewSource, /pointer-events-none/);
  assert.match(previewSource, /getAvatarPresetOption/);
  assert.match(registrySource, /original-standard[\s\S]*keyboardActive:\s*true[\s\S]*mouseActive:\s*true/);
  assert.match(registrySource, /AVAILABLE_AVATAR_PRESET_IDS\.has\(presetId\)/);
  assert.match(registrySource, /AVATAR_PRESET_DEFAULT/);
});

test('键盘专注与极简办公应保持各自场景图，只补充键盘和鼠标联动', () => {
  const canvasSource = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.match(canvasSource, /contentTransform/);
  assert.match(canvasSource, /staticCoverSrc/);
  assert.match(canvasSource, /keyboardOverlayStyle/);
  assert.match(canvasSource, /mouseDeviceStyle/);
  assert.match(canvasSource, /mouseArmStyle/);
  assert.match(registrySource, /model-standard\/background\.png/);
  assert.match(registrySource, /model-standard\/cover\.png/);
  assert.match(registrySource, /keyboard-focus[\s\S]*showKeyboardOverlay:\s*false/);
  assert.match(registrySource, /keyboard-focus[\s\S]*showMouseDevice:\s*false/);
  assert.match(registrySource, /keyboard-focus[\s\S]*showMouseArm:\s*true/);
  assert.match(registrySource, /model-gamepad\/background\.png/);
  assert.match(registrySource, /model-gamepad\/cover\.png/);
  assert.match(registrySource, /minimal-office[\s\S]*showKeyboardOverlay:\s*false/);
  assert.match(registrySource, /minimal-office[\s\S]*showMouseDevice:\s*false/);
  assert.match(registrySource, /minimal-office[\s\S]*showMouseArm:\s*true/);
});

test('三套官方预设应保持明显不同的视觉场景，而不是都退回标准底图', () => {
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.match(registrySource, /original-standard[\s\S]*sceneSrc:\s*mouseBg/);
  assert.match(registrySource, /keyboard-focus[\s\S]*sceneSrc:\s*modelStandardBackground/);
  assert.match(registrySource, /minimal-office[\s\S]*sceneSrc:\s*modelGamepadBackground/);
  assert.match(registrySource, /keyboard-focus[\s\S]*staticCoverSrc:\s*modelStandardCover/);
  assert.match(registrySource, /minimal-office[\s\S]*staticCoverSrc:\s*modelGamepadCover/);
});

test('键盘专注与极简办公应在各自场景上消费静态图层和热点映射来联动键盘与鼠标', () => {
  const canvasSource = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.match(canvasSource, /sceneInteractionLayout/);
  assert.match(canvasSource, /keyboardHotspots/);
  assert.match(canvasSource, /mouseHotspots/);
  assert.match(canvasSource, /keyboardVisualSrc/);
  assert.match(registrySource, /keyboard-focus[\s\S]*keyboardVisualLayers:\s*STANDARD_MODEL_LEFT_KEYS/);
  assert.match(registrySource, /keyboard-focus[\s\S]*interactionLayout:\s*STANDARD_MODEL_INTERACTION/);
  assert.match(registrySource, /minimal-office[\s\S]*keyboardVisualLayers:\s*GAMEPAD_KEYBOARD_LAYER_BY_KEY/);
  assert.match(registrySource, /minimal-office[\s\S]*mouseVisualLayers:\s*GAMEPAD_MOUSE_LAYER_BY_GROUP/);
  assert.match(registrySource, /minimal-office[\s\S]*interactionLayout:\s*GAMEPAD_MODEL_INTERACTION/);
});

test('静态场景预设的鼠标联动不应再用 multiply 叠加产生阴影块，鼠标移动高亮应保持可见', () => {
  const canvasSource = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.match(canvasSource, /scene-interaction-layer/);
  assert.doesNotMatch(canvasSource, /mix-blend-mode:\s*multiply/);
  assert.match(registrySource, /trackpad:\s*polygonHotspot\('44,126 214,101 242,204 73,229',\s*HOTSPOT_CYAN_FILL,\s*HOTSPOT_CYAN_STROKE\)/);
  assert.match(registrySource, /STANDARD_MODEL_INTERACTION[\s\S]*'mouse-move':\s*\[polygonHotspot\('44,126 214,101 242,204 73,229',\s*HOTSPOT_CYAN_FILL,\s*HOTSPOT_CYAN_STROKE\)\]/);
  assert.match(registrySource, /STANDARD_MODEL_INTERACTION[\s\S]*'mouse-left':\s*\[polygonHotspot\('44,126 124,114 157,217 73,229',\s*HOTSPOT_CYAN_FILL,\s*HOTSPOT_CYAN_STROKE\)\]/);
  assert.match(registrySource, /STANDARD_MODEL_INTERACTION[\s\S]*'mouse-right':\s*\[polygonHotspot\('124,114 214,101 242,204 157,217',\s*HOTSPOT_CYAN_FILL,\s*HOTSPOT_CYAN_STROKE\)\]/);
  assert.match(registrySource, /GAMEPAD_MODEL_INTERACTION[\s\S]*'mouse-move':\s*\[polygonHotspot\('333,189 359,202 332,216 306,203',\s*HOTSPOT_CYAN_FILL,\s*HOTSPOT_CYAN_STROKE\)\]/);
});

test('静态封面预设不应再整体抖动，而应只保留键盘层和鼠标层反馈', () => {
  const canvasSource = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');

  assert.match(canvasSource, /getStaticCoverTransform/);
  assert.match(canvasSource, /staticCoverTransform/);
  assert.match(canvasSource, /static-cover-layer/);
  assert.match(canvasSource, /return '';/);
  assert.doesNotMatch(canvasSource, /translate\(-4\.2 2\.8\) rotate\(-1\.5 300 162\)/);
  assert.doesNotMatch(canvasSource, /translate\(1\.2 2\.1\) rotate\(0\.8 304 160\)/);
});

test('深色桌面上的提醒气泡应使用固定浅色面板，避免文字和底色一起发黑', () => {
  const popoverSource = readFileSync(new URL('./AvatarPopover.svelte', import.meta.url), 'utf8');

  assert.match(popoverSource, /background:\s*rgba\(255,\s*255,\s*255,\s*0\.96\)/);
  assert.match(popoverSource, /color:\s*rgb\(15,\s*23,\s*42\)/);
  assert.match(popoverSource, /border-color:\s*rgba\(226,\s*232,\s*240,\s*0\.96\)/);
  assert.match(popoverSource, /backdrop-filter:\s*blur\(14px\)\s*saturate\(1\.06\)/);
});

test('静态场景预设应继续通过精确键名驱动源图层映射，保证各自视觉里的键盘鼠标联动', () => {
  const canvasSource = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const windowSource = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');
  const inputSource = readFileSync(new URL('../../../../src-tauri/src/avatar_input.rs', import.meta.url), 'utf8');
  const engineSource = readFileSync(new URL('../../../../src-tauri/src/avatar_engine.rs', import.meta.url), 'utf8');

  assert.match(engineSource, /keyboard_visual_key/);
  assert.match(windowSource, /keyboardVisualKey/);
  assert.match(canvasSource, /keyboardVisualKey/);
  assert.match(canvasSource, /preset\.keyboardVisualLayers\?\.\[keyboardVisualKey\]/);
  assert.match(canvasSource, /preset\.mouseVisualLayers\?\.\[mouseGroup\]/);
  assert.match(
    registrySource,
    /import\.meta\.glob\('\.\/assets\/bongocat\/model-standard\/left-keys\/\*\.png'/
  );
  assert.match(
    registrySource,
    /import\.meta\.glob\('\.\/assets\/bongocat\/model-gamepad\/left-keys\/\*\.png'/
  );
  assert.match(
    registrySource,
    /import\.meta\.glob\('\.\/assets\/bongocat\/model-gamepad\/right-keys\/\*\.png'/
  );
  assert.match(inputSource, /fn keyboard_visual_key_from_key_code/);
  assert.match(inputSource, /"KeyA"/);
  assert.match(inputSource, /"KeyN"/);
  assert.match(inputSource, /"KeyO"/);
  assert.match(inputSource, /"KeyP"/);
  assert.match(inputSource, /"Num9"/);
  assert.match(inputSource, /"Space"/);
  assert.match(inputSource, /"Return"/);
  assert.match(inputSource, /"ShiftLeft"/);
  assert.match(inputSource, /"ControlRight"/);
});

test('静态场景预设应默认先绘制热点层，再绘制 cover，最后再叠加精确键位图层；仅键盘专注允许触控板热点提升到 cover 上方', () => {
  const canvasSource = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');

  assert.match(
    canvasSource,
    /\{#if preCoverKeyboardHotspots.length \|\| preCoverMouseHotspots.length\}[\s\S]*scene-interaction-layer[\s\S]*\{\/if\}[\s\S]*\{#if staticCoverSrc\}[\s\S]*class="static-cover-layer"/
  );
  assert.match(
    canvasSource,
    /\{#if staticCoverSrc\}[\s\S]*class="static-cover-layer"[\s\S]*\{\/if\}[\s\S]*\{#if showKeyboardLayers && keyboardVisualSrc\}[\s\S]*keyboard-visual-layer/
  );
  assert.match(
    canvasSource,
    /\{#if staticCoverSrc\}[\s\S]*class="static-cover-layer"[\s\S]*\{\/if\}[\s\S]*\{#if showMouseLayers && mouseVisualSrc\}[\s\S]*mouse-visual-layer/
  );
  assert.match(
    canvasSource,
    /\{#if staticCoverSrc\}[\s\S]*class="static-cover-layer"[\s\S]*\{\/if\}[\s\S]*\{#if postCoverMouseHotspots.length\}[\s\S]*post-cover-mouse-hotspot-layer/
  );
});

test('静态场景预设应把精确联动图层裁剪到设备区域，并允许键盘专注在 cover 上方单独显示触控板热点', () => {
  const canvasSource = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.match(canvasSource, /keyboardVisualClip/);
  assert.match(canvasSource, /mouseVisualClip/);
  assert.match(canvasSource, /mouseHotspotsAboveCover/);
  assert.match(canvasSource, /clipPath/);
  assert.match(canvasSource, /clip-path=\{keyboardVisualClipUrl \|\| undefined\}/);
  assert.match(canvasSource, /clip-path=\{mouseVisualClipUrl \|\| undefined\}/);
  assert.match(canvasSource, /post-cover-mouse-hotspot-layer/);
  assert.match(registrySource, /keyboard-focus[\s\S]*keyboardVisualClip:/);
  assert.match(registrySource, /keyboard-focus[\s\S]*keyboardHotspotsAboveCover:\s*true/);
  assert.match(registrySource, /minimal-office[\s\S]*keyboardVisualClip:/);
  assert.match(registrySource, /minimal-office[\s\S]*mouseVisualClip:/);
});

test('键盘专注与极简办公应根据鼠标坐标渲染独立指示器，避免静态场景里鼠标动作丢失', () => {
  const canvasSource = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.match(canvasSource, /computeStaticSceneMouseGeometry/);
  assert.match(canvasSource, /staticSceneMouseGeometry =/);
  assert.match(canvasSource, /preset\.mouseMotionModel/);
  assert.match(canvasSource, /class="scene-mouse-paw"/);
  assert.match(canvasSource, /cursorRatioX/);
  assert.match(canvasSource, /cursorRatioY/);
  assert.match(registrySource, /keyboard-focus[\s\S]*mouseMotionModel:/);
  assert.match(registrySource, /minimal-office[\s\S]*mouseMotionModel:/);
});

test('静态预设应把键盘高亮提升到封面上方，并移除整块鼠标移动蓝色蒙层', () => {
  const canvasSource = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.match(canvasSource, /keyboardHotspotsAboveCover/);
  assert.match(canvasSource, /postCoverKeyboardHotspots/);
  assert.match(canvasSource, /post-cover-keyboard-hotspot-layer/);
  assert.match(canvasSource, /!mouseVisualSrc && !preset\.mouseMotionModel/);
  assert.match(registrySource, /keyboard-focus[\s\S]*keyboardHotspotsAboveCover:\s*true/);
  assert.match(registrySource, /minimal-office[\s\S]*keyboardHotspotsAboveCover:\s*true/);
});
