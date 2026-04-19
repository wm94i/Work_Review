import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { formatBubbleMessage } from './bubbleMessage.js';

test('桌宠应直接复用开源 BongoCat 图层，而不是继续手绘 SVG 轮廓', () => {
  const source = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const registrySource = readFileSync(new URL('./avatarPresetRegistry.js', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /getAvatarOutline/);
  assert.doesNotMatch(source, /AVATAR_OUTLINE_LAYOUT/);
  assert.match(registrySource, /assets\/bongocat\/mouse-bg\.png/);
  assert.match(registrySource, /assets\/bongocat\/standard-keyboard-0\.png/);
  assert.match(registrySource, /assets\/bongocat\/standard-hand-0\.png/);
  assert.match(registrySource, /assets\/bongocat\/standard-up\.png/);
  assert.match(source, /<svg/);
  assert.match(source, /<image/);
  assert.match(source, /sceneSrc/);
  assert.match(source, /standardHandSrc/);
  assert.match(source, /keyOverlaySrc/);
  assert.match(source, /computeStandardMouseGeometry/);
  assert.match(source, /mouseArmPoints/);
  assert.match(source, /getAvatarPresetDefinition/);
  assert.match(source, /showKeyboardLayers/);
  assert.match(source, /frameIndex/);
});

test('BongoCat 资源文件和来源说明应落在桌宠组件目录内', () => {
  assert.equal(
    existsSync(new URL('./assets/bongocat/standard-bg.png', import.meta.url)),
    true
  );
  assert.equal(
    existsSync(new URL('./assets/bongocat/mouse-bg.png', import.meta.url)),
    true
  );
  assert.equal(
    existsSync(new URL('./assets/bongocat/standard-hand-0.png', import.meta.url)),
    true
  );
  assert.equal(
    existsSync(new URL('./assets/bongocat/model-standard/background.png', import.meta.url)),
    true
  );
  assert.equal(
    existsSync(new URL('./assets/bongocat/model-standard/cover.png', import.meta.url)),
    true
  );
  assert.equal(
    existsSync(new URL('./assets/bongocat/model-gamepad/background.png', import.meta.url)),
    true
  );
  assert.equal(
    existsSync(new URL('./assets/bongocat/model-gamepad/cover.png', import.meta.url)),
    true
  );
  assert.equal(
    existsSync(new URL('./assets/bongocat/standard-keyboard-0.png', import.meta.url)),
    true
  );
  assert.equal(
    existsSync(new URL('./assets/bongocat/standard-up.png', import.meta.url)),
    true
  );
  assert.equal(
    existsSync(new URL('./assets/bongocat/README.md', import.meta.url)),
    true
  );
});

test('桌宠路径不应复用全局 outline 类名，避免 SVG 包围框泄漏', () => {
  const source = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const windowSource = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');
  const engineSource = readFileSync(new URL('../../../../src-tauri/src/avatar_engine.rs', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /class="outline"/);
  assert.match(source, /avatar-shell/);
  assert.match(source, /--avatar-shell-opacity/);
  assert.match(source, /getAvatarIdleMotionMeta/);
  assert.match(source, /state\.contextLabel/);
  assert.match(source, /motionBeat/);
  assert.match(source, /transitionClass/);
  assert.match(source, /idle-breathe/);
  assert.match(source, /transition-focus-shift/);
  assert.match(source, /scene-svg/);
  assert.match(source, /pointer-events:\s*none/);
  assert.match(source, /showKeyboardLayers/);
  assert.match(source, /Math\.floor\(motionBeat \/ 2\)/);
  assert.match(source, /keyboard-layer/);
  assert.match(source, /standard-hand-layer/);
  assert.match(source, /mouse-arm-fill/);
  assert.match(source, /mouse-device-layer/);
  assert.doesNotMatch(windowSource, /aria-label="打开主界面"/);
  assert.match(windowSource, /on:avatarpointerdown=\{startAvatarDrag\}/);
  assert.match(windowSource, /on:avataractivate=\{openMainWindow\}/);
  assert.match(windowSource, /getAvatarTransitionMeta/);
  assert.match(windowSource, /getAvatarMotionStepDelay/);
  assert.match(windowSource, /motionBeat = \(motionBeat \+ 1\) % 96/);
  assert.match(windowSource, /scheduleNextMotionStep/);
  assert.match(windowSource, /transitionClass = transition\.className/);
  assert.match(windowSource, /getAvatarStateBubble/);
  assert.match(windowSource, /showBubble\(stateBubble\)/);
  assert.match(
    windowSource,
    /nextState\.mode !== state\.mode\s*\|\|\s*nextState\.contextLabel !== state\.contextLabel/
  );
  assert.match(engineSource, /const AVATAR_SCALE_DEFAULT: f64 = 0\.9;/);
  assert.match(engineSource, /const AVATAR_WINDOW_BASE_WIDTH: f64 = 276\.0;/);
  assert.match(engineSource, /const AVATAR_WINDOW_BASE_HEIGHT: f64 = 248\.0;/);
  assert.match(engineSource, /const AVATAR_WINDOW_MARGIN: f64 = 8\.0;/);
});

test('状态气泡应采用贴头短气泡，减少白卡感和视觉侵入', () => {
  const source = readFileSync(new URL('./AvatarPopover.svelte', import.meta.url), 'utf8');
  const windowSource = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /Array\.from\(bubble\.message\.replace\(\/\\s\+\/g, ''\)\)/);
  assert.match(source, /class="avatar-popover-anchor absolute right-\[8%\] top-\[8px\]"/);
  assert.doesNotMatch(source, /writing-mode: vertical-rl/);
  assert.match(source, /compactBubbleMessage/);
  assert.match(source, /bubblePanelStyle = compactBubbleMessage/);
  assert.match(source, /width: fit-content; min-width: 120px; max-width: min\(68vw, 196px\);/);
  assert.match(source, /width: min\(88vw, 336px\); min-width: 180px; max-width: min\(88vw, 336px\);/);
  assert.match(source, /min-height: 40px/);
  assert.match(source, /padding: 6px 14px 7px 14px/);
  assert.match(source, /display: block;/);
  assert.match(source, /text-\[12px\] font-semibold leading-\[1\.35\]/);
  assert.match(source, /rounded-\[16px\]/);
  assert.match(source, /shadow-\[0_10px_24px_rgba\(15,23,42,0\.1\),0_3px_10px_rgba\(15,23,42,0\.05\)\]/);
  assert.match(source, /word-break: normal/);
  assert.match(source, /overflow-wrap: anywhere/);
  assert.match(source, /white-space: normal/);
  assert.doesNotMatch(source, /line-break: strict/);
  assert.match(source, /text-align: \{compactBubbleMessage \? 'center' : 'left'\}/);
  assert.match(source, /bubble-tail/);
  assert.match(source, /bubble-tail-dot/);
  assert.match(windowSource, /h-\[86px\]/);
  assert.match(windowSource, /top-\[78px\]/);
  assert.match(windowSource, /class="h-full w-\[82%\]"/);
  assert.doesNotMatch(source, /-translate-x-1\/2/);
});

test('休息提醒气泡应支持常驻显示和手动关闭', () => {
  const source = readFileSync(new URL('./AvatarPopover.svelte', import.meta.url), 'utf8');
  const windowSource = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.match(source, /export let onClose = \(\) => \{\};/);
  assert.match(source, /bubble\?\.persistent/);
  assert.match(source, /class="absolute inset-0 z-20 overflow-visible pointer-events-none"/);
  assert.match(source, /class="pointer-events-auto relative rounded-\[16px\]/);
  assert.match(source, /<button[\s\S]*type="button"[\s\S]*on:click=\{onClose\}/);
  assert.match(source, /aria-label="关闭提醒"/);
  assert.match(source, /class="absolute inset-0 rounded-\[16px\]"/);
  assert.match(source, /class="bubble-tail-dot absolute/);
  assert.match(windowSource, /<AvatarPopover \{bubble\} onClose=\{dismissBubble\} \/>/);
  assert.match(windowSource, /if \(!payload\?\.persistent\)/);
});

test('英文桌宠气泡文案应保留单词间空格', () => {
  assert.equal(
    formatBubbleMessage('Time for a break. Stand up and stretch a bit.'),
    'Time for a break. Stand up and stretch a bit.'
  );
});

test('接上次继续卡片应保留可操作结构，但与轻气泡共用圆角和尾巴语言', () => {
  const source = readFileSync(new URL('./AvatarFollowupCard.svelte', import.meta.url), 'utf8');
  const windowSource = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.match(source, /w-\[min\(92vw,348px\)\]/);
  assert.match(source, /max-w-\[348px\]/);
  assert.match(source, /flex-col overflow-hidden/);
  assert.match(source, /style="max-height: calc\(100vh - 16px\);"/);
  assert.match(source, /min-h-0 flex-1 overflow-y-auto pr-1/);
  assert.match(source, /mt-3 shrink-0 space-y-2/);
  assert.match(source, /rounded-\[16px\]/);
  assert.match(source, /bg-\[rgba\(255,255,255,0\.96\)\]/);
  assert.match(source, /shadow-\[0_14px_28px_rgba\(15,23,42,0\.14\),0_4px_12px_rgba\(15,23,42,0\.07\)\]/);
  assert.match(source, /copy\.surfaceClass/);
  assert.match(source, /copy\.badgeClass/);
  assert.match(source, /copy\.primaryClass/);
  assert.match(source, /shrink-0 whitespace-nowrap/);
  assert.match(source, /grid grid-cols-\[auto,minmax\(0,1fr\)\] items-start gap-2 pr-7/);
  assert.match(source, /CLAMP_STYLE_TWO_LINES/);
  assert.match(source, /CLAMP_STYLE_FOUR_LINES/);
  assert.match(source, /followup-tail/);
  assert.match(source, /flex flex-col gap-2/);
  assert.match(source, /border border-slate-200 bg-white/);
  assert.match(source, /inline-flex w-full items-center justify-center/);
  assert.match(windowSource, /truncateFollowupTitle/);
  assert.match(
    windowSource,
    /<AvatarFollowupCard[\s\S]*followup=\{followup\}[\s\S]*onDismiss=\{dismissFollowup\}[\s\S]*\/>/
  );
});

test('桌宠键盘反馈应恢复单键模式，不再显示组合键提示浮层', () => {
  const source = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /keyboard-combo-indicator/);
  assert.doesNotMatch(source, /keyboard-combo-chip/);
});
