import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('编辑式强调面板应共用统一的冷色 surface token，而不是各页单独写暖色渐变', async () => {
  const [appCss, timelineSource] = await Promise.all([
    readFile(new URL('./app.css', import.meta.url), 'utf8'),
    readFile(new URL('./routes/timeline/Timeline.svelte', import.meta.url), 'utf8'),
  ]);

  assert.match(appCss, /--editorial-surface-featured:/);
  assert.match(appCss, /--editorial-surface-subtle:/);
  assert.match(appCss, /\.overview-command-deck\s*\{[\s\S]*var\(--editorial-surface-featured\)/);
  assert.match(appCss, /\.overview-panel-featured\s*\{[\s\S]*var\(--editorial-surface-featured\)/);
  assert.match(appCss, /\.overview-browser-dialog\s*\{[\s\S]*var\(--editorial-surface-featured\)/);
  assert.match(appCss, /\.about-brand-card\s*\{[\s\S]*var\(--editorial-surface-featured\)/);
  assert.match(appCss, /\.about-trust-card\s*\{[\s\S]*var\(--editorial-surface-subtle\)/);
  assert.match(timelineSource, /\.timeline-editorial-board\s*\{[\s\S]*var\(--editorial-surface-featured\)/);
  assert.match(timelineSource, /\.timeline-detail-dialog\s*\{[\s\S]*var\(--editorial-surface-featured\)/);
});
