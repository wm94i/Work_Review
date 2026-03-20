import { writable } from 'svelte/store';

const { subscribe, set } = writable(null);

let toastId = 0;
let hideTimer = null;

export const toast = {
  subscribe,
};

export function showToast(message, type = 'info', duration = 3000) {
  toastId += 1;
  const currentId = toastId;

  set({
    id: currentId,
    message,
    type,
  });

  if (hideTimer) {
    clearTimeout(hideTimer);
  }

  hideTimer = setTimeout(() => {
    set((current) => (current?.id === currentId ? null : current));
    hideTimer = null;
  }, duration);
}

export function clearToast() {
  if (hideTimer) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }

  set(null);
}
