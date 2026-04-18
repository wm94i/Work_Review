import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('macOS 打包配置应显式注入 Info.plist 权限说明', async () => {
  const tauriConfig = JSON.parse(
    await readFile(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8')
  );
  const infoPlist = await readFile(
    new URL('../src-tauri/Info.plist', import.meta.url),
    'utf8'
  );

  assert.equal(tauriConfig.bundle.macOS.infoPlist, 'Info.plist');
  assert.match(infoPlist, /NSScreenCaptureUsageDescription/);
  assert.match(infoPlist, /NSAppleEventsUsageDescription/);
});

test('Linux deb 依赖应覆盖 gdbus 与 Wayland 截图工具', async () => {
  const tauriConfig = JSON.parse(
    await readFile(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8')
  );
  const depends = tauriConfig.bundle.linux.deb.depends;

  assert.ok(depends.includes('libglib2.0-bin'));
  assert.ok(depends.includes('gnome-screenshot | grim | scrot | maim | imagemagick'));
});

test('README 应说明 mac 输入监控权限与三端桌宠联动条件', async () => {
  const zh = await readFile(new URL('../README.md', import.meta.url), 'utf8');
  const en = await readFile(new URL('../README.en.md', import.meta.url), 'utf8');
  const tw = await readFile(new URL('../README.tw.md', import.meta.url), 'utf8');

  assert.match(zh, /输入监控/);
  assert.match(zh, /Windows/);
  assert.match(zh, /Linux/);
  assert.match(en, /Input Monitoring/);
  assert.match(en, /Windows/);
  assert.match(en, /Linux/);
  assert.match(tw, /輸入監控/);
  assert.match(tw, /Windows/);
  assert.match(tw, /Linux/);
});
