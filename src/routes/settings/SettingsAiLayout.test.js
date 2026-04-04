import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('AI 设置中的 API 密钥输入应支持显示与隐藏切换', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /let showApiKey = false;/);
  assert.match(source, /\{#if showApiKey\}/);
  assert.match(source, /type="text"/);
  assert.match(source, /type="password"/);
  assert.match(source, /settingsAI\.hideApiKey/);
  assert.match(source, /settingsAI\.showApiKey/);
});

test('日报导出目录应从 AI 设置移到存储设置', async () => {
  const aiSource = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );
  const storageSource = await readFile(
    new URL('./components/SettingsStorage.svelte', import.meta.url),
    'utf8'
  );

  assert.doesNotMatch(aiSource, /日报 Markdown 导出目录/);
  assert.match(storageSource, /settingsStorage\.exportDir/);
  assert.match(storageSource, /pickDailyReportExportDir/);
  assert.match(storageSource, /settingsStorage\.exportDirHint/);
  assert.match(storageSource, /settingsStorage\.exportDesc/);
  assert.match(storageSource, /settingsStorage\.screenshotCardTitle/);
  assert.match(storageSource, /settingsStorage\.exportTitle/);
  assert.match(storageSource, /settingsStorage\.dataDirTitle/);
  assert.match(storageSource, /settingsStorage\.modeAll/);
  assert.match(storageSource, /settingsStorage\.modeAllDesc/);
});

test('支持模型发现的提供商应共用模型列表拉取与手动输入回退', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /invoke\('get_provider_models'/);
  assert.match(source, /refreshProviderModels/);
  assert.match(source, /providerModels/);
  assert.match(source, /supports_model_discovery/);
  assert.match(source, /settingsAI\.manualModel/);
  assert.match(source, /settingsAI\.refreshModels/);
});

test('支持模型发现的提供商刷新模型列表后应给出反馈并支持输入筛选', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /function getFilteredProviderModels\(\)/);
  assert.match(source, /filter\(model =>/);
  assert.match(source, /settingsAI\.loadedModels/);
  assert.match(source, /settingsAI\.suggestedModels/);
});

test('刷新模型列表按钮应显式调用刷新函数，避免点击事件对象覆盖提供商参数', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /on:click=\{\(\) => refreshProviderModels\(\)\}/);
  assert.doesNotMatch(source, /on:click=\{refreshProviderModels\}/);
});

test('模型列表为空时应保留当前模型值并继续允许手动输入', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /settingsAI\.currentModelLoading/);
  assert.match(source, /settingsAI\.currentModelMissing/);
  assert.match(source, /providerModelsLoading/);
  assert.match(source, /providerModelsError/);
});

test('建议列表应只展示匹配当前输入的实际模型，并明确区分手动输入值', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.doesNotMatch(source, /return \[config\.text_model\.model, \.\.\.providerModels\];/);
  assert.match(source, /settingsAI\.manualModelMissing/);
  assert.match(source, /#each getFilteredProviderModels\(\) as model \(model\)/);
});
