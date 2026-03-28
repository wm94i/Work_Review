import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('多屏幕截图应按活动窗口所在屏幕选屏', async () => {
  const screenshotSource = await readFile(
    new URL('./src/screenshot.rs', import.meta.url),
    'utf8'
  );
  const monitorSource = await readFile(
    new URL('./src/monitor.rs', import.meta.url),
    'utf8'
  );

  assert.match(screenshotSource, /Screen::from_point/);
  assert.match(screenshotSource, /capture_for_window/);
  assert.match(monitorSource, /window_bounds/);
  assert.match(screenshotSource, /MonitorFromPoint/);
  assert.match(screenshotSource, /from_raw_hmonitor/);
  assert.match(screenshotSource, /GetMonitorInfoW/);
  assert.match(screenshotSource, /capture_target_monitor_rect/);
});
