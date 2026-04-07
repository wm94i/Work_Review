import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('按小时活跃度图表的首尾刻度应贴边对齐，避免 00:00 和 23:00 溢出', async () => {
  const source = await readFile(new URL('./ActivityHourlyChart.svelte', import.meta.url), 'utf8');

  assert.match(source, /function hourAxisLabelAlignmentClass\(hour\)/);
  assert.match(source, /hour === 0/);
  assert.match(source, /hour === 23/);
  assert.match(source, /justify-start/);
  assert.match(source, /justify-end/);
  assert.match(
    source,
    /class=\{`flex w-full \$\{hourAxisLabelAlignmentClass\(bucket\.hour\)\}`\}/
  );
});
