import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import { check } from '@tauri-apps/plugin-updater';
import { ask, message as showMessage } from '@tauri-apps/plugin-dialog';
import { relaunch } from '@tauri-apps/plugin-process';
import { showToast } from '$lib/stores/toast.js';

const DEFAULT_RELEASE_URL = 'https://github.com/wm94i/Work_Review/releases/latest';
const UPDATE_CHECK_TIMEOUT_MS = 8000;
const UPDATE_DOWNLOAD_TIMEOUT_MS = 5 * 60 * 1000;

let updateInFlight = false;

async function promptOpenRelease(message, releaseUrl = DEFAULT_RELEASE_URL) {
  const openRelease = await ask(message, { title: '更新提示', kind: 'warning' });
  if (openRelease) {
    await open(releaseUrl);
  }
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
  onStatusChange('正在检查更新...');

  try {
    const releaseInfo = await invoke('check_github_update');

    if (!releaseInfo?.available) {
      onStatusChange(silentWhenUpToDate ? '' : '当前已是最新版本');
      if (!silentWhenUpToDate) {
        showToast('当前已是最新版本', 'success');
      }
      return { updated: false, available: false };
    }

    if (confirmBeforeDownload) {
      const shouldStart = await ask(
        `检测到新版本 ${releaseInfo.latestVersion}。是否现在开始更新？`,
        { title: '发现新版本', kind: 'info' }
      );

      if (!shouldStart) {
        onStatusChange('');
        return { updated: false, cancelled: true };
      }
    }

    if (!releaseInfo.autoUpdateReady) {
      onStatusChange(`发现新版本 ${releaseInfo.latestVersion}，自动更新包暂未就绪`);
      showToast(`发现新版本 ${releaseInfo.latestVersion}，请手动下载`, 'warning');
      await promptOpenRelease(
        `检测到新版本 ${releaseInfo.latestVersion}，但自动更新通道暂未就绪。是否打开 Releases 页面手动下载？`,
        releaseInfo.releaseUrl
      );
      return { updated: false, manualOnly: true };
    }

    const update = await check({ timeout: UPDATE_CHECK_TIMEOUT_MS });

    if (!update) {
      onStatusChange('自动更新通道暂不可用');
      showToast('自动更新通道暂不可用，请手动下载', 'warning');
      await promptOpenRelease(
        '检测到新版本，但原生更新元数据尚未同步完成。是否打开 Releases 页面手动下载？',
        releaseInfo.releaseUrl
      );
      return { updated: false, manualOnly: true };
    }

    onStatusChange(`发现新版本 ${update.version || releaseInfo.latestVersion}，开始下载...`);
    let downloaded = 0;
    let contentLength = 0;

    await update.downloadAndInstall((event) => {
      if (event.event === 'Started') {
        contentLength = event.data.contentLength || 0;
      } else if (event.event === 'Progress') {
        downloaded += event.data.chunkLength;
        if (contentLength > 0) {
          const percent = Math.min(100, Math.round((downloaded / contentLength) * 100));
          onStatusChange(`下载中 (${percent}%)`);
        } else {
          onStatusChange(`下载中 (${Math.max(1, Math.round(downloaded / 1024 / 1024))} MB)`);
        }
      } else if (event.event === 'Finished') {
        onStatusChange('下载完成，正在安装...');
      }
    }, {
      timeout: UPDATE_DOWNLOAD_TIMEOUT_MS,
    });

    await update.close();
    onStatusChange('安装完成，正在重启...');
    await relaunch();
    return { updated: true };
  } catch (error) {
    const errMsg = String(error);
    console.error('检查更新失败:', error);

    if (errMsg.includes('timeout') || errMsg.includes('timed out')) {
      onStatusChange('检查更新超时');
      showToast('检查更新超时', 'error');
      await promptOpenRelease('检查更新超时。是否打开 Releases 页面手动下载最新版本？');
    } else if (
      errMsg.includes('Download request failed') ||
      errMsg.includes('failed to download') ||
      errMsg.includes('Network')
    ) {
      onStatusChange('下载更新失败');
      showToast('下载更新失败', 'error');
      await promptOpenRelease('更新包下载失败。是否打开 Releases 页面手动下载最新版本？');
    } else {
      onStatusChange('检查更新失败');
      await showMessage(`检查更新出现问题: ${errMsg}`, { title: '错误', kind: 'error' });
    }

    return { updated: false, error: errMsg };
  } finally {
    updateInFlight = false;
  }
}
