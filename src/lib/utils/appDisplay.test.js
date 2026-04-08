import test from 'node:test';
import assert from 'node:assert/strict';
import {
  getPreferredTimelineAppName,
  shouldPreferTimelineFallbackIcon,
} from './appDisplay.js';

test('时间线应优先显示更友好的窗口标题作为应用名', () => {
  assert.equal(
    getPreferredTimelineAppName({
      appName: 'uninstall',
      windowTitle: 'Work Review Uninstall',
    }),
    'Work Review Uninstall'
  );

  assert.equal(
    getPreferredTimelineAppName({
      appName: 'xfltd',
      windowTitle: 'XFLTD',
    }),
    'XFLTD'
  );

  assert.equal(
    getPreferredTimelineAppName({
      appName: 'Work_Review.v1.0.35_x64-setup',
      windowTitle: 'Work Review Setup',
    }),
    'Work Review Setup'
  );
});

test('时间线对安装器与原始小写进程名应优先使用 fallback icon', () => {
  assert.equal(
    shouldPreferTimelineFallbackIcon({
      appName: 'uninstall',
      windowTitle: 'Work Review Uninstall',
    }),
    true
  );

  assert.equal(
    shouldPreferTimelineFallbackIcon({
      appName: 'Work_Review.v1.0.35_x64-setup',
      windowTitle: 'Work Review Setup',
    }),
    true
  );

  assert.equal(
    shouldPreferTimelineFallbackIcon({
      appName: 'xfltd',
      windowTitle: 'XFLTD',
    }),
    true
  );

  assert.equal(
    shouldPreferTimelineFallbackIcon({
      appName: 'Microsoft Edge',
      windowTitle: 'downloads-hub',
    }),
    false
  );
});
