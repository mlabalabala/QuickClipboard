/**
 * 更新器核心类
 */

import { showNotification } from '../js/notificationManager.js';
import { fetchLatestRelease, getCurrentVersion, checkPortableMode } from './updater-api.js';
import { compareVersions } from './updater-utils.js';
import { BadgeManager, UpdateDialog } from './updater-ui.js';
import { downloadAndInstall } from './updater-installer.js';

export class Updater {
  constructor() {
    this.checking = false;
    this.downloading = false;
    this.currentUpdate = null;
    this.isPortable = false;
    this.portableChecked = false;
    this.latestReleaseCache = null;
  }

  /**
   * 检查是否为便携版模式
   */
  async checkPortableModeInternal() {
    if (!this.portableChecked) {
      this.isPortable = await checkPortableMode();
      this.portableChecked = true;
    }
    return this.isPortable;
  }

  /**
   * 快速检查更新
   */
  async checkForUpdates(silent = false, triggerButton = null) {
    const isPortable = await this.checkPortableModeInternal();
    if (isPortable) {
      if (!silent) {
        showNotification('便携版模式下已禁用自动更新功能', 'info');
      }
      return null;
    }

    if (this.checking) {
      return null;
    }

    this.checking = true;

    let originalHTML = '';
    if (triggerButton) {
      originalHTML = triggerButton.innerHTML;
      triggerButton.disabled = true;
      triggerButton.style.opacity = '0.6';
      triggerButton.style.cursor = 'not-allowed';
      triggerButton.innerHTML = '<i class="ti ti-loader-2" style="animation: spin 1s linear infinite; margin-right: 6px;"></i>检查中...';
    }

    try {
      const latestRelease = await fetchLatestRelease();
      this.latestReleaseCache = latestRelease;
      
      const currentVersion = await getCurrentVersion();
      const hasUpdate = compareVersions(latestRelease.version, currentVersion) > 0;

      if (hasUpdate) {
        this.currentUpdate = {
          available: true,
          version: latestRelease.version,
          currentVersion: currentVersion,
          date: latestRelease.date,
          body: latestRelease.body,
          name: latestRelease.name,
          htmlUrl: latestRelease.htmlUrl
        };

        BadgeManager.show();

        window.dispatchEvent(new CustomEvent('app-update-available', {
          detail: this.currentUpdate
        }));

        if (!silent) {
          this.showUpdateDialog();
        }

        return this.currentUpdate;
      } else {
        this.currentUpdate = null;
        BadgeManager.hide();

        if (!silent) {
          showNotification('当前使用的已是最新版本', 'success');
        }

        return null;
      }
    } catch (error) {
      if (!silent) {
        showNotification('无法检查更新，请稍后重试', 'error');
      }
      return null;
    } finally {
      this.checking = false;
      
      if (triggerButton) {
        triggerButton.disabled = false;
        triggerButton.style.opacity = '';
        triggerButton.style.cursor = '';
        triggerButton.innerHTML = originalHTML;
      }
    }
  }

  /**
   * 显示更新对话框
   */
  showUpdateDialog() {
    if (!this.currentUpdate) return;

    const dialog = new UpdateDialog(
      this.currentUpdate,
      this.latestReleaseCache,
      (elements) => this.handleInstall(elements)
    );
    
    dialog.show();
  }

  /**
   * 处理安装
   */
  async handleInstall(elements) {
    if (this.downloading) return;

    this.downloading = true;
    
    try {
      await downloadAndInstall(elements);
    } catch (error) {
    } finally {
      this.downloading = false;
    }
  }

  /**
   * 启动时自动检查更新
   */
  initAutoUpdate() {
    setTimeout(async () => {
      const isPortable = await this.checkPortableModeInternal();
      if (!isPortable) {
        await this.checkForUpdates(true);
      }
    }, 3000);
  }
}

