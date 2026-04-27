import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('日报生成应在网站访问部分体现域名语义分类', async () => {
  const [summarySource, localSource] = await Promise.all([
    readFile(new URL('../crates/core/src/analysis/summary.rs', import.meta.url), 'utf8'),
    readFile(new URL('../crates/core/src/analysis/local.rs', import.meta.url), 'utf8'),
  ]);

  assert.match(summarySource, /domain\.semantic_category/);
  assert.match(localSource, /domain\.semantic_category/);
  assert.match(summarySource, /translate_semantic_category_name\(semantic_category,\s*locale\)/);
  assert.match(localSource, /translate_semantic_category_name\(semantic_category,\s*locale\)/);
});

test('日报生成应体现按小时活跃度分布', async () => {
  const [summarySource, localSource, analysisModSource] = await Promise.all([
    readFile(new URL('../crates/core/src/analysis/summary.rs', import.meta.url), 'utf8'),
    readFile(new URL('../crates/core/src/analysis/local.rs', import.meta.url), 'utf8'),
    readFile(new URL('../crates/core/src/analysis/mod.rs', import.meta.url), 'utf8'),
  ]);

  assert.match(summarySource, /hourly_activity_distribution|高峰时段|按小时活跃度/);
  assert.match(localSource, /hourly_activity_distribution|高峰时段|按小时活跃度/);
  assert.match(analysisModSource, /hourly_activity_distribution|高峰时段|按小时活跃度/);
});
