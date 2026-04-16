import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

test('前端依赖与入口不应再保留 Live2D 运行时', () => {
  const packageSource = readFileSync(new URL('./package.json', import.meta.url), 'utf8');
  const indexSource = readFileSync(new URL('./index.html', import.meta.url), 'utf8');
  const viteSource = readFileSync(new URL('./vite.config.js', import.meta.url), 'utf8');

  assert.doesNotMatch(packageSource, /easy-live2d/);
  assert.doesNotMatch(packageSource, /pixi\.js/);
  assert.doesNotMatch(indexSource, /live2dcubismcore\.min\.js/);
  assert.doesNotMatch(indexSource, /live2d\.min\.js/);
  assert.doesNotMatch(viteSource, /live2dcubismcore\.min\.js/);
  assert.doesNotMatch(viteSource, /live2d\.min\.js/);
});

test('Rust 侧不应再保留 Live2D 配置和输入监听入口', () => {
  const cargoSource = readFileSync(new URL('./src-tauri/Cargo.toml', import.meta.url), 'utf8');
  const configSource = readFileSync(new URL('./src-tauri/src/config.rs', import.meta.url), 'utf8');
  const mainSource = readFileSync(new URL('./src-tauri/src/main.rs', import.meta.url), 'utf8');
  const tauriConfigSource = readFileSync(new URL('./src-tauri/tauri.conf.json', import.meta.url), 'utf8');

  assert.doesNotMatch(cargoSource, /rdev/);
  assert.doesNotMatch(cargoSource, /block =/);
  assert.doesNotMatch(configSource, /avatar_type/);
  assert.doesNotMatch(configSource, /avatar_live2d_model/);
  assert.doesNotMatch(mainSource, /device_listener/);
  assert.doesNotMatch(mainSource, /start_device_listening/);
  assert.doesNotMatch(tauriConfigSource, /live2d-models/);
});
