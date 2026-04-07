import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('关于页应提供赞助支持按钮并展示微信与支付宝收款码', async () => {
  const source = await readFile(new URL('./About.svelte', import.meta.url), 'utf8');

  assert.match(source, /let isSponsorshipOpen = false;/);
  assert.match(source, /about\.sponsorship/);
  assert.match(source, /text-rose-500/);
  assert.match(source, /about\.wechat/);
  assert.match(source, /about\.alipay/);
  assert.match(source, /docs\/sponsorship\/vx\.png/);
  assert.match(source, /docs\/sponsorship\/zfb\.png/);
});

test('关于页赞助弹层应展示鼓励文案，检查更新按钮应单独居中', async () => {
  const source = await readFile(new URL('./About.svelte', import.meta.url), 'utf8');

  assert.match(source, /about\.supportCopy/);
  assert.match(source, /about\.supportCopy2/);
  assert.doesNotMatch(source, /推荐微信扫码支持/);
  assert.doesNotMatch(source, /也可以使用支付宝扫码/);
  assert.match(source, /if \(event\.key === 'Escape' && isSponsorshipOpen\)/);
  assert.ok(
    source.indexOf("t('about.sponsorship')")
      < source.indexOf("t('about.checkUpdates')"),
    '赞助支持按钮应位于检查更新之前的首排按钮区'
  );
  assert.ok(
    source.indexOf('<div class="flex justify-center">')
      < source.indexOf("t('about.checkUpdates')"),
    '检查更新按钮应放在单独居中的第二排'
  );
});

test('关于页应展示 Linux 会话兼容性提示，Wayland 状态可直接查看', async () => {
  const source = await readFile(new URL('./About.svelte', import.meta.url), 'utf8');

  assert.match(source, /open_data_dir/);
  assert.doesNotMatch(source, /invoke\('get_linux_session_support'\)/);
  assert.doesNotMatch(source, /about\.linuxSessionTitle/);
  assert.doesNotMatch(source, /activeWindowProvider/);
  assert.doesNotMatch(source, /browserUrlSupportLevel/);
});
