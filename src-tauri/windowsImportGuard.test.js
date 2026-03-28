import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('Windows RECT 应从 windef 导入而不是 winuser', async () => {
  const source = await readFile(new URL('./src/monitor.rs', import.meta.url), 'utf8');

  assert.match(source, /use winapi::shared::windef::RECT;/);
  assert.doesNotMatch(source, /GetWindowThreadProcessId,\s*RECT,/);
});
