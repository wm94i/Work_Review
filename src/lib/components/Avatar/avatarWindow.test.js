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
