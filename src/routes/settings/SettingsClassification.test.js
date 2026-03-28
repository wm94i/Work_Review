import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('设置页不应再提供独立的应用分类页入口', async () => {
  const settingsSource = await readFile(
    new URL('./Settings.svelte', import.meta.url),
    'utf8'
  );

  assert.doesNotMatch(settingsSource, /SettingsClassification/);
  assert.doesNotMatch(settingsSource, /id:\s*'classification'/);
  assert.doesNotMatch(settingsSource, /label:\s*'应用分类'/);
});

test('分类修改应仅保留时间线入口，不再依赖设置页组件', async () => {
  const timelineSource = await readFile(
    new URL('../timeline/Timeline.svelte', import.meta.url),
    'utf8'
  );

  assert.match(timelineSource, /set_app_category_rule/);
  assert.match(timelineSource, /修改应用默认分类/);
});
