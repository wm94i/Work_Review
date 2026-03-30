import test from 'node:test';
import assert from 'node:assert/strict';

import {
  getAvatarActionLoopMeta,
  getAvatarIdleMotionMeta,
  getAvatarModeMeta,
  getAvatarMotionStepDelay,
  getAvatarStateBubble,
  getAvatarTransitionMeta,
} from './avatarStateMeta.js';

test('桌宠状态元信息应为不同模式提供细节变化', () => {
  const reading = getAvatarModeMeta('reading');
  const music = getAvatarModeMeta('music');
  const idle = getAvatarModeMeta('idle');

  assert.notEqual(reading.earTone, music.earTone);
  assert.notEqual(reading.cheekTone, music.cheekTone);
  assert.notEqual(reading.tailClass, music.tailClass);
  assert.equal(idle.tailClass, 'tail-idle');
});

test('桌宠状态切换气泡应返回短文案', () => {
  assert.deepEqual(getAvatarStateBubble('meeting', 'zh-CN'), {
    message: '开会中',
    tone: 'info',
    duration: 1800,
  });
  assert.deepEqual(getAvatarStateBubble('music', 'zh-TW'), {
    message: '聽歌中',
    tone: 'info',
    duration: 1800,
  });
  assert.deepEqual(getAvatarStateBubble('generating', 'en'), {
    message: 'Generating',
    tone: 'info',
    duration: 2000,
  });
  assert.deepEqual(getAvatarStateBubble('music', 'zh-CN'), {
    message: '听歌中',
    tone: 'info',
    duration: 1800,
  });
  assert.equal(getAvatarStateBubble('unknown'), null);
});

test('同一大状态下应根据上下文切换不同动作变体', () => {
  const writing = getAvatarModeMeta('working', '写作中');
  const chatting = getAvatarModeMeta('working', '沟通中');
  const planning = getAvatarModeMeta('working', '规划中');

  assert.notEqual(writing.leftPawClass, chatting.leftPawClass);
  assert.notEqual(chatting.tailClass, planning.tailClass);
  assert.notEqual(writing.mouthPath, planning.mouthPath);
});

test('阅读和调研应共享母状态但拥有不同动作节奏', () => {
  const reading = getAvatarModeMeta('reading', '阅读中');
  const researching = getAvatarModeMeta('reading', '调研中');

  assert.equal(reading.shellClass, researching.shellClass);
  assert.notEqual(reading.eyePath, researching.eyePath);
  assert.notEqual(reading.tailClass, researching.tailClass);
});

test('判断中应回退为中性动作而不是高活跃动作', () => {
  const judging = getAvatarModeMeta('working', '判断中');

  assert.equal(judging.leftPawClass, 'paw-rest');
  assert.equal(judging.rightPawClass, 'paw-rest');
  assert.equal(judging.tailClass, 'tail-observing');
});

test('会议、视频和生成中应拥有各自独立动作，不再复用完全相同的待机姿态', () => {
  const meeting = getAvatarModeMeta('meeting', '开会中');
  const video = getAvatarModeMeta('video', '视频中');
  const generating = getAvatarModeMeta('generating', '生成中');

  assert.equal(meeting.leftPawClass, 'paw-meeting-left');
  assert.equal(video.leftPawClass, 'paw-video-left');
  assert.equal(generating.leftPawClass, 'paw-generate-left');
  assert.notEqual(meeting.tailClass, video.tailClass);
  assert.notEqual(video.rightPawClass, generating.rightPawClass);
});

test('编码、排期、演示、学习和播客场景应支持更细的动作变体', () => {
  const coding = getAvatarModeMeta('working', '编码中');
  const scheduling = getAvatarModeMeta('working', '排期中');
  const demo = getAvatarModeMeta('meeting', '演示中');
  const normalMeeting = getAvatarModeMeta('meeting', '开会中');
  const learning = getAvatarModeMeta('video', '学习中');
  const normalVideo = getAvatarModeMeta('video', '视频中');
  const podcast = getAvatarModeMeta('music', '播客中');
  const music = getAvatarModeMeta('music', '听歌中');

  assert.notEqual(coding.tailClass, scheduling.tailClass);
  assert.notEqual(demo.mouthPath, normalMeeting.mouthPath);
  assert.notEqual(learning.eyePath, normalVideo.eyePath);
  assert.notEqual(podcast.cheekOpacity, music.cheekOpacity);
});

test('同一上下文下应轮换不同主动作而不是永远固定一个姿态', () => {
  const codingBeat0 = getAvatarActionLoopMeta('working', '编码中', 0);
  const codingBeat1 = getAvatarActionLoopMeta('working', '编码中', 1);
  const codingBeat2 = getAvatarActionLoopMeta('working', '编码中', 2);
  const codingBeat3 = getAvatarActionLoopMeta('working', '编码中', 3);
  const meetingBeat1 = getAvatarActionLoopMeta('meeting', '演示中', 1);
  const meetingBeat2 = getAvatarActionLoopMeta('meeting', '演示中', 2);

  assert.notEqual(codingBeat0.leftPawClass, codingBeat1.leftPawClass);
  assert.notEqual(codingBeat1.rightPawClass, codingBeat2.rightPawClass);
  assert.notEqual(codingBeat1.tailClass, codingBeat2.tailClass);
  assert.notEqual(codingBeat2.leftPawClass, codingBeat3.leftPawClass);
  assert.notEqual(meetingBeat1.mouthPath, meetingBeat2.mouthPath);
});

test('动作节拍应按状态生成不同停顿时长而不是固定三秒二', () => {
  const codingDelay0 = getAvatarMotionStepDelay('working', '编码中', 0);
  const codingDelay1 = getAvatarMotionStepDelay('working', '编码中', 1);
  const meetingDelay2 = getAvatarMotionStepDelay('meeting', '演示中', 2);
  const idleDelay3 = getAvatarMotionStepDelay('idle', '待机中', 3);

  assert.notEqual(codingDelay0, codingDelay1);
  assert.ok(codingDelay0 >= 1800);
  assert.ok(meetingDelay2 <= 5200);
  assert.ok(idleDelay3 >= 2800);
});

test('不同状态切换应返回对应过渡动作类型', () => {
  const toMeeting = getAvatarTransitionMeta('working', 'meeting', '写作中', '开会中');
  const fromMeeting = getAvatarTransitionMeta('meeting', 'working', '开会中', '写作中');
  const resumeFocus = getAvatarTransitionMeta('slacking', 'working', '休息中', '写作中');
  const videoToGenerating = getAvatarTransitionMeta('video', 'generating', '视频中', '生成中');

  assert.equal(toMeeting.className, 'transition-alert');
  assert.equal(fromMeeting.className, 'transition-settle');
  assert.equal(resumeFocus.className, 'transition-snap-back');
  assert.equal(videoToGenerating.className, 'transition-lift');
  assert.ok(toMeeting.durationMs > 0);
});

test('待机小动作应按节拍轮换而不是固定不变', () => {
  const readingBeat0 = getAvatarIdleMotionMeta('reading', '阅读中', 0);
  const readingBeat1 = getAvatarIdleMotionMeta('reading', '阅读中', 1);
  const judgingBeat2 = getAvatarIdleMotionMeta('working', '判断中', 2);
  const musicBeat1 = getAvatarIdleMotionMeta('music', '听歌中', 1);
  const generatingBeat2 = getAvatarIdleMotionMeta('generating', '生成中', 2);

  assert.notEqual(readingBeat0.headClass, readingBeat1.headClass);
  assert.equal(readingBeat0.shellClass, 'idle-breathe');
  assert.equal(judgingBeat2.shellClass, 'idle-observe');
  assert.equal(musicBeat1.shellClass, 'idle-groove');
  assert.equal(generatingBeat2.headClass, 'idle-head-focus');
});
