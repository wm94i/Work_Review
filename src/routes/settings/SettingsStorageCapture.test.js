import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('存储设置应提供独立的启用截图开关，而不只是截图间隔', async () => {
  const source = await readFile(
    new URL('./components/SettingsStorage.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /settingsStorage\.screenshotsEnabled/);
  assert.match(source, /config\.storage\.screenshots_enabled/);
  assert.match(source, /settingsStorage\.screenshotsEnabledHint/);
});
