import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('路由级详情弹层应使用全局对话层级，主滚动区不应创建局部堆叠上下文', async () => {
  const [appCss, timelineSource, overviewSource] = await Promise.all([
    readFile(new URL('./app.css', import.meta.url), 'utf8'),
    readFile(new URL('./routes/timeline/Timeline.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./routes/Overview.svelte', import.meta.url), 'utf8'),
  ]);

  assert.doesNotMatch(appCss, /\.app-shell-main-scroll\s*\{[^}]*z-index:\s*1;/);
  assert.match(
    timelineSource,
    /class="fixed inset-0 [^"]*z-\[140\][^"]*"/
  );
  assert.match(
    overviewSource,
    /class="fixed inset-0 [^"]*z-\[140\][^"]*"/
  );
});
