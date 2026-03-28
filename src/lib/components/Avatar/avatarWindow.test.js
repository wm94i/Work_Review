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

  assert.match(source, /桌面化身/);
  assert.match(source, />\s*Beta\s*</);
  assert.match(source, /实验功能，当前仍在持续优化显示细节、状态反馈与交互体验/);
});

test('桌宠控制项应迁移到外观页独立区域，并提供猫体透明度滑杆', () => {
  const appearanceSource = readFileSync(new URL('../../../routes/settings/components/SettingsAppearance.svelte', import.meta.url), 'utf8');
  const generalSource = readFileSync(new URL('../../../routes/settings/components/SettingsGeneral.svelte', import.meta.url), 'utf8');

  assert.match(appearanceSource, /桌宠透明度/);
  assert.match(appearanceSource, /仅作用于猫体本身，不影响背景图片透明度/);
  assert.match(appearanceSource, /avatar_opacity/);
  assert.match(appearanceSource, /min="0\.45"/);
  assert.match(appearanceSource, /max="1"/);
  assert.match(appearanceSource, /调整桌宠透明度/);
  assert.doesNotMatch(generalSource, /桌宠透明度/);
  assert.doesNotMatch(generalSource, /桌面化身/);
});

test('常规设置页应提供关闭主界面后释放 Webview 的轻量模式开关', () => {
  const source = readFileSync(new URL('../../../routes/settings/components/SettingsGeneral.svelte', import.meta.url), 'utf8');

  assert.match(source, /轻量模式/);
  assert.match(source, /关闭主界面后释放 Webview/);
  assert.match(source, /config\.lightweight_mode/);
});
