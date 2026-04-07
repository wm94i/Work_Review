import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('主窗口应默认隐藏并由启动流程决定是否显示', async () => {
  const tauriConfig = JSON.parse(
    await readFile(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8')
  );
  const mainSource = await readFile(
    new URL('../src-tauri/src/main.rs', import.meta.url),
    'utf8'
  );

  assert.equal(tauriConfig.app.windows[0].visible, false);
  assert.match(
    mainSource,
    /if should_hide_main_window \{\s*let _ = window\.hide\(\);\s*\} else \{\s*let _ = window\.show\(\);\s*\}/
  );
});
