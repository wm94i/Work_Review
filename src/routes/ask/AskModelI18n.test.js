import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('助手页模型选择器应按当前语言本地化展示 provider 名称，而不是直接显示后端中文档案名', async () => {
  const source = await readFile(new URL('./Ask.svelte', import.meta.url), 'utf8');

  assert.match(source, /function localizedProviderName\(/);
  assert.match(source, /function displayModelProfileName\(/);
  assert.match(source, /profile\.model_config\?\.provider/);
  assert.match(source, /localizedProviderName\(profile\.model_config\?\.provider\)/);
  assert.match(source, /profile\.model_config\?\.model/);
  assert.match(source, /displayModelProfileName\(profile\) \|\| t\('ask\.aiEnhanced'\)/);
  assert.doesNotMatch(source, /<option value=\{profile\.id\}>\{profile\.name \|\| t\('ask\.aiEnhanced'\)\}<\/option>/);
});
