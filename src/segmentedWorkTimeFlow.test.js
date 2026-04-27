import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('统计与工作时段判断应使用有效分段时间配置', async () => {
  const commandsSource = await readFile(new URL('../src-tauri/src/commands.rs', import.meta.url), 'utf8');

  assert.match(commandsSource, /state\.config\.effective_work_segments\(\)/);
  assert.match(commandsSource, /get_daily_stats_with_segments/);
  assert.match(commandsSource, /is_work_time_in_segments/);
});

test('自动日报时间应优先依据分段工作时间配置', async () => {
  const appSource = await readFile(new URL('./App.svelte', import.meta.url), 'utf8');

  assert.match(appSource, /work_time_segments/);
  assert.match(appSource, /resolveAutoReportWorkEnd/);
});
