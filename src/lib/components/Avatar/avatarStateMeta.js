const MODE_META = {
  idle: {
    eyePath: 'M78 90 Q84 86 90 90 M110 90 Q116 86 122 90',
    mouthPath: 'M96 111 Q100 116 104 111',
    leftPawClass: 'paw-rest',
    rightPawClass: 'paw-rest',
    shellClass: 'mode-idle',
    tailClass: 'tail-idle',
    earTone: 'rgba(248, 218, 214, 0.92)',
    cheekTone: 'rgba(251, 214, 218, 0.52)',
    cheekOpacity: 0.42,
  },
  working: {
    eyePath: 'M78 89 Q84 95 90 89 M110 89 Q116 95 122 89',
    mouthPath: 'M95 110 Q100 118 105 110',
    leftPawClass: 'paw-work-left',
    rightPawClass: 'paw-work-right',
    shellClass: 'mode-working',
    tailClass: 'tail-working',
    earTone: 'rgba(202, 228, 255, 0.92)',
    cheekTone: 'rgba(191, 219, 254, 0.52)',
    cheekOpacity: 0.44,
  },
  reading: {
    eyePath: 'M79 92 Q85 87 91 92 M109 92 Q115 87 121 92',
    mouthPath: 'M94 111 Q100 118 106 111',
    leftPawClass: 'paw-rest',
    rightPawClass: 'paw-rest',
    shellClass: 'mode-reading',
    tailClass: 'tail-reading',
    earTone: 'rgba(224, 226, 236, 0.9)',
    cheekTone: 'rgba(214, 220, 235, 0.26)',
    cheekOpacity: 0.16,
  },
  meeting: {
    eyePath: 'M79 90 Q84 93 89 90 M111 90 Q116 93 121 90',
    mouthPath: 'M97 111 Q100 113 103 111',
    leftPawClass: 'paw-meeting-left',
    rightPawClass: 'paw-meeting-right',
    shellClass: 'mode-meeting',
    tailClass: 'tail-meeting',
    earTone: 'rgba(209, 218, 242, 0.92)',
    cheekTone: 'rgba(196, 208, 255, 0.34)',
    cheekOpacity: 0.28,
  },
  music: {
    eyePath: 'M78 89 Q84 96 90 89 M110 89 Q116 96 122 89',
    mouthPath: 'M94 110 Q100 121 106 110',
    leftPawClass: 'paw-music-left',
    rightPawClass: 'paw-music-right',
    shellClass: 'mode-music',
    tailClass: 'tail-music',
    earTone: 'rgba(244, 200, 226, 0.94)',
    cheekTone: 'rgba(249, 168, 212, 0.54)',
    cheekOpacity: 0.52,
  },
  video: {
    eyePath: 'M80 88 Q85 96 90 88 M110 88 Q115 96 120 88',
    mouthPath: 'M98 111 Q100 114 102 111',
    leftPawClass: 'paw-video-left',
    rightPawClass: 'paw-video-right',
    shellClass: 'mode-video',
    tailClass: 'tail-video',
    earTone: 'rgba(211, 234, 250, 0.94)',
    cheekTone: 'rgba(186, 230, 253, 0.38)',
    cheekOpacity: 0.26,
  },
  generating: {
    eyePath: 'M79 89 Q84 95 90 89 M110 89 Q116 95 121 89',
    mouthPath: 'M96 110 Q100 117 104 110',
    leftPawClass: 'paw-generate-left',
    rightPawClass: 'paw-generate-right',
    shellClass: 'mode-generating',
    tailClass: 'tail-generating',
    earTone: 'rgba(207, 244, 227, 0.94)',
    cheekTone: 'rgba(167, 243, 208, 0.44)',
    cheekOpacity: 0.34,
  },
  slacking: {
    eyePath: 'M79 90 Q84 95 90 90 M110 89 Q115 94 121 89',
    mouthPath: 'M94 109 Q100 119 106 109',
    leftPawClass: 'paw-rest',
    rightPawClass: 'paw-rest',
    shellClass: 'mode-slacking',
    tailClass: 'tail-slacking',
    earTone: 'rgba(255, 222, 200, 0.94)',
    cheekTone: 'rgba(253, 186, 116, 0.48)',
    cheekOpacity: 0.46,
  },
};

const MODE_VARIANTS = {
  working: {
    编码中: {
      leftPawClass: 'paw-work-left',
      rightPawClass: 'paw-work-right',
      tailClass: 'tail-coding',
      mouthPath: 'M96 111 Q100 114 104 111',
      eyePath: 'M78 88 Q84 93 90 88 M110 88 Q116 93 122 88',
      cheekOpacity: 0.32,
    },
    写作中: {
      leftPawClass: 'paw-write-left',
      rightPawClass: 'paw-write-right',
      tailClass: 'tail-writing',
      mouthPath: 'M95 111 Q100 115 105 111',
      cheekOpacity: 0.38,
    },
    规划中: {
      leftPawClass: 'paw-think-left',
      rightPawClass: 'paw-think-right',
      tailClass: 'tail-planning',
      mouthPath: 'M96 111 Q100 113 104 111',
      eyePath: 'M79 91 Q84 88 89 91 M111 91 Q116 88 121 91',
    },
    排期中: {
      leftPawClass: 'paw-think-left',
      rightPawClass: 'paw-think-right',
      tailClass: 'tail-scheduling',
      mouthPath: 'M96 111 Q100 112 104 111',
      eyePath: 'M79 91 Q84 89 89 91 M111 91 Q116 89 121 91',
      cheekOpacity: 0.2,
    },
    拆解中: {
      leftPawClass: 'paw-think-left',
      rightPawClass: 'paw-work-right',
      tailClass: 'tail-planning',
      mouthPath: 'M95 111 Q100 114 105 111',
      cheekOpacity: 0.28,
    },
    沟通中: {
      leftPawClass: 'paw-chat-left',
      rightPawClass: 'paw-chat-right',
      tailClass: 'tail-chatting',
      mouthPath: 'M95 110 Q100 120 105 110',
      eyePath: 'M78 90 Q84 97 90 90 M110 90 Q116 97 122 90',
    },
    创作中: {
      leftPawClass: 'paw-create-left',
      rightPawClass: 'paw-create-right',
      tailClass: 'tail-creative',
      mouthPath: 'M94 110 Q100 119 106 110',
      eyePath: 'M78 88 Q84 94 90 88 M110 88 Q116 94 122 88',
      cheekOpacity: 0.48,
    },
    总结中: {
      leftPawClass: 'paw-write-left',
      rightPawClass: 'paw-write-right',
      tailClass: 'tail-writing',
      mouthPath: 'M95 111 Q100 116 105 111',
      cheekOpacity: 0.34,
    },
    方案中: {
      leftPawClass: 'paw-think-left',
      rightPawClass: 'paw-write-right',
      tailClass: 'tail-planning',
      mouthPath: 'M95 111 Q100 114 105 111',
      cheekOpacity: 0.3,
    },
    判断中: {
      leftPawClass: 'paw-rest',
      rightPawClass: 'paw-rest',
      tailClass: 'tail-observing',
      mouthPath: 'M97 111 Q100 113 103 111',
      eyePath: 'M79 92 Q84 89 89 92 M111 92 Q116 89 121 92',
      cheekOpacity: 0.22,
    },
  },
  reading: {
    文档中: {
      eyePath: 'M79 91 Q85 88 91 91 M109 91 Q115 88 121 91',
      tailClass: 'tail-reading',
      mouthPath: 'M96 111 Q100 113 104 111',
      cheekOpacity: 0.14,
    },
    调研中: {
      eyePath: 'M78 90 Q84 95 90 90 M110 90 Q116 95 122 90',
      tailClass: 'tail-research',
      mouthPath: 'M95 111 Q100 114 105 111',
      cheekOpacity: 0.22,
    },
  },
  slacking: {
    休息中: {
      tailClass: 'tail-slacking',
      mouthPath: 'M95 109 Q100 118 105 109',
    },
  },
  meeting: {
    开会中: {
      mouthPath: 'M97 111 Q100 114 103 111',
      tailClass: 'tail-meeting',
      cheekOpacity: 0.24,
    },
    演示中: {
      mouthPath: 'M96 111 Q100 116 104 111',
      tailClass: 'tail-presenting',
      cheekOpacity: 0.2,
    },
    通话中: {
      mouthPath: 'M97 111 Q100 115 103 111',
      tailClass: 'tail-meeting',
      cheekOpacity: 0.22,
    },
  },
  music: {
    听歌中: {
      tailClass: 'tail-music-groove',
      cheekOpacity: 0.58,
    },
    播客中: {
      tailClass: 'tail-podcast',
      mouthPath: 'M96 110 Q100 114 104 110',
      cheekOpacity: 0.24,
    },
  },
  video: {
    视频中: {
      tailClass: 'tail-video-watch',
      mouthPath: 'M98 111 Q100 113 102 111',
      cheekOpacity: 0.22,
    },
    学习中: {
      tailClass: 'tail-learning',
      eyePath: 'M79 90 Q84 94 89 90 M111 90 Q116 94 121 90',
      mouthPath: 'M97 111 Q100 112 103 111',
      cheekOpacity: 0.16,
    },
    直播中: {
      tailClass: 'tail-video-watch',
      eyePath: 'M80 89 Q85 95 90 89 M110 89 Q115 95 120 89',
      cheekOpacity: 0.2,
    },
  },
  generating: {
    生成中: {
      tailClass: 'tail-generating-focus',
      mouthPath: 'M96 110 Q100 115 104 110',
      cheekOpacity: 0.3,
    },
  },
};

const STATE_BUBBLES = {
  idle: {
    'zh-CN': { message: '待机中', tone: 'info', duration: 1600 },
    'zh-TW': { message: '待機中', tone: 'info', duration: 1600 },
    en: { message: 'Idle', tone: 'info', duration: 1600 },
  },
  working: {
    'zh-CN': { message: '办公中', tone: 'info', duration: 1800 },
    'zh-TW': { message: '工作中', tone: 'info', duration: 1800 },
    en: { message: 'Working', tone: 'info', duration: 1800 },
  },
  reading: {
    'zh-CN': { message: '阅读中', tone: 'info', duration: 1800 },
    'zh-TW': { message: '閱讀中', tone: 'info', duration: 1800 },
    en: { message: 'Reading', tone: 'info', duration: 1800 },
  },
  meeting: {
    'zh-CN': { message: '开会中', tone: 'info', duration: 1800 },
    'zh-TW': { message: '開會中', tone: 'info', duration: 1800 },
    en: { message: 'Meeting', tone: 'info', duration: 1800 },
  },
  music: {
    'zh-CN': { message: '听歌中', tone: 'info', duration: 1800 },
    'zh-TW': { message: '聽歌中', tone: 'info', duration: 1800 },
    en: { message: 'Music', tone: 'info', duration: 1800 },
  },
  video: {
    'zh-CN': { message: '视频中', tone: 'info', duration: 1800 },
    'zh-TW': { message: '影片中', tone: 'info', duration: 1800 },
    en: { message: 'Video', tone: 'info', duration: 1800 },
  },
  generating: {
    'zh-CN': { message: '生成中', tone: 'info', duration: 2000 },
    'zh-TW': { message: '生成中', tone: 'info', duration: 2000 },
    en: { message: 'Generating', tone: 'info', duration: 2000 },
  },
  slacking: {
    'zh-CN': { message: '摸鱼中', tone: 'info', duration: 1800 },
    'zh-TW': { message: '摸魚中', tone: 'info', duration: 1800 },
    en: { message: 'On break', tone: 'info', duration: 1800 },
  },
};

const IDLE_MOTION_VARIANTS = {
  idle: [
    { shellClass: 'idle-breathe', headClass: 'idle-head-neutral' },
    { shellClass: 'idle-sway', headClass: 'idle-head-tilt' },
    { shellClass: 'idle-breathe', headClass: 'idle-head-peek' },
  ],
  reading: [
    { shellClass: 'idle-breathe', headClass: 'idle-head-neutral' },
    { shellClass: 'idle-breathe', headClass: 'idle-head-dip' },
    { shellClass: 'idle-sway', headClass: 'idle-head-peek' },
  ],
  working: [
    { shellClass: 'idle-breathe', headClass: 'idle-head-neutral' },
    { shellClass: 'idle-sway', headClass: 'idle-head-tilt' },
    { shellClass: 'idle-breathe', headClass: 'idle-head-dip' },
  ],
  judging: [
    { shellClass: 'idle-observe', headClass: 'idle-head-peek' },
    { shellClass: 'idle-observe', headClass: 'idle-head-tilt' },
    { shellClass: 'idle-observe', headClass: 'idle-head-neutral' },
  ],
  meeting: [
    { shellClass: 'idle-breathe', headClass: 'idle-head-neutral' },
    { shellClass: 'idle-sway', headClass: 'idle-head-dip' },
    { shellClass: 'idle-breathe', headClass: 'idle-head-tilt' },
  ],
  music: [
    { shellClass: 'idle-breathe', headClass: 'idle-head-neutral' },
    { shellClass: 'idle-groove', headClass: 'idle-head-bob' },
    { shellClass: 'idle-sway', headClass: 'idle-head-tilt' },
  ],
  video: [
    { shellClass: 'idle-breathe', headClass: 'idle-head-neutral' },
    { shellClass: 'idle-observe', headClass: 'idle-head-focus' },
    { shellClass: 'idle-sway', headClass: 'idle-head-dip' },
  ],
  generating: [
    { shellClass: 'idle-breathe', headClass: 'idle-head-focus' },
    { shellClass: 'idle-focus-pulse', headClass: 'idle-head-tilt' },
    { shellClass: 'idle-sway', headClass: 'idle-head-focus' },
  ],
  slacking: [
    { shellClass: 'idle-breathe', headClass: 'idle-head-neutral' },
    { shellClass: 'idle-sway', headClass: 'idle-head-peek' },
    { shellClass: 'idle-breathe', headClass: 'idle-head-tilt' },
  ],
};

const ACTION_LOOP_VARIANTS = {
  idle: {
    default: [
      {},
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-tight',
        mouthPath: 'M96 111 Q100 115 104 111',
      },
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-flick',
        eyePath: 'M79 91 Q84 88 89 91 M111 91 Q116 88 121 91',
      },
    ],
  },
  working: {
    default: [
      {},
      {
        leftPawClass: 'paw-work-left',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-tight',
        mouthPath: 'M96 111 Q100 114 104 111',
      },
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-work-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    编码中: [
      {},
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-work-right',
        tailClass: 'tail-loop-tight',
        mouthPath: 'M97 111 Q100 113 103 111',
      },
      {
        leftPawClass: 'paw-work-left',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-flick',
        eyePath: 'M79 90 Q84 94 89 90 M111 90 Q116 94 121 90',
      },
    ],
    写作中: [
      {},
      {
        leftPawClass: 'paw-write-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-wide',
        mouthPath: 'M95 111 Q100 116 105 111',
      },
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-write-right',
        tailClass: 'tail-loop-tight',
      },
    ],
    排期中: [
      {},
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-tight',
        mouthPath: 'M97 111 Q100 112 103 111',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    沟通中: [
      {},
      {
        leftPawClass: 'paw-chat-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-wide',
        mouthPath: 'M95 110 Q100 118 105 110',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-chat-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    创作中: [
      {},
      {
        leftPawClass: 'paw-create-left',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-wide',
      },
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-create-right',
        tailClass: 'tail-loop-flick',
        mouthPath: 'M95 110 Q100 117 105 110',
      },
    ],
    判断中: [
      {},
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-tight',
      },
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-flick',
      },
    ],
  },
  reading: {
    default: [
      {},
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-tight',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    文档中: [
      {},
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-tight',
        mouthPath: 'M97 111 Q100 112 103 111',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    调研中: [
      {},
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-work-right',
        tailClass: 'tail-loop-wide',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-flick',
        eyePath: 'M79 91 Q84 96 89 91 M111 91 Q116 96 121 91',
      },
    ],
  },
  meeting: {
    default: [
      {},
      {
        leftPawClass: 'paw-meeting-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-tight',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-meeting-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    开会中: [
      {},
      {
        leftPawClass: 'paw-meeting-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-tight',
        mouthPath: 'M97 111 Q100 114 103 111',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-meeting-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    演示中: [
      {},
      {
        leftPawClass: 'paw-chat-left',
        rightPawClass: 'paw-meeting-right',
        tailClass: 'tail-loop-wide',
        mouthPath: 'M95 111 Q100 117 105 111',
      },
      {
        leftPawClass: 'paw-meeting-left',
        rightPawClass: 'paw-chat-right',
        tailClass: 'tail-loop-flick',
        mouthPath: 'M97 111 Q100 113 103 111',
      },
    ],
  },
  music: {
    default: [
      {},
      {
        leftPawClass: 'paw-music-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-wide',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-music-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    听歌中: [
      {},
      {
        leftPawClass: 'paw-music-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-wide',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-music-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    播客中: [
      {},
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-tight',
        mouthPath: 'M97 110 Q100 113 103 110',
      },
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-flick',
      },
    ],
  },
  video: {
    default: [
      {},
      {
        leftPawClass: 'paw-video-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-tight',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-video-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    视频中: [
      {},
      {
        leftPawClass: 'paw-video-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-tight',
      },
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-video-right',
        tailClass: 'tail-loop-flick',
      },
    ],
    学习中: [
      {},
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-video-right',
        tailClass: 'tail-loop-wide',
        mouthPath: 'M97 111 Q100 113 103 111',
      },
      {
        leftPawClass: 'paw-video-left',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-flick',
      },
    ],
  },
  generating: {
    default: [
      {},
      {
        leftPawClass: 'paw-generate-left',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-tight',
      },
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-generate-right',
        tailClass: 'tail-loop-wide',
        mouthPath: 'M97 110 Q100 114 103 110',
      },
    ],
    生成中: [
      {},
      {
        leftPawClass: 'paw-generate-left',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-tight',
      },
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-generate-right',
        tailClass: 'tail-loop-wide',
        mouthPath: 'M97 110 Q100 114 103 110',
      },
    ],
  },
  slacking: {
    default: [
      {},
      {
        leftPawClass: 'paw-rest',
        rightPawClass: 'paw-think-right',
        tailClass: 'tail-loop-tight',
      },
      {
        leftPawClass: 'paw-think-left',
        rightPawClass: 'paw-rest',
        tailClass: 'tail-loop-flick',
      },
    ],
  },
};

const ACTION_LOOP_PATTERNS = {
  idle: {
    default: [0, 1, 0, 2, 0, 1, 2, 0],
  },
  working: {
    default: [0, 1, 0, 2, 1, 0, 2, 0],
    编码中: [0, 1, 0, 2, 1, 0, 1, 2],
    写作中: [0, 1, 0, 1, 2, 0, 1, 0],
    排期中: [0, 1, 0, 2, 0, 1, 0, 2],
    沟通中: [0, 1, 2, 1, 0, 2, 1, 0],
    创作中: [0, 1, 2, 0, 1, 0, 2, 1],
    判断中: [0, 0, 1, 0, 2, 0, 1, 0],
  },
  reading: {
    default: [0, 1, 0, 2, 0, 1, 0, 2],
    文档中: [0, 1, 0, 1, 2, 0, 1, 0],
    调研中: [0, 1, 2, 0, 1, 0, 2, 1],
  },
  meeting: {
    default: [0, 1, 0, 2, 1, 0, 2, 0],
    开会中: [0, 1, 0, 2, 0, 1, 2, 0],
    演示中: [0, 1, 2, 1, 0, 2, 1, 0],
  },
  music: {
    default: [0, 1, 2, 1, 0, 2, 1, 0],
    听歌中: [0, 1, 2, 1, 0, 2, 1, 0],
    播客中: [0, 0, 1, 0, 2, 0, 1, 0],
  },
  video: {
    default: [0, 1, 0, 2, 0, 1, 2, 0],
    视频中: [0, 1, 0, 2, 0, 1, 2, 0],
    学习中: [0, 1, 2, 0, 1, 0, 2, 0],
  },
  generating: {
    default: [0, 1, 0, 2, 1, 0, 2, 0],
    生成中: [0, 1, 0, 2, 1, 0, 2, 0],
  },
  slacking: {
    default: [0, 0, 1, 0, 2, 0, 1, 0],
  },
};

const MOTION_STEP_DELAYS = {
  idle: {
    default: [4200, 5200, 3600, 6100, 3400, 4700, 5800, 3900],
  },
  working: {
    default: [2600, 3400, 2200, 3800, 2400, 3100, 3600, 2500],
    编码中: [2200, 3000, 1800, 3400, 2100, 2800, 3600, 2000],
    写作中: [3200, 4600, 2800, 4200, 3000, 5200, 3400, 4100],
    排期中: [3400, 4800, 2600, 4300, 3000, 5200, 2800, 3900],
    沟通中: [1800, 2400, 2000, 2800, 1900, 2600, 2200, 3000],
    创作中: [2400, 3800, 2200, 4200, 2600, 3600, 3000, 4500],
    判断中: [3600, 5200, 3000, 4800, 3400, 5600, 3200, 4300],
  },
  reading: {
    default: [3400, 4600, 3000, 5200, 3200, 4300, 5600, 3600],
    文档中: [3200, 4200, 2800, 5100, 3400, 4500, 5400, 3600],
    调研中: [2600, 3400, 2400, 4000, 2200, 3200, 3800, 2800],
  },
  meeting: {
    default: [2200, 2800, 2000, 3200, 2100, 2600, 3400, 2300],
    开会中: [2200, 3000, 2100, 3400, 2000, 2800, 3200, 2300],
    演示中: [1800, 2400, 2100, 2800, 1900, 2600, 3000, 2200],
  },
  music: {
    default: [2000, 2600, 1800, 3000, 1900, 2400, 3200, 2100],
    听歌中: [1800, 2400, 1700, 2800, 1900, 2500, 3000, 2000],
    播客中: [3400, 5000, 3000, 4200, 3600, 5400, 3200, 4600],
  },
  video: {
    default: [3200, 4200, 2800, 5000, 3000, 4300, 5600, 3500],
    视频中: [3200, 4300, 2900, 5200, 3100, 4400, 5600, 3600],
    学习中: [2800, 3800, 2500, 4600, 2700, 3600, 5000, 3200],
  },
  generating: {
    default: [2100, 2900, 1900, 3400, 2200, 3000, 3600, 2400],
    生成中: [2100, 3000, 1800, 3600, 2200, 3200, 3800, 2000],
  },
  slacking: {
    default: [4200, 5600, 3600, 6200, 4000, 5200, 6400, 4300],
  },
};

export function getAvatarModeMeta(mode, contextLabel = '') {
  const baseMeta = MODE_META[mode] || MODE_META.idle;
  const variantMeta = MODE_VARIANTS[mode]?.[contextLabel] || {};

  return {
    ...baseMeta,
    ...variantMeta,
  };
}

export function getAvatarStateBubble(mode, locale = 'zh-CN') {
  const messageSet = STATE_BUBBLES[mode];
  if (!messageSet) {
    return null;
  }
  return messageSet[locale] || messageSet['zh-CN'] || null;
}

export function getAvatarActionLoopMeta(mode, contextLabel = '', beat = 0) {
  const variantsByMode = ACTION_LOOP_VARIANTS[mode] || ACTION_LOOP_VARIANTS.idle;
  const sequence =
    variantsByMode?.[contextLabel] ||
    variantsByMode?.default ||
    ACTION_LOOP_VARIANTS.idle.default;
  const patternsByMode = ACTION_LOOP_PATTERNS[mode] || ACTION_LOOP_PATTERNS.idle;
  const pattern =
    patternsByMode?.[contextLabel] ||
    patternsByMode?.default ||
    ACTION_LOOP_PATTERNS.idle.default;
  const patternIndex = Math.abs(Number(beat) || 0) % pattern.length;
  const index = pattern[patternIndex] % sequence.length;

  return sequence[index];
}

export function getAvatarMotionStepDelay(mode, contextLabel = '', beat = 0) {
  const delaysByMode = MOTION_STEP_DELAYS[mode] || MOTION_STEP_DELAYS.idle;
  const sequence =
    delaysByMode?.[contextLabel] ||
    delaysByMode?.default ||
    MOTION_STEP_DELAYS.idle.default;
  const index = Math.abs(Number(beat) || 0) % sequence.length;

  return sequence[index];
}

export function getAvatarTransitionMeta(
  fromMode,
  toMode,
  fromContextLabel = '',
  toContextLabel = '',
) {
  if (!fromMode || fromMode === toMode) {
    return { className: '', durationMs: 0 };
  }

  if (toContextLabel === '生成中') {
    return { className: 'transition-lift', durationMs: 660 };
  }

  if (toMode === 'meeting') {
    return { className: 'transition-alert', durationMs: 720 };
  }

  if (
    fromMode === 'meeting' &&
    ['写作中', '办公中', '规划中'].includes(toContextLabel)
  ) {
    return { className: 'transition-settle', durationMs: 680 };
  }

  if (fromMode === 'slacking' && toMode === 'working') {
    return { className: 'transition-snap-back', durationMs: 760 };
  }

  if (
    (fromMode === 'working' && toMode === 'reading') ||
    (fromContextLabel === '调研中' && ['写作中', '规划中'].includes(toContextLabel))
  ) {
    return { className: 'transition-focus-shift', durationMs: 640 };
  }

  if (fromMode === 'meeting') {
    return { className: 'transition-settle', durationMs: 680 };
  }

  return { className: 'transition-glide', durationMs: 620 };
}

export function getAvatarIdleMotionMeta(mode, contextLabel = '', beat = 0) {
  const sequenceKey =
    mode === 'working' && contextLabel === '判断中'
      ? 'judging'
      : IDLE_MOTION_VARIANTS[mode]
        ? mode
        : 'idle';
  const sequence = IDLE_MOTION_VARIANTS[sequenceKey];
  const index = Math.abs(Number(beat) || 0) % sequence.length;

  return sequence[index];
}
