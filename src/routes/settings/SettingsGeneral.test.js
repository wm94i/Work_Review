import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('工作时间跨零点时应显示跨天后的总时长而不是横线', async () => {
  const source = await readFile(
    new URL('./components/SettingsGeneral.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /endTotal === startTotal/);
  assert.match(source, /endTotal < startTotal/);
  assert.match(source, /24 \* 60/);
  assert.doesNotMatch(source, /const diffSeconds = \(endTotal - startTotal\) \* 60;/);
});

test('开始时间等于结束时间时应显示零时长而不是横线', async () => {
  const source = await readFile(
    new URL('./components/SettingsGeneral.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /endTotal === startTotal/);
  assert.match(source, /formatDurationLocalized\(0\)/);
});

test('关闭开机自启动失败时不应吞掉所有异常并伪称已经移除成功', async () => {
  const source = await readFile(
    new URL('./components/SettingsGeneral.svelte', import.meta.url),
    'utf8'
  );

  assert.doesNotMatch(source, /未知原因报错os error -2147024891/);
  assert.doesNotMatch(source, /确实已经正常移除了开机自启动/);
});

test('自启动后端应避免强制依赖 Windows 自定义状态并为未初始化场景返回明确错误', async () => {
  const source = await readFile(
    new URL('../../../src-tauri/src/autostart.rs', import.meta.url),
    'utf8'
  );

  assert.match(source, /开机自启动服务未初始化/);
  assert.match(source, /try_state::<AutoLaunchManager>/);
  assert.match(source, /tauri_plugin_autostart::ManagerExt/);
  assert.doesNotMatch(source, /tauri::State<'_, AutoLaunchManager>/);
});
