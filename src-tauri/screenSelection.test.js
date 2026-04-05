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
  assert.match(screenshotSource, /xrandr/);
  assert.match(screenshotSource, /org\.gnome\.Mutter\.DisplayConfig/);
  assert.match(screenshotSource, /crop_linux_capture_to_rect/);
  assert.match(screenshotSource, /linux_capture_target_rect/);
});

test('选屏辅助函数应在测试构建中可见，避免 Linux cargo test 因条件编译缺失而失败', async () => {
  const screenshotSource = await readFile(
    new URL('./src/screenshot.rs', import.meta.url),
    'utf8'
  );

  assert.match(
    screenshotSource,
    /#\[cfg\(any\(target_os = "macos", target_os = "windows", target_os = "linux", test\)\)\]\s*fn capture_target_point/
  );
});

test('macOS 截图应提供 screencapture 原生命令兜底，避免新系统只截到桌面层', async () => {
  const screenshotSource = await readFile(
    new URL('./src/screenshot.rs', import.meta.url),
    'utf8'
  );

  assert.match(screenshotSource, /screencapture/);
  assert.match(screenshotSource, /-R/);
});
