import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('应用壳层应监听录制状态变更事件并同步侧边栏状态', async () => {
  const source = await readFile(new URL('./App.svelte', import.meta.url), 'utf8');

  assert.match(source, /listen\('recording-state-changed'/);
  assert.match(source, /isRecording\s*=\s*event\.payload\.isRecording/);
  assert.match(source, /isPaused\s*=\s*event\.payload\.isPaused/);
});

test('托盘和设置的配置变更应回推到前端缓存与设置页', async () => {
  const appSource = await readFile(new URL('./App.svelte', import.meta.url), 'utf8');
  const settingsSource = await readFile(
    new URL('./routes/settings/Settings.svelte', import.meta.url),
    'utf8'
  );
  const rustSource = (
    await Promise.all([
      readFile(new URL('../src-tauri/src/commands.rs', import.meta.url), 'utf8'),
      readFile(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8'),
    ])
  ).join('\n');

  assert.match(appSource, /listen\('config-changed'/);
  assert.match(appSource, /cache\.setConfig\(event\.payload\)/);
  assert.match(settingsSource, /cache\.subscribe\(\(state\)\s*=>/);
  assert.match(settingsSource, /config\s*=\s*state\.config/);
  assert.match(rustSource, /config-changed/);
});
