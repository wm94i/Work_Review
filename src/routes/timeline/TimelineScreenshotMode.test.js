import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('时间线详情在无截图记录时应显示明确占位，而不是截图加载失败', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /if \(!screenshotPath\) \{\s*return null;\s*\}/);
  assert.match(source, /selectedActivity\.screenshot_path/);
  assert.match(source, /timeline\.detail\.screenshotMissing/);
});
