import mouseBg from './assets/bongocat/mouse-bg.png';
import modelStandardBackground from './assets/bongocat/model-standard/background.png';
import modelStandardCover from './assets/bongocat/model-standard/cover.png';
import modelGamepadBackground from './assets/bongocat/model-gamepad/background.png';
import modelGamepadCover from './assets/bongocat/model-gamepad/cover.png';
import standardUp from './assets/bongocat/standard-up.png';
import standardHand0 from './assets/bongocat/standard-hand-0.png';
import standardHand1 from './assets/bongocat/standard-hand-1.png';
import standardHand2 from './assets/bongocat/standard-hand-2.png';
import standardHand3 from './assets/bongocat/standard-hand-3.png';
import standardHand4 from './assets/bongocat/standard-hand-4.png';
import standardHand5 from './assets/bongocat/standard-hand-5.png';
import standardHand6 from './assets/bongocat/standard-hand-6.png';
import standardHand7 from './assets/bongocat/standard-hand-7.png';
import standardHand8 from './assets/bongocat/standard-hand-8.png';
import standardHand9 from './assets/bongocat/standard-hand-9.png';
import standardHand10 from './assets/bongocat/standard-hand-10.png';
import standardHand11 from './assets/bongocat/standard-hand-11.png';
import standardHand12 from './assets/bongocat/standard-hand-12.png';
import standardHand13 from './assets/bongocat/standard-hand-13.png';
import standardHand14 from './assets/bongocat/standard-hand-14.png';
import standardKeyboard0 from './assets/bongocat/standard-keyboard-0.png';
import standardKeyboard1 from './assets/bongocat/standard-keyboard-1.png';
import standardKeyboard2 from './assets/bongocat/standard-keyboard-2.png';
import standardKeyboard3 from './assets/bongocat/standard-keyboard-3.png';
import standardKeyboard4 from './assets/bongocat/standard-keyboard-4.png';
import standardKeyboard5 from './assets/bongocat/standard-keyboard-5.png';
import standardKeyboard6 from './assets/bongocat/standard-keyboard-6.png';
import standardKeyboard7 from './assets/bongocat/standard-keyboard-7.png';
import standardKeyboard8 from './assets/bongocat/standard-keyboard-8.png';
import standardKeyboard9 from './assets/bongocat/standard-keyboard-9.png';
import standardKeyboard10 from './assets/bongocat/standard-keyboard-10.png';
import standardKeyboard11 from './assets/bongocat/standard-keyboard-11.png';
import standardKeyboard12 from './assets/bongocat/standard-keyboard-12.png';
import standardKeyboard13 from './assets/bongocat/standard-keyboard-13.png';
import standardKeyboard14 from './assets/bongocat/standard-keyboard-14.png';
import mouseDevice from './assets/bongocat/mouse.png';
import mouseLeft from './assets/bongocat/mouse_left.png';
import mouseRight from './assets/bongocat/mouse_right.png';
import mouseSide from './assets/bongocat/mouse_side.png';

export const AVATAR_PRESET_DEFAULT = 'original-standard';

function createPngLayerMap(layerEntries) {
  return Object.fromEntries(
    Object.entries(layerEntries).map(
      ([path, source]) => {
        const filename = path.split('/').pop() ?? '';
        const stem = filename.replace(/\.png$/i, '');
        return [stem, source];
      }
    )
  );
}

function aliasLayers(baseLayers, aliasGroups) {
  const nextLayers = {};

  for (const [aliasName, baseName] of aliasGroups) {
    if (baseLayers[baseName]) {
      nextLayers[aliasName] = baseLayers[baseName];
    }
  }

  return nextLayers;
}

const HOTSPOT_CYAN_FILL = 'rgba(34, 211, 238, 0.32)';
const HOTSPOT_CYAN_STROKE = 'rgba(8, 145, 178, 0.72)';
const HOTSPOT_PINK_FILL = 'rgba(244, 114, 182, 0.34)';
const HOTSPOT_PINK_STROKE = 'rgba(225, 29, 72, 0.62)';
const HOTSPOT_YELLOW_FILL = 'rgba(250, 204, 21, 0.34)';
const HOTSPOT_YELLOW_STROKE = 'rgba(202, 138, 4, 0.62)';
const HOTSPOT_WHITE_FILL = 'rgba(255, 255, 255, 0.28)';
const HOTSPOT_WHITE_STROKE = 'rgba(71, 85, 105, 0.48)';

function rectHotspot(x, y, width, height, transform, fill = HOTSPOT_CYAN_FILL, stroke = HOTSPOT_CYAN_STROKE) {
  return { kind: 'rect', x, y, width, height, rx: 5, fill, stroke, transform };
}

function ellipseHotspot(cx, cy, rx, ry, fill = HOTSPOT_CYAN_FILL, stroke = HOTSPOT_CYAN_STROKE) {
  return { kind: 'ellipse', cx, cy, rx, ry, fill, stroke };
}

function polygonHotspot(points, fill = HOTSPOT_CYAN_FILL, stroke = HOTSPOT_CYAN_STROKE) {
  return { kind: 'polygon', points, fill, stroke };
}

function motionBounds(x, y, width, height) {
  return { x, y, width, height };
}

function mouseMotionModel(anchorX, anchorY, boundsX, boundsY, boundsWidth, boundsHeight, bend = 0.24) {
  return {
    anchorX,
    anchorY,
    bounds: motionBounds(boundsX, boundsY, boundsWidth, boundsHeight),
    bend,
  };
}

const STANDARD_KEYBOARD_FRAME_BY_GROUP = {
  'digit-1': standardKeyboard0,
  'digit-2': standardKeyboard1,
  'digit-3': standardKeyboard2,
  'digit-4': standardKeyboard3,
  'digit-5': standardKeyboard4,
  'digit-6': standardKeyboard5,
  'digit-7': standardKeyboard6,
  'key-q': standardKeyboard7,
  'key-e': standardKeyboard8,
  'key-r': standardKeyboard9,
  space: standardKeyboard10,
  'key-a': standardKeyboard11,
  'key-d': standardKeyboard12,
  'key-s': standardKeyboard13,
  'key-w': standardKeyboard14,
};

const STANDARD_HAND_FRAME_BY_GROUP = {
  'digit-1': standardHand0,
  'digit-2': standardHand1,
  'digit-3': standardHand2,
  'digit-4': standardHand3,
  'digit-5': standardHand4,
  'digit-6': standardHand5,
  'digit-7': standardHand6,
  'key-q': standardHand7,
  'key-e': standardHand8,
  'key-r': standardHand9,
  space: standardHand10,
  'key-a': standardHand11,
  'key-d': standardHand12,
  'key-s': standardHand13,
  'key-w': standardHand14,
};

const STANDARD_MODEL_LEFT_KEYS = createPngLayerMap(
  import.meta.glob('./assets/bongocat/model-standard/left-keys/*.png', {
    eager: true,
    import: 'default',
  })
);
const GAMEPAD_MODEL_LEFT_KEYS = createPngLayerMap(
  import.meta.glob('./assets/bongocat/model-gamepad/left-keys/*.png', {
    eager: true,
    import: 'default',
  })
);
const GAMEPAD_MODEL_RIGHT_KEYS = createPngLayerMap(
  import.meta.glob('./assets/bongocat/model-gamepad/right-keys/*.png', {
    eager: true,
    import: 'default',
  })
);

const GAMEPAD_KEYBOARD_LAYER_BY_KEY = aliasLayers(GAMEPAD_MODEL_LEFT_KEYS, [
  ['BackQuote', 'LeftTrigger'],
  ['Num1', 'LeftTrigger'],
  ['Num2', 'LeftTrigger'],
  ['Num3', 'LeftTrigger'],
  ['Num4', 'LeftTrigger'],
  ['Num5', 'LeftTrigger'],
  ['Num6', 'LeftTrigger'],
  ['Num7', 'LeftTrigger'],
  ['Num8', 'LeftTrigger'],
  ['Num9', 'LeftTrigger'],
  ['Num0', 'LeftTrigger'],
  ['Escape', 'LeftTrigger2'],
  ['Tab', 'LeftTrigger2'],
  ['CapsLock', 'LeftTrigger2'],
  ['Shift', 'LeftTrigger2'],
  ['ShiftLeft', 'LeftTrigger2'],
  ['ShiftRight', 'LeftTrigger2'],
  ['Control', 'LeftTrigger2'],
  ['ControlLeft', 'LeftTrigger2'],
  ['ControlRight', 'LeftTrigger2'],
  ['Alt', 'LeftTrigger2'],
  ['AltGr', 'LeftTrigger2'],
  ['Meta', 'LeftTrigger2'],
  ['Fn', 'LeftTrigger2'],
  ['Backspace', 'LeftTrigger2'],
  ['Delete', 'LeftTrigger2'],
  ['Return', 'DPadRight'],
  ['Slash', 'DPadRight'],
  ['KeyH', 'DPadRight'],
  ['KeyJ', 'DPadRight'],
  ['KeyK', 'DPadRight'],
  ['KeyL', 'DPadRight'],
  ['KeyW', 'DPadUp'],
  ['KeyE', 'DPadUp'],
  ['KeyR', 'DPadUp'],
  ['KeyT', 'DPadUp'],
  ['KeyY', 'DPadUp'],
  ['KeyU', 'DPadUp'],
  ['KeyI', 'DPadUp'],
  ['KeyO', 'DPadUp'],
  ['KeyP', 'DPadUp'],
  ['KeyQ', 'DPadLeft'],
  ['KeyA', 'DPadLeft'],
  ['KeyS', 'DPadDown'],
  ['KeyD', 'DPadRight'],
  ['KeyF', 'DPadRight'],
  ['KeyG', 'DPadRight'],
  ['KeyZ', 'DPadLeft'],
  ['KeyX', 'DPadLeft'],
  ['KeyC', 'DPadDown'],
  ['KeyV', 'DPadDown'],
  ['KeyB', 'DPadDown'],
  ['KeyN', 'DPadDown'],
  ['KeyM', 'DPadDown'],
  ['Space', 'DPadDown'],
]);

const GAMEPAD_MOUSE_LAYER_BY_GROUP = {
  'mouse-move': GAMEPAD_MODEL_RIGHT_KEYS.West,
  'mouse-left': GAMEPAD_MODEL_RIGHT_KEYS.South,
  'mouse-right': GAMEPAD_MODEL_RIGHT_KEYS.East,
  'mouse-side': GAMEPAD_MODEL_RIGHT_KEYS.RightTrigger,
};

const MOUSE_DEVICE_OVERLAY_BY_GROUP = {
  'mouse-left': mouseLeft,
  'mouse-right': mouseRight,
  'mouse-side': mouseSide,
};

const STANDARD_BASE = {
  keyboardFrames: STANDARD_KEYBOARD_FRAME_BY_GROUP,
  handFrames: STANDARD_HAND_FRAME_BY_GROUP,
  idleHand: standardUp,
  mouseDeviceBase: mouseDevice,
  mouseDeviceOverlays: MOUSE_DEVICE_OVERLAY_BY_GROUP,
};

const STANDARD_MODEL_INTERACTION = {
  trackpad: polygonHotspot('44,126 214,101 242,204 73,229', HOTSPOT_CYAN_FILL, HOTSPOT_CYAN_STROKE),
  keyboardHotspots: {
    'digit-1': [rectHotspot(305, 236, 22, 13, 'rotate(-10 316 242)')],
    'digit-2': [rectHotspot(329, 232, 22, 13, 'rotate(-10 340 238)')],
    'digit-3': [rectHotspot(352, 228, 22, 13, 'rotate(-10 363 234)')],
    'digit-4': [rectHotspot(376, 224, 22, 13, 'rotate(-10 387 230)')],
    'digit-5': [rectHotspot(399, 220, 22, 13, 'rotate(-10 410 226)')],
    'digit-6': [rectHotspot(422, 216, 22, 13, 'rotate(-10 433 222)')],
    'digit-7': [rectHotspot(445, 212, 22, 13, 'rotate(-10 456 218)')],
    'key-q': [rectHotspot(315, 208, 26, 14, 'rotate(-10 328 215)')],
    'key-e': [rectHotspot(360, 200, 26, 14, 'rotate(-10 373 207)')],
    'key-r': [rectHotspot(382, 196, 26, 14, 'rotate(-10 395 203)')],
    space: [rectHotspot(345, 260, 118, 15, 'rotate(-10 404 267)')],
    'key-a': [rectHotspot(322, 183, 25, 14, 'rotate(-10 334 190)')],
    'key-d': [rectHotspot(365, 176, 25, 14, 'rotate(-10 377 183)')],
    'key-s': [rectHotspot(344, 180, 25, 14, 'rotate(-10 356 187)')],
    'key-w': [rectHotspot(336, 204, 25, 14, 'rotate(-10 348 211)')],
  },
  mouseHotspots: {
    'mouse-move': [polygonHotspot('44,126 214,101 242,204 73,229', HOTSPOT_CYAN_FILL, HOTSPOT_CYAN_STROKE)],
    'mouse-left': [polygonHotspot('44,126 124,114 157,217 73,229', HOTSPOT_CYAN_FILL, HOTSPOT_CYAN_STROKE)],
    'mouse-right': [polygonHotspot('124,114 214,101 242,204 157,217', HOTSPOT_CYAN_FILL, HOTSPOT_CYAN_STROKE)],
    'mouse-side': [polygonHotspot('182,106 214,101 242,204 210,209', HOTSPOT_CYAN_FILL, HOTSPOT_CYAN_STROKE)],
  },
};

const STANDARD_MODEL_KEYBOARD_CLIP = polygonHotspot('283,167 554,122 597,289 323,336');
const GAMEPAD_MODEL_LEFT_CLUSTER_CLIP = polygonHotspot('32,147 286,153 286,293 42,292');
const GAMEPAD_MODEL_RIGHT_CLUSTER_CLIP = polygonHotspot('301,156 556,182 566,308 300,305');

const GAMEPAD_MODEL_INTERACTION = {
  keyboardHotspots: {
    'digit-1': [polygonHotspot('333,189 359,202 332,216 306,203')],
    'digit-2': [polygonHotspot('360,202 387,215 360,229 334,216')],
    'digit-3': [polygonHotspot('333,216 360,229 333,244 307,231')],
    'digit-4': [polygonHotspot('307,203 333,216 307,231 281,218')],
    'digit-5': [ellipseHotspot(170, 205, 22, 13)],
    'digit-6': [ellipseHotspot(220, 205, 22, 13)],
    'digit-7': [ellipseHotspot(262, 217, 22, 13)],
    'key-q': [ellipseHotspot(136, 166, 28, 18, HOTSPOT_PINK_FILL, HOTSPOT_PINK_STROKE)],
    'key-e': [ellipseHotspot(474, 244, 29, 20, HOTSPOT_YELLOW_FILL, HOTSPOT_YELLOW_STROKE)],
    'key-r': [polygonHotspot('360,174 388,188 360,202 334,188')],
    space: [rectHotspot(395, 268, 74, 20, '', HOTSPOT_WHITE_FILL, HOTSPOT_WHITE_STROKE)],
    'key-a': [rectHotspot(109, 233, 62, 20, '', HOTSPOT_WHITE_FILL, HOTSPOT_WHITE_STROKE)],
    'key-d': [rectHotspot(180, 266, 60, 18, '', HOTSPOT_WHITE_FILL, HOTSPOT_WHITE_STROKE)],
    'key-s': [ellipseHotspot(196, 205, 22, 13)],
    'key-w': [ellipseHotspot(196, 174, 22, 13)],
  },
  mouseHotspots: {
    'mouse-move': [polygonHotspot('333,189 359,202 332,216 306,203', HOTSPOT_CYAN_FILL, HOTSPOT_CYAN_STROKE)],
    'mouse-left': [ellipseHotspot(136, 166, 28, 18, HOTSPOT_PINK_FILL, HOTSPOT_PINK_STROKE)],
    'mouse-right': [ellipseHotspot(474, 244, 29, 20, HOTSPOT_YELLOW_FILL, HOTSPOT_YELLOW_STROKE)],
    'mouse-side': [polygonHotspot('360,202 387,215 360,229 334,216', HOTSPOT_CYAN_FILL, HOTSPOT_CYAN_STROKE)],
  },
};

export const AVATAR_PRESET_OPTIONS = [
  {
    id: 'original-standard',
    titleKey: 'settingsAppearance.avatarPresetOriginalTitle',
    descriptionKey: 'settingsAppearance.avatarPresetOriginalDesc',
    previewMotionBeat: 18,
    previewInputActivity: {
      keyboardActive: true,
      mouseActive: true,
      keyboardGroup: 'key-q',
      keyboardVisualKey: 'KeyQ',
      mouseGroup: 'mouse-left',
      cursorRatioX: 0.36,
      cursorRatioY: 0.62,
      lastKeyboardInputAtMs: 0,
      lastMouseInputAtMs: 0,
    },
  },
];

const AVAILABLE_AVATAR_PRESET_IDS = new Set(
  AVATAR_PRESET_OPTIONS.map((option) => option.id)
);

const AVATAR_PRESET_REGISTRY = {
  'original-standard': {
    ...STANDARD_BASE,
    id: 'original-standard',
    sceneSrc: mouseBg,
    contentTransform: '',
    keyboardVisualLayers: null,
    mouseVisualLayers: null,
    showKeyboardOverlay: true,
    showMouseDevice: true,
    showMouseArm: true,
    keyboardOverlayOpacity: 1,
    handOpacity: 1,
    mouseDeviceOpacity: 1,
    mouseOverlayOpacity: 0.96,
    mouseArmOpacity: 1,
  },
  'keyboard-focus': {
    ...STANDARD_BASE,
    id: 'keyboard-focus',
    sceneSrc: modelStandardBackground,
    staticCoverSrc: modelStandardCover,
    contentTransform: '',
    keyboardVisualLayers: STANDARD_MODEL_LEFT_KEYS,
    keyboardVisualClip: STANDARD_MODEL_KEYBOARD_CLIP,
    mouseVisualLayers: null,
    showKeyboardOverlay: false,
    showMouseDevice: false,
    showMouseArm: true,
    keyboardOverlayOpacity: 0,
    handOpacity: 0,
    mouseDeviceOpacity: 0,
    mouseOverlayOpacity: 0,
    mouseArmOpacity: 0,
  },
  'minimal-office': {
    ...STANDARD_BASE,
    id: 'minimal-office',
    sceneSrc: modelGamepadBackground,
    staticCoverSrc: modelGamepadCover,
    contentTransform: '',
    keyboardVisualLayers: GAMEPAD_KEYBOARD_LAYER_BY_KEY,
    keyboardVisualClip: GAMEPAD_MODEL_LEFT_CLUSTER_CLIP,
    mouseVisualLayers: GAMEPAD_MOUSE_LAYER_BY_GROUP,
    mouseVisualClip: GAMEPAD_MODEL_RIGHT_CLUSTER_CLIP,
    showKeyboardOverlay: false,
    showMouseDevice: false,
    showMouseArm: false,
    keyboardOverlayOpacity: 0,
    handOpacity: 0,
    mouseDeviceOpacity: 0,
    mouseOverlayOpacity: 0,
    mouseArmOpacity: 0,
  },
};

export function normalizeAvatarPresetId(presetId) {
  return AVAILABLE_AVATAR_PRESET_IDS.has(presetId) ? presetId : AVATAR_PRESET_DEFAULT;
}

export function getAvatarPresetDefinition(presetId) {
  return AVATAR_PRESET_REGISTRY[normalizeAvatarPresetId(presetId)];
}

export function getAvatarPresetOption(presetId) {
  return AVATAR_PRESET_OPTIONS.find((option) => option.id === normalizeAvatarPresetId(presetId))
    ?? AVATAR_PRESET_OPTIONS[0];
}
