import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('关于页应接入编辑部风格壳层与信息分组', async () => {
  const [aboutSource, appCssSource] = await Promise.all([
    readFile(new URL('./About.svelte', import.meta.url), 'utf8'),
    readFile(new URL('../../app.css', import.meta.url), 'utf8'),
  ]);

  assert.match(aboutSource, /about-editorial-shell/);
  assert.match(aboutSource, /about-minimal-shell/);
  assert.match(aboutSource, /about-brand-card/);
  assert.match(aboutSource, /about-action-strip/);
  assert.match(aboutSource, /about-trust-grid/);
  assert.match(aboutSource, /about-tech-stack/);
  assert.match(aboutSource, /about-tech-pill/);
  assert.doesNotMatch(aboutSource, /about-system-note/);
  assert.doesNotMatch(aboutSource, /about-system-grid/);
  assert.doesNotMatch(aboutSource, /about-linux-pills/);
  assert.match(appCssSource, /\.about-minimal-shell/);
  assert.match(appCssSource, /\.about-brand-card/);
  assert.match(appCssSource, /\.about-action-strip/);
  assert.match(appCssSource, /\.about-trust-grid/);
  assert.match(appCssSource, /\.about-tech-stack/);
  assert.match(appCssSource, /\.about-tech-pill/);
  assert.doesNotMatch(appCssSource, /\.about-system-note/);
});
