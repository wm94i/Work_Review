// 应用图标全局缓存（模块级单例，跨页面导航不丢失）
// 图标通过 Tauri 后端获取并转为 base64，此缓存避免重复 invoke 调用
import { writable } from 'svelte/store';

// 模块级缓存对象，不随组件销毁而丢失
const _iconCache = {};
const _pendingRequests = {};
const _cacheKeys = [];
const _requestQueue = [];
const MAX_ICON_CACHE = 120;
const MAX_PERSISTED_ICON_CACHE = 36;
const MAX_CONCURRENT_ICON_REQUESTS = 3;
const FAILED_ICON_RETRY_MS = 30 * 1000;
const STORAGE_KEY = 'work-review-app-icon-cache-v1';
const _failedAt = {};
let _activeRequestCount = 0;

function normalizeIconRequest(entry) {
    if (!entry) return { appName: '', executablePath: '' };
    if (typeof entry === 'string') {
        return { appName: entry, executablePath: '' };
    }
    return {
        appName: entry.appName || entry.app_name || entry.browserName || entry.browser_name || '',
        executablePath: entry.executablePath || entry.executable_path || '',
    };
}

export function getIconCacheKey(entry) {
    const { appName, executablePath } = normalizeIconRequest(entry);
    return executablePath ? `${appName}::${executablePath}` : appName;
}

function loadPersistentIconCache() {
    if (typeof window === 'undefined') {
        return;
    }

    try {
        const raw = window.localStorage.getItem(STORAGE_KEY);
        if (!raw) {
            return;
        }

        const parsed = JSON.parse(raw);
        const items = Array.isArray(parsed?.items) ? parsed.items : [];
        for (const item of items) {
            if (!item || typeof item.key !== 'string' || typeof item.value !== 'string') {
                continue;
            }
            if (item.value.length <= 100 || _iconCache[item.key] !== undefined) {
                continue;
            }

            _iconCache[item.key] = item.value;
            _cacheKeys.push(item.key);
        }
    } catch (error) {
        console.warn('加载应用图标缓存失败:', error);
    }
}

function touchCacheKey(cacheKey) {
    const index = _cacheKeys.indexOf(cacheKey);
    if (index >= 0) {
        _cacheKeys.splice(index, 1);
    }
    _cacheKeys.push(cacheKey);
}

function pruneCache() {
    while (_cacheKeys.length > MAX_ICON_CACHE) {
        const oldest = _cacheKeys.shift();
        delete _iconCache[oldest];
        delete _pendingRequests[oldest];
        delete _failedAt[oldest];
    }
}

function persistIconCache() {
    if (typeof window === 'undefined') {
        return;
    }

    try {
        const items = _cacheKeys
            .slice(-MAX_PERSISTED_ICON_CACHE)
            .map((key) => ({ key, value: _iconCache[key] }))
            .filter((item) => typeof item.value === 'string' && item.value.length > 100);
        window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ items }));
    } catch (error) {
        console.warn('保存应用图标缓存失败:', error);
    }
}

loadPersistentIconCache();
pruneCache();

// 响应式 store，通知 Svelte 更新 UI
export const appIconStore = writable({ ..._iconCache });

let _storeDirty = false;
let _storeFlushTimer = null;

function updateIconStore() {
    if (_storeFlushTimer) return;
    _storeFlushTimer = setTimeout(() => {
        _storeFlushTimer = null;
        appIconStore.set({ ..._iconCache });
    }, 100);
}

function runNextIconRequest() {
    while (_activeRequestCount < MAX_CONCURRENT_ICON_REQUESTS && _requestQueue.length > 0) {
        const next = _requestQueue.shift();
        if (!next) {
            return;
        }

        _activeRequestCount += 1;

        void (async () => {
            const { cacheKey, appName, executablePath, invoke } = next;

            try {
                const base64 = await invoke('get_app_icon', {
                    appName,
                    executablePath: executablePath || null,
                });

                if (base64 && base64.length > 100) {
                    _iconCache[cacheKey] = base64;
                    delete _failedAt[cacheKey];
                } else {
                    _iconCache[cacheKey] = null;
                    _failedAt[cacheKey] = Date.now();
                }

                touchCacheKey(cacheKey);
                pruneCache();
                persistIconCache();
            } catch {
                _iconCache[cacheKey] = null;
                _failedAt[cacheKey] = Date.now();
                touchCacheKey(cacheKey);
                pruneCache();
                persistIconCache();
            } finally {
                delete _pendingRequests[cacheKey];
                _activeRequestCount -= 1;
                updateIconStore();
                runNextIconRequest();
            }
        })();
    }
}

// 加载指定应用的图标
export function loadAppIcon(entry, invoke, options = {}) {
    const { appName, executablePath } = normalizeIconRequest(entry);
    if (!appName) return;
    const cacheKey = getIconCacheKey({ appName, executablePath });

    // 成功缓存直接复用；失败缓存仅在冷却期内跳过重试
    if (_iconCache[cacheKey] !== undefined) {
        if (_iconCache[cacheKey] !== null) {
            touchCacheKey(cacheKey);
            return;
        }

        const lastFailedAt = _failedAt[cacheKey] || 0;
        if (Date.now() - lastFailedAt < FAILED_ICON_RETRY_MS) {
            return;
        }
    }

    // 避免同一应用重复排队或并发请求
    if (_pendingRequests[cacheKey]) return;
    _pendingRequests[cacheKey] = true;

    const queueItem = { cacheKey, appName, executablePath, invoke };
    if (options.priority) {
        _requestQueue.unshift(queueItem);
    } else {
        _requestQueue.push(queueItem);
    }

    runNextIconRequest();
}

// 批量预加载
export function preloadAppIcons(entries, invoke, options = {}) {
    const normalizedEntries = Array.isArray(entries) ? entries.filter(Boolean) : [];
    const queueEntries = options.priority ? normalizedEntries.slice().reverse() : normalizedEntries;

    queueEntries.forEach((entry) => loadAppIcon(entry, invoke, options));
}

// 获取已缓存的图标（同步）
export function getIcon(entry) {
    return _iconCache[getIconCacheKey(entry)] || null;
}
