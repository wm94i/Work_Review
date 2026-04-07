import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('应用壳层应使用统一外壳类名承载背景、侧边栏与窗口控制区', async () => {
  const [appSource, appCssSource] = await Promise.all([
    readFile(new URL('./App.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./app.css', import.meta.url), 'utf8'),
  ]);

  assert.match(appSource, /app-shell/);
  assert.match(appSource, /app-shell-sidebar/);
  assert.match(appSource, /app-shell-main/);
  assert.match(appSource, /app-shell-windowbar/);
  assert.match(appSource, /app-shell-window-btn/);

  assert.match(appCssSource, /\.app-shell\b/);
  assert.match(appCssSource, /\.app-shell-sidebar\b/);
  assert.match(appCssSource, /\.app-shell-main\b/);
  assert.match(appCssSource, /\.app-shell-windowbar\b/);
  assert.match(appCssSource, /\.app-shell-window-btn\b/);
});

test('主导航字号应高于设置内导航，形成稳定层级', async () => {
  const appCssSource = await readFile(new URL('./app.css', import.meta.url), 'utf8');

  assert.match(appCssSource, /\.sidebar-nav-label\s*\{[\s\S]*font-size:\s*0\.98rem;/);
  assert.match(appCssSource, /\.settings-tab-rail-item\s*\{[\s\S]*font-size:\s*0\.92rem;/);
});
