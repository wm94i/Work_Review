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

test('桌宠窗口重新同步时应优先保持当前窗口位置，避免尺寸调整后跳回默认点位', () => {
  const source = readFileSync(new URL('../../../../src-tauri/src/avatar_engine.rs', import.meta.url), 'utf8');

  assert.match(source, /window\.outer_position\(\)/);
  assert.match(source, /current_position\.or\(saved_position\)/);
});
