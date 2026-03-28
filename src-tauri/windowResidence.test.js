import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('主窗口显示逻辑应尽量跟随当前活跃空间而不是停留在应用原空间', async () => {
  const mainSource = await readFile(new URL('./src/main.rs', import.meta.url), 'utf8');
  const commandSource = await readFile(new URL('./src/commands.rs', import.meta.url), 'utf8');
  const avatarSource = await readFile(new URL('../src/routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.match(mainSource, /MoveToActiveSpace|setCollectionBehavior_/);
  assert.match(mainSource, /source_window_label/);
  assert.match(commandSource, /show_main_window/);
  assert.match(avatarSource, /invoke\('show_main_window', \{ sourceWindowLabel: appWindow\.label \}\)/);
});
