import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('应提供 MiniMax 作为新的 AI 提供商并同步到文档', async () => {
  const [configSource, commandSource, readmeSource, readmeEnSource] = await Promise.all([
    readFile(new URL('../crates/core/src/config.rs', import.meta.url), 'utf8'),
    readFile(new URL('../src-tauri/src/commands.rs', import.meta.url), 'utf8'),
    readFile(new URL('../README.md', import.meta.url), 'utf8'),
    readFile(new URL('../README.en.md', import.meta.url), 'utf8'),
  ]);

  assert.match(configSource, /MiniMax/);
  assert.match(configSource, /https:\/\/api\.minimaxi\.com\/v1/);
  assert.match(commandSource, /稀宇科技 MiniMax/);
  assert.match(commandSource, /MiniMax-M2\.5/);
  assert.match(readmeSource, /MiniMax/);
  assert.match(readmeEnSource, /MiniMax/);
});
