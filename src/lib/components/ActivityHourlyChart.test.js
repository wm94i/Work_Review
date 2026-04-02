import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('按小时活跃度图表应将所选时段直接显示在柱状图内', async () => {
  const source = await readFile(new URL('./ActivityHourlyChart.svelte', import.meta.url), 'utf8');

  assert.match(source, /let selectedHour = null/);
  assert.match(source, /function selectHour\(hour\)/);
  assert.match(source, /aria-pressed=\{selectedHour === bucket\.hour\}/);
  assert.match(source, /on:click=\{\(\) => selectHour\(bucket\.hour\)\}/);
  assert.match(source, /\{#if mode === 'column' && selectedHour === bucket\.hour\}/);
  assert.match(
    source,
    /\{#if mode === 'column' && selectedHour === bucket\.hour\}[\s\S]*\{formatHourRangeLabel\(bucket\.hour\)\}[\s\S]*\{formatDurationLocalized\(bucket\.duration, \{ compact: true \}\)\}[\s\S]*\{\/if\}/
  );
  assert.doesNotMatch(source, /hourlyChart\.selectedHour/);
  assert.doesNotMatch(source, /hourlyChart\.selectedHourHint/);
});

test('按小时活跃度图表文案不应继续维护独立所选时段提示', async () => {
  const source = await readFile(new URL('../i18n/index.js', import.meta.url), 'utf8');

  assert.equal((source.match(/selectedHour:/g) || []).length, 0);
  assert.equal((source.match(/selectedHourHint:/g) || []).length, 0);
});
