/**
 * 更新器安装模块
 */

import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { showNotification } from '../js/notificationManager.js';

/**
 * 下载并安装更新
 */
export async function downloadAndInstall(elements) {
  const { progressContainer, progressFill, progressText, updateBtn, laterBtn, closeBtn } = elements;

  try {
    updateBtn.disabled = true;
    laterBtn.disabled = true;
    closeBtn.disabled = true;
    updateBtn.style.opacity = '0.5';
    updateBtn.style.cursor = 'not-allowed';
    laterBtn.style.opacity = '0.5';
    laterBtn.style.cursor = 'not-allowed';
    closeBtn.style.opacity = '0.5';
    closeBtn.style.cursor = 'not-allowed';

    progressContainer.style.display = 'block';
    progressText.innerHTML = '<i class="ti ti-loader-2" style="animation: spin 1s linear infinite;"></i> 正在连接更新服务器...';

    const update = await check();

    if (!update || !update.available) {
      throw new Error('无法连接到更新服务器或未找到更新');
    }
    
    let contentLength = 0;
    let downloadedLength = 0;
    let lastUpdateTime = Date.now();
    let lastDownloadedLength = 0;

    await update.downloadAndInstall((event) => {
      const now = Date.now();
      
      switch (event.event) {
        case 'Started':
          contentLength = event.data.contentLength || 0;
          const sizeMB = (contentLength / 1024 / 1024).toFixed(2);
          progressText.innerHTML = `<i class="ti ti-loader-2" style="animation: spin 1s linear infinite;"></i> 开始下载... (${sizeMB} MB)`;
          progressFill.style.width = '0%';
          lastUpdateTime = now;
          lastDownloadedLength = 0;
          break;

        case 'Progress':
          downloadedLength += event.data.chunkLength || 0;

          if (contentLength > 0 && (now - lastUpdateTime > 100 || downloadedLength === contentLength)) {
            const percent = Math.round((downloadedLength / contentLength) * 100);
            const downloadedMB = (downloadedLength / 1024 / 1024).toFixed(2);
            const totalMB = (contentLength / 1024 / 1024).toFixed(2);

            const timeDiff = (now - lastUpdateTime) / 1000; 
            const dataDiff = downloadedLength - lastDownloadedLength;
            const speedKB = (dataDiff / 1024 / timeDiff).toFixed(0);
            
            progressFill.style.width = `${percent}%`;
            progressText.innerHTML = `<i class="ti ti-download" style="animation: pulse 1s ease infinite;"></i> 下载中 ${percent}% (${downloadedMB}/${totalMB} MB) · ${speedKB} KB/s`;
            
            lastUpdateTime = now;
            lastDownloadedLength = downloadedLength;
          }
          break;

        case 'Finished':
          progressFill.style.width = '100%';
          progressText.innerHTML = '<i class="ti ti-check"></i> 下载完成，正在安装...';
          break;
      }
    });

    progressText.innerHTML = '<i class="ti ti-refresh"></i> 安装完成！正在重启应用...';
    progressText.style.color = 'var(--success-color, #52c41a)';

    setTimeout(async () => {
      try {
        await relaunch();
      } catch {
        showNotification('请手动重启应用以完成更新', 'error');
      }
    }, 1000);

  } catch (error) {
    // 恢复按钮
    updateBtn.disabled = false;
    laterBtn.disabled = false;
    closeBtn.disabled = false;
    updateBtn.style.opacity = '1';
    updateBtn.style.cursor = 'pointer';
    laterBtn.style.opacity = '1';
    laterBtn.style.cursor = 'pointer';
    closeBtn.style.opacity = '1';
    closeBtn.style.cursor = 'pointer';

    progressText.innerHTML = '<i class="ti ti-alert-circle"></i> 更新失败: ' + (error.message || '未知错误');
    progressText.style.color = 'var(--error-color, #ff4d4f)';

    showNotification(
      error.message || '下载或安装更新失败，请稍后重试',
      'error'
    );
    
    throw error;
  }
}

