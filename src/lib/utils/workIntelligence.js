import { get } from 'svelte/store';
import { formatDurationLocalized, locale } from '../i18n/index.js';

export function todayString() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
}

export function daysAgoString(days) {
  const date = new Date();
  date.setDate(date.getDate() - days);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

export function normalizedDate(value) {
  return value && value.trim() ? value : null;
}

export function formatDuration(seconds) {
  return formatDurationLocalized(seconds);
}

export function formatDateTime(timestamp) {
  if (!timestamp) return '';
  return new Intl.DateTimeFormat(get(locale), {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(timestamp * 1000));
}
