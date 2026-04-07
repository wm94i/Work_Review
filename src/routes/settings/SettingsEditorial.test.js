import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('设置页应接入编辑部风格壳层并强化保存操作区', async () => {
  const [settingsSource, appCssSource] = await Promise.all([
    readFile(new URL('./Settings.svelte', import.meta.url), 'utf8'),
    readFile(new URL('../../app.css', import.meta.url), 'utf8'),
  ]);

  assert.match(settingsSource, /settings-editorial-shell/);
  assert.match(settingsSource, /settings-editorial-board/);
  assert.match(settingsSource, /settings-stage-layout/);
  assert.match(settingsSource, /settings-tab-rail/);
  assert.match(settingsSource, /settings-stage-shell/);
  assert.match(settingsSource, /settings-save-dock/);
  assert.doesNotMatch(settingsSource, /settings-summary-grid/);
  assert.doesNotMatch(settingsSource, /settings-summary-toolbar/);
  assert.doesNotMatch(settingsSource, /settings-summary-manager/);
  assert.doesNotMatch(settingsSource, /settings-hero-panel/);
  assert.doesNotMatch(settingsSource, /settings-hero-kicker/);
  assert.doesNotMatch(settingsSource, /settings-hero-title/);
  assert.doesNotMatch(settingsSource, /settings-hero-note/);
  assert.doesNotMatch(settingsSource, /settings-hero-tags/);
  assert.doesNotMatch(settingsSource, /settings-hero-chip/);
  assert.match(appCssSource, /\.settings-editorial-shell/);
  assert.match(appCssSource, /\.settings-stage-layout/);
  assert.match(appCssSource, /\.settings-tab-rail/);
  assert.match(appCssSource, /\.settings-stage-shell/);
  assert.match(appCssSource, /\.settings-save-dock/);
  assert.doesNotMatch(appCssSource, /\.settings-summary-grid/);
  assert.doesNotMatch(appCssSource, /\.settings-summary-toolbar/);
  assert.doesNotMatch(appCssSource, /\.settings-summary-manager/);
  assert.doesNotMatch(appCssSource, /\.settings-hero-panel/);
  assert.doesNotMatch(appCssSource, /\.settings-hero-kicker/);
  assert.doesNotMatch(appCssSource, /\.settings-hero-title/);
  assert.doesNotMatch(appCssSource, /\.settings-hero-note/);
  assert.doesNotMatch(appCssSource, /\.settings-hero-tags/);
  assert.doesNotMatch(appCssSource, /\.settings-hero-chip/);
});

test('设置页不应继续渲染顶部摘要卡和摘要卡管理入口', async () => {
  const source = await readFile(new URL('./Settings.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /SETTINGS_SUMMARY_PREFS_KEY/);
  assert.doesNotMatch(source, /MAX_VISIBLE_SUMMARY_CARDS/);
  assert.doesNotMatch(source, /loadSummaryCardPrefs\(/);
  assert.doesNotMatch(source, /persistSummaryCardPrefs\(/);
  assert.doesNotMatch(source, /toggleSummaryCardVisibility\(/);
  assert.doesNotMatch(source, /moveSummaryCard\(/);
  assert.doesNotMatch(source, /resetSummaryCards\(/);
  assert.doesNotMatch(source, /summaryCards/);
  assert.doesNotMatch(source, /visibleSummaryCards/);
  assert.doesNotMatch(source, /settings-summary-grid/);
  assert.doesNotMatch(source, /settings-glance-card/);
  assert.doesNotMatch(source, /settings-summary-manager/);
  assert.doesNotMatch(source, /t\('settings\.summary\.manage'\)/);
});

test('设置页不应再直接格式化顶部摘要中的存储统计值', async () => {
  const source = await readFile(new URL('./Settings.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /function normalizeStorageUsageMb\(/);
  assert.doesNotMatch(source, /storageStats\.total_size_mb\.toFixed\(1\)/);
});
