import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('时间线应渲染编辑部轨道布局与重点卡片容器', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /timeline-editorial-board[\s\S]*timeline-summary-strip/);
  assert.match(source, /timeline-editorial-shell/);
  assert.match(source, /timeline-rail/);
  assert.match(source, /timeline-entry-card-featured/);
  assert.match(source, /timeline-entry-card-compact/);
});

test('时间线应通过显式函数判断重点卡片并读取缩略图', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /function selectFeaturedActivityIds/);
  assert.match(source, /featuredActivityIds = new Set/);
  assert.match(source, /function getTimelineThumbnail/);
  assert.match(source, /getPreferredTimelineAppName/);
  assert.match(source, /shouldPreferTimelineFallbackIcon/);
});

test('时间线重点卡片应使用横向标题区与胶囊分类，避免标题和分类互相挤压', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /timeline-entry-meta-featured/);
  assert.match(source, /timeline-entry-heading-featured/);
  assert.match(source, /timeline-entry-category-pill/);
});

test('时间线紧凑卡片应显式定义信息区与标题区的排版区域，避免标题挤占应用信息宽度', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /timeline-entry-card-compact-grid/);
  assert.match(source, /timeline-entry-app-compact/);
  assert.match(source, /timeline-entry-tail-compact/);
});

test('时间线详情打开时应先显示已有缩略图，并并行请求活动详情与高清图', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /thumbnail:\s*getTimelineThumbnail\(activity\)/);
  assert.match(source, /const freshActivityPromise =/);
  assert.match(source, /const fullImagePromise =/);
  assert.match(source, /Promise\.all\(\[freshActivityPromise,\s*fullImagePromise\]\)/);
});

test('时间线首屏重点卡片图片应在列表加载阶段预热，减少第一页占位延迟', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /async function preloadTimelineLeadThumbnails/);
  assert.match(source, /await preloadTimelineLeadThumbnails\(preparedActivities\)/);
});

test('时间线实时更新收到新截图后应主动预热缩略图，避免沿用旧展示', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /listen\('screenshot-taken'/);
  assert.match(source, /if \(newActivity\?\.screenshot_path\) \{\s*loadThumbnail\(newActivity\.screenshot_path\)/);
});
