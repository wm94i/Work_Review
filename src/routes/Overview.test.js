import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('概览页面的浏览器详情列表应格式化显示 URL', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(
    source,
    /formatBrowserUrlForDisplay\(url\.url\)/,
    '概览页的浏览器详情列表应对原始 URL 做可读化处理'
  );
});

test('概览页面应支持在网站访问弹层中直接修改域名语义分类并回填历史', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(source, /invoke\('set_domain_semantic_rule'/);
  assert.match(source, /translateSemanticCategoryLabel/);
  assert.match(source, /overview\.changeDomainCategoryMessage/);
  assert.match(source, /overview\.selectCategory/);
  assert.match(source, /overview\.currentCategory/);
  assert.match(source, /<select/);
  assert.match(source, /semanticCategoryOptions/);
  assert.match(source, /编码开发/);
  assert.match(source, /资料阅读/);
  assert.match(source, /休息娱乐/);
});

test('概览页面应展示按小时活跃度柱状图', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(source, /ActivityHourlyChart/);
  assert.match(source, /overview\.hourlyActivity/);
  assert.match(source, /stats\.hourly_activity_distribution/);
  assert.ok(
    source.indexOf('overview.appUsage') < source.indexOf('overview.hourlyActivity'),
    '按小时活跃度应位于应用使用模块下方'
  );
});
