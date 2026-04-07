import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('应用壳层应使用统一 i18n 资源并支持三种界面语言', async () => {
  const [appSource, i18nSource] = await Promise.all([
    readFile(new URL('./App.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./lib/i18n/index.js', import.meta.url), 'utf8'),
  ]);

  assert.match(i18nSource, /'zh-CN'/);
  assert.match(i18nSource, /'en'/);
  assert.match(i18nSource, /'zh-TW'/);
  assert.match(i18nSource, /export function cycleLocale\(\)/);
  assert.match(appSource, /applyLocaleToDocument/);
  assert.match(appSource, /initializeLocale/);
  assert.match(appSource, /\{#key currentLocale\}/);
  assert.match(appSource, /<Router \{routes\} \/>/);
});

test('侧边栏底部应提供语言切换菜单，并按 ZH、TW、EN 顺序展示缩写标识', async () => {
  const source = await readFile(new URL('./lib/components/Sidebar.svelte', import.meta.url), 'utf8');

  assert.match(source, /locale-switch/);
  assert.match(source, /aria-haspopup="menu"/);
  assert.match(source, /toggleLocaleMenu/);
  assert.match(source, /selectLocale\(option\.value\)/);
  assert.match(source, /value: 'zh-CN', label: 'ZH'/);
  assert.match(source, /value: 'zh-TW', label: 'TW'/);
  assert.match(source, /value: 'en', label: 'EN'/);
  assert.ok(
    source.indexOf("value: 'zh-CN', label: 'ZH'")
      < source.indexOf("value: 'zh-TW', label: 'TW'")
      && source.indexOf("value: 'zh-TW', label: 'TW'")
      < source.indexOf("value: 'en', label: 'EN'"),
    '语言顺序应为 ZH、TW、EN'
  );
  assert.match(source, /emitTo\('avatar', 'locale-changed', normalizedLocale\)/);
  assert.doesNotMatch(source, /sidebar-footer-version/);
});

test('语言菜单应左对齐展开，并按“英文缩写 + 语言名称”展示避免裁切', async () => {
  const source = await readFile(new URL('./lib/components/Sidebar.svelte', import.meta.url), 'utf8');

  assert.match(source, /class="absolute bottom-full left-0 mb-2/);
  assert.match(source, /min-w-\[148px\]/);
  assert.match(source, /whitespace-nowrap/);
  assert.match(source, /fullLabelKey: 'sidebar\.localeNames\.zhCN'/);
  assert.match(source, /fullLabel: translate\(option\.fullLabelKey\)/);
  assert.match(
    source,
    /<span class="font-semibold tracking-\[0\.08em\] text-slate-500 dark:text-slate-400">\{option\.label\}<\/span>\s*<span class="text-slate-700 dark:text-slate-200">\{option\.fullLabel\}<\/span>/s
  );
});

test('桌宠窗口应初始化 locale 并监听语言切换事件', async () => {
  const source = await readFile(new URL('./routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.match(source, /initializeLocale\(\)/);
  assert.match(source, /applyLocaleToDocument/);
  assert.match(source, /listen\('locale-changed'/);
  assert.match(source, /getAvatarStateBubble\(nextState\.mode, currentLocale\)/);
});
