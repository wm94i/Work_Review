import { writable } from 'svelte/store';

export const BASIC_ASSISTANT_MODEL_ID = '__basic__';

const STORAGE_KEY = 'work-review-assistant-state';
const DEFAULT_STATE = {
  messages: [],
  selectedModelId: BASIC_ASSISTANT_MODEL_ID,
};

function normalizeMessage(message) {
  return {
    ...message,
    cards: Array.isArray(message?.cards) ? message.cards : [],
    references: Array.isArray(message?.references) ? message.references : [],
    toolLabels: Array.isArray(message?.toolLabels) ? message.toolLabels : [],
  };
}

function loadState() {
  if (typeof window === 'undefined') {
    return DEFAULT_STATE;
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return DEFAULT_STATE;
    }

    const parsed = JSON.parse(raw);
    return {
      messages: Array.isArray(parsed?.messages)
        ? parsed.messages.map((message) => normalizeMessage(message))
        : [],
      selectedModelId:
        typeof parsed?.selectedModelId === 'string' && parsed.selectedModelId.trim()
          ? parsed.selectedModelId
          : BASIC_ASSISTANT_MODEL_ID,
    };
  } catch (error) {
    console.warn('加载助手会话缓存失败:', error);
    return DEFAULT_STATE;
  }
}

function persistState(state) {
  if (typeof window === 'undefined') {
    return;
  }

  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch (error) {
    console.warn('保存助手会话缓存失败:', error);
  }
}

function createAssistantStore() {
  const { subscribe, set, update } = writable(loadState());

  let _persistTimer = null;
  subscribe((state) => {
    if (_persistTimer) clearTimeout(_persistTimer);
    _persistTimer = setTimeout(() => {
      _persistTimer = null;
      persistState(state);
    }, 500);
  });

  return {
    subscribe,
    appendMessage: (message) =>
      update((state) => ({
        ...state,
        messages: [...state.messages, normalizeMessage(message)].slice(-40),
      })),
    clearMessages: () =>
      update((state) => ({
        ...state,
        messages: [],
      })),
    setSelectedModelId: (selectedModelId) =>
      update((state) => ({
        ...state,
        selectedModelId:
          typeof selectedModelId === 'string' && selectedModelId.trim()
            ? selectedModelId
            : BASIC_ASSISTANT_MODEL_ID,
      })),
    setMessages: (messages) =>
      update((state) => ({
        ...state,
        messages: Array.isArray(messages)
          ? messages.slice(-40).map((message) => normalizeMessage(message))
          : [],
      })),
    reset: () => set(DEFAULT_STATE),
  };
}

export const assistantStore = createAssistantStore();
