import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { relaunch } from '@tauri-apps/plugin-process';
import { open } from '@tauri-apps/plugin-shell';
import { confirm } from '$lib/stores/confirm.js';
import { showToast } from '$lib/stores/toast.js';
import { t } from '$lib/i18n/index.js';

const UPDATE_STATUS_EVENT = 'update-status';

let updateInFlight = false;
let runtimePlatformPromise = null;

function localizeRuntimeStatusMessage(message) {
  const text = String(message || '').trim();
  if (!text) {
    return '';
  }

  let matched = text.match(/^正在检查更新源\s+(.+)\.\.\.$/);
  if (matched) {
    return t('updater.checkingSource', { sourceLabel: matched[1] });
  }

  matched = text.match(/^发现新版本\s+(.+?)，准备从\s+(.+)\s+下载\.\.\.$/);
  if (matched) {
    return t('updater.preparingDownload', {
      version: matched[1],
      sourceLabel: matched[2],
    });
  }

  matched = text.match(/^更新安装完成，来源\s+(.+)$/);
  if (matched) {
    return t('updater.installedFromSource', { sourceLabel: matched[1] });
  }

  matched = text.match(/^未找到可用于版本\s+(.+)\s+的在线更新源$/);
  if (matched) {
    return t('updater.noSourceForVersion', { version: matched[1] });
  }

  matched = text.match(/^在线更新失败，已尝试全部更新源：(.+)$/);
  if (matched) {
    return t('updater.failedWithDetails', { details: matched[1] });
  }

  if (text === '在线更新已完成') {
    return t('updater.completed');
  }

  if (text === '当前未发现可安装的在线更新') {
    return t('updater.noInstallAvailable');
  }

  return text;
}

async function getRuntimePlatform() {
  if (!runtimePlatformPromise) {
    runtimePlatformPromise = invoke('get_runtime_platform').catch((error) => {
      runtimePlatformPromise = null;
      throw error;
    });
  }

  return runtimePlatformPromise;
}

export async function runUpdateFlow(options = {}) {
  const {
    silentWhenUpToDate = false,
    confirmBeforeDownload = false,
    onStatusChange = () => {},
  } = options;

  if (updateInFlight) {
    return { skipped: true, reason: 'in-flight' };
  }

  updateInFlight = true;
  onStatusChange(t('updater.checking'));

  try {
    const releaseInfo = await invoke('check_github_update');
    await invoke('update_last_check_time').catch((error) => {
      console.warn('记录更新检查时间失败:', error);
    });

    if (!releaseInfo?.available) {
      onStatusChange(silentWhenUpToDate ? '' : t('updater.upToDate'));
      if (!silentWhenUpToDate) {
        showToast(t('updater.upToDate'), 'success');
      }
      return { updated: false, available: false };
    }

    if (!releaseInfo.autoUpdateReady) {
      onStatusChange(t('updater.availableManual'));
      showToast(t('updater.availableManual'), 'info', 4500);

      if (confirmBeforeDownload && releaseInfo.releaseUrl) {
        const shouldOpenRelease = await confirm({
          title: t('updater.newVersionTitle'),
          message: t('updater.openReleaseMessage', { version: releaseInfo.latestVersion }),
          confirmText: t('updater.openRelease'),
          cancelText: t('updater.later'),
          tone: 'info',
        });

        if (shouldOpenRelease) {
          await open(releaseInfo.releaseUrl);
        }
      }

      return {
        updated: false,
        available: true,
        autoUpdateReady: false,
        releaseUrl: releaseInfo.releaseUrl,
      };
    }

    if (confirmBeforeDownload) {
      const shouldStart = await confirm({
        title: t('updater.newVersionTitle'),
        message: t('updater.startUpdateMessage', { version: releaseInfo.latestVersion }),
        confirmText: t('updater.startUpdate'),
        cancelText: t('updater.later'),
        tone: 'info',
      });

      if (!shouldStart) {
        onStatusChange('');
        return { updated: false, cancelled: true };
      }
    }

    const unlistenUpdateStatus = await listen(UPDATE_STATUS_EVENT, (event) => {
      const payload = event.payload || {};
      if (payload.message) {
        onStatusChange(localizeRuntimeStatusMessage(payload.message));
      }
    });

    try {
      await invoke('download_and_install_github_update', {
        expectedVersion: releaseInfo.latestVersion,
      });
    } finally {
      await unlistenUpdateStatus();
    }

    const runtimePlatform = await getRuntimePlatform();
    if (runtimePlatform === 'windows') {
      onStatusChange(t('updater.installerStartedStatus'));
      showToast(t('updater.installerStartedToast'), 'success');
      await invoke('quit_app_for_update');
      return { updated: true, handoffToInstaller: true };
    }

    onStatusChange(t('updater.restarting'));
    await relaunch();
    return { updated: true };
  } catch (error) {
    const errMsg = String(error);
    console.error('检查更新失败:', error);

    if (errMsg.includes('timeout') || errMsg.includes('timed out')) {
      onStatusChange(t('updater.failed'));
      showToast(t('updater.timeout'), 'error');
    } else if (
      errMsg.includes('Download request failed') ||
      errMsg.includes('failed to download') ||
      errMsg.includes('Network')
    ) {
      onStatusChange(t('updater.failed'));
      showToast(t('updater.failedAllSources'), 'error');
    } else {
      onStatusChange(t('updater.failed'));
      showToast(t('updater.failed'), 'error');
    }

    await confirm({
      title: t('updater.errorTitle'),
      message: t('updater.errorMessage', { error: errMsg }),
      confirmText: t('updater.acknowledge'),
      cancelText: t('updater.retryLater'),
      tone: 'error',
    });

    return { updated: false, error: errMsg };
  } finally {
    updateInFlight = false;
  }
}
