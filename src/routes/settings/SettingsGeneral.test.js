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

  assert.match(source, /#\[cfg\(windows\)\]\s*const AUTOSTART_LAUNCH_ARG/);
  assert.match(source, /开机自启动服务未初始化/);
  assert.match(source, /try_state::<AutoLaunchConfig>/);
  assert.match(source, /tauri_plugin_autostart::ManagerExt/);
  assert.doesNotMatch(source, /tauri::State<'_,\s*AutoLaunch(Manager|Config)>/);
});

test('切换开机自启后应立即持久化 config，避免用户忘点保存导致下次开机行为失同步', async () => {
  const source = await readFile(
    new URL('./components/SettingsGeneral.svelte', import.meta.url),
    'utf8'
  );

  const toggleMatch = source.match(/async function toggleAutoStart\([^)]*\)\s*\{[\s\S]*?\n\s{2}\}/);
  assert.ok(toggleMatch, '未找到 toggleAutoStart 函数');
  assert.match(toggleMatch[0], /invoke\('save_config'\s*,\s*\{\s*config\s*\}\)/);
});

test('开启自启时应把 silent 选择传给后端，注册表参数才能反映用户的显/隐模式', async () => {
  const source = await readFile(
    new URL('./components/SettingsGeneral.svelte', import.meta.url),
    'utf8'
  );

  const toggleMatch = source.match(/async function toggleAutoStart\([^)]*\)\s*\{[\s\S]*?\n\s{2}\}/);
  assert.ok(toggleMatch, '未找到 toggleAutoStart 函数');
  assert.match(
    toggleMatch[0],
    /invoke\('enable_autostart'\s*,\s*\{\s*silent:\s*[^}]*auto_start_silent[^}]*\}\)/
  );
});

test('切换"静默驻留"模式后应立即持久化 config，否则开机决策仍会用旧值', async () => {
  const source = await readFile(
    new URL('./components/SettingsGeneral.svelte', import.meta.url),
    'utf8'
  );

  const updateMatch = source.match(
    /async function updateAutoStartLaunchMode\([^)]*\)\s*\{[\s\S]*?\n\s{2}\}/
  );
  assert.ok(updateMatch, '未找到 async 版本的 updateAutoStartLaunchMode 函数');
  assert.match(updateMatch[0], /config\.auto_start_silent\s*=/);
  assert.match(updateMatch[0], /invoke\('save_config'\s*,\s*\{\s*config\s*\}\)/);
});

test('切换静默模式时若自启已启用应重注册，让注册表参数与新 silent 值对齐', async () => {
  const source = await readFile(
    new URL('./components/SettingsGeneral.svelte', import.meta.url),
    'utf8'
  );

  const updateMatch = source.match(
    /async function updateAutoStartLaunchMode\([^)]*\)\s*\{[\s\S]*?\n\s{2}\}/
  );
  assert.ok(updateMatch, '未找到 async 版本的 updateAutoStartLaunchMode 函数');
  assert.match(updateMatch[0], /if\s*\(\s*autoStartEnabled\s*\)/);
  assert.match(
    updateMatch[0],
    /invoke\('enable_autostart'\s*,\s*\{\s*silent:\s*silentMode\s*\}\)/
  );
});

test('onMount 同步注册表与 config 不一致时也应落盘，避免下次启动再次失同步', async () => {
  const source = await readFile(
    new URL('./components/SettingsGeneral.svelte', import.meta.url),
    'utf8'
  );

  const onMountMatch = source.match(/onMount\(async \(\)\s*=>\s*\{[\s\S]*?\n\s{2}\}\);/);
  assert.ok(onMountMatch, '未找到 onMount 块');
  assert.match(onMountMatch[0], /config\.auto_start\s*!==\s*autoStartEnabled/);
  assert.match(onMountMatch[0], /invoke\('save_config'\s*,\s*\{\s*config\s*\}\)/);
});
