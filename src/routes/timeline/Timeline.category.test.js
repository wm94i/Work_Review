import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('时间线详情应支持修改应用默认分类并二次确认后回填历史', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /import\s+\{\s*confirm\s*\}\s+from\s+'\.\.\/\.\.\/lib\/stores\/confirm\.js'/);
  assert.match(source, /invoke\('set_app_category_rule'/);
  assert.match(source, /将\$\{activity\.app_name\}的默认分类改为/);
  assert.match(source, /同步更新该应用的历史记录/);
});
