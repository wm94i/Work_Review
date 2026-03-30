import { writable } from 'svelte/store';
import { t } from '$lib/i18n/index.js';

const { subscribe, set } = writable(null);

let currentId = 0;
let activeResolver = null;

export const confirmDialog = {
  subscribe,
};

function closeWith(result) {
  const resolver = activeResolver;
  activeResolver = null;
  set(null);
  resolver?.(result);
}

export function confirm(options = {}) {
  if (activeResolver) {
    closeWith(false);
  }

  currentId += 1;

  const state = {
    id: currentId,
    title: options.title?.trim() || t('common.notice'),
    message: options.message?.trim() || '',
    confirmText: options.confirmText?.trim() || t('common.confirm'),
    cancelText: options.cancelText?.trim() || t('common.cancel'),
    tone: options.tone || 'info',
  };

  set(state);

  return new Promise((resolve) => {
    activeResolver = resolve;
  });
}

export function resolveConfirm(result) {
  closeWith(result);
}
