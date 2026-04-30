import { get, writable } from 'svelte/store';
import zhCN from './locales/zh-CN.js';
import en from './locales/en.js';
import zhTW from './locales/zh-TW.js';

const LOCALE_STORAGE_KEY = 'work-review.locale';
const DEFAULT_LOCALE = 'zh-CN';

export const SUPPORTED_LOCALES = ['zh-CN', 'en', 'zh-TW'];
const LOCALE_CYCLE = ['zh-CN', 'en', 'zh-TW'];

const LOCALE_META = {
  'zh-CN': {
    short: 'ZH',
    label: '简体中文',
  },
  en: {
    short: 'EN',
    label: 'English',
  },
  'zh-TW': {
    short: 'TW',
    label: '繁體中文',
  },
};

const CATEGORY_LABELS = {
  development: {
    'zh-CN': '开发工具',
    en: 'Development',
    'zh-TW': '開發工具',
  },
  browser: {
    'zh-CN': '浏览器',
    en: 'Browser',
    'zh-TW': '瀏覽器',
  },
  communication: {
    'zh-CN': '通讯协作',
    en: 'Communication',
    'zh-TW': '通訊協作',
  },
  office: {
    'zh-CN': '办公软件',
    en: 'Office',
    'zh-TW': '辦公軟體',
  },
  design: {
    'zh-CN': '设计工具',
    en: 'Design',
    'zh-TW': '設計工具',
  },
  entertainment: {
    'zh-CN': '娱乐摸鱼',
    en: 'Leisure',
    'zh-TW': '娛樂摸魚',
  },
  other: {
    'zh-CN': '其他',
    en: 'Other',
    'zh-TW': '其他',
  },
};

const SEMANTIC_LABELS = {
  '编码开发': {
    'zh-CN': '编码开发',
    en: 'Development',
    'zh-TW': '編碼開發',
  },
  '内容撰写': {
    'zh-CN': '内容撰写',
    en: 'Writing',
    'zh-TW': '內容撰寫',
  },
  '资料阅读': {
    'zh-CN': '资料阅读',
    en: 'Reading',
    'zh-TW': '資料閱讀',
  },
  '资料调研': {
    'zh-CN': '资料调研',
    en: 'Research',
    'zh-TW': '資料調研',
  },
  '任务规划': {
    'zh-CN': '任务规划',
    en: 'Planning',
    'zh-TW': '任務規劃',
  },
  '设计创作': {
    'zh-CN': '设计创作',
    en: 'Design',
    'zh-TW': '設計創作',
  },
  'AI 协作': {
    'zh-CN': 'AI 协作',
    en: 'AI Collaboration',
    'zh-TW': 'AI 協作',
  },
  '即时聊天': {
    'zh-CN': '即时聊天',
    en: 'Chat',
    'zh-TW': '即時聊天',
  },
  '会议沟通': {
    'zh-CN': '会议沟通',
    en: 'Meetings',
    'zh-TW': '會議溝通',
  },
  '视频内容': {
    'zh-CN': '视频内容',
    en: 'Video',
    'zh-TW': '影片內容',
  },
  '音乐音频': {
    'zh-CN': '音乐音频',
    en: 'Audio',
    'zh-TW': '音樂音訊',
  },
  '休息娱乐': {
    'zh-CN': '休息娱乐',
    en: 'Leisure',
    'zh-TW': '休息娛樂',
  },
  '未知活动': {
    'zh-CN': '未知活动',
    en: 'Unknown',
    'zh-TW': '未知活動',
  },
};


const MESSAGES = {
  'zh-CN': zhCN,
  en,
  'zh-TW': zhTW,
};


export const locale = writable(DEFAULT_LOCALE);

function normalizeLocale(value) {
  if (!value) {
    return DEFAULT_LOCALE;
  }

  const normalized = value.trim();
  if (SUPPORTED_LOCALES.includes(normalized)) {
    return normalized;
  }

  if (normalized.toLowerCase().startsWith('zh-tw') || normalized.toLowerCase().startsWith('zh-hk')) {
    return 'zh-TW';
  }

  if (normalized.toLowerCase().startsWith('zh')) {
    return 'zh-CN';
  }

  if (normalized.toLowerCase().startsWith('en')) {
    return 'en';
  }

  return DEFAULT_LOCALE;
}

function getStoredLocale() {
  if (typeof window === 'undefined') {
    return null;
  }

  try {
    return window.localStorage.getItem(LOCALE_STORAGE_KEY);
  } catch {
    return null;
  }
}

function persistLocale(nextLocale) {
  if (typeof window === 'undefined') {
    return;
  }

  try {
    window.localStorage.setItem(LOCALE_STORAGE_KEY, nextLocale);
  } catch {
    // ignore persistence errors
  }
}

function detectBrowserLocale() {
  if (typeof navigator === 'undefined') {
    return DEFAULT_LOCALE;
  }

  for (const candidate of navigator.languages || [navigator.language]) {
    const normalized = normalizeLocale(candidate);
    if (SUPPORTED_LOCALES.includes(normalized)) {
      return normalized;
    }
  }

  return DEFAULT_LOCALE;
}

export function initializeLocale(preferredLocale) {
  const nextLocale = normalizeLocale(preferredLocale || getStoredLocale() || detectBrowserLocale());
  locale.set(nextLocale);
  persistLocale(nextLocale);
  return nextLocale;
}

export function setLocale(nextLocale) {
  const normalized = normalizeLocale(nextLocale);
  locale.set(normalized);
  persistLocale(normalized);
  return normalized;
}

export function cycleLocale() {
  const currentLocale = get(locale);
  const currentIndex = LOCALE_CYCLE.indexOf(currentLocale);
  const nextLocale = LOCALE_CYCLE[(currentIndex + 1 + LOCALE_CYCLE.length) % LOCALE_CYCLE.length];
  return setLocale(nextLocale);
}

function resolveKey(object, key) {
  return key.split('.').reduce((current, segment) => current?.[segment], object);
}

function resolveMessageValue(key) {
  const currentLocale = get(locale);
  return (
    resolveKey(MESSAGES[currentLocale], key) ??
    resolveKey(MESSAGES[DEFAULT_LOCALE], key)
  );
}

function interpolate(template, params) {
  return Object.entries(params).reduce(
    (output, [paramKey, paramValue]) => output.replaceAll(`{${paramKey}}`, String(paramValue)),
    template,
  );
}

export function t(key, params = {}) {
  const rawValue = resolveMessageValue(key) ?? key;

  if (typeof rawValue !== 'string') {
    return key;
  }

  return interpolate(rawValue, params);
}

export function tm(key) {
  return resolveMessageValue(key);
}

export function getLocaleShortLabel(localeCode = get(locale)) {
  return LOCALE_META[normalizeLocale(localeCode)]?.short || LOCALE_META[DEFAULT_LOCALE].short;
}

export function getLocaleLabel(localeCode = get(locale)) {
  return LOCALE_META[normalizeLocale(localeCode)]?.label || LOCALE_META[DEFAULT_LOCALE].label;
}

export function applyLocaleToDocument(nextLocale = get(locale)) {
  if (typeof document === 'undefined') {
    return;
  }

  document.documentElement.lang = normalizeLocale(nextLocale);
  document.documentElement.dir = 'ltr';
}

export function formatLocalizedDate(date, options) {
  return new Intl.DateTimeFormat(get(locale), options).format(date);
}

export function formatLocalizedTime(date, options) {
  return new Intl.DateTimeFormat(get(locale), options).format(date);
}

export function formatDurationLocalized(seconds, { compact = false } = {}) {
  const currentLocale = get(locale);
  const hourUnit = currentLocale === 'zh-TW' ? (compact ? '時' : '小時') : (compact ? 'h' : '小时');
  const minuteUnit = currentLocale === 'zh-TW' ? (compact ? '分' : '分鐘') : (compact ? 'm' : '分钟');
  const secondUnit = currentLocale === 'zh-TW' ? '秒' : '秒';

  if (!seconds || seconds <= 0) {
    if (currentLocale === 'en') {
      return compact ? '0m' : '0s';
    }
    return `0${minuteUnit}`;
  }

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (currentLocale === 'en') {
    if (hours > 0) {
      return compact ? (minutes > 0 ? `${hours}h${minutes}m` : `${hours}h`) : (minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`);
    }
    if (minutes > 0) {
      return compact ? `${minutes}m` : `${minutes}m`;
    }
    return compact ? `${secs}s` : `${secs}s`;
  }

  if (hours > 0) {
    return minutes > 0 ? `${hours}${hourUnit}${minutes}${minuteUnit}` : `${hours}${hourUnit}`;
  }

  if (minutes > 0) {
    return `${minutes}${minuteUnit}`;
  }

  return `${secs}${secondUnit}`;
}

export function translateCategoryLabel(categoryKey) {
  const currentLocale = get(locale);
  return CATEGORY_LABELS[categoryKey]?.[currentLocale] || CATEGORY_LABELS[categoryKey]?.[DEFAULT_LOCALE] || categoryKey;
}

export function translateSemanticCategoryLabel(label) {
  const currentLocale = get(locale);
  return SEMANTIC_LABELS[label]?.[currentLocale] || SEMANTIC_LABELS[label]?.[DEFAULT_LOCALE] || label;
}
