import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('网站语义规则命中后应同步基础分类，保障工休统计可用', async () => {
  const commandsSource = await readFile(new URL('../src-tauri/src/commands.rs', import.meta.url), 'utf8');
  const mainSource = await readFile(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.match(commandsSource, /semantic_category_to_base_category/);
  assert.match(commandsSource, /update_activity_classification\([\s\S]*next_base_category/);
  assert.match(mainSource, /semantic_category_to_base_category/);
  assert.match(mainSource, /classification\.base_category\s*=\s*monitor::semantic_category_to_base_category/);
});
