import { invoke } from '@tauri-apps/api/core';
import { showNotification } from './notificationManager.js';

// 初始化截屏功能
export function initScreenshot() {
  document.addEventListener('click', (e) => {
    const screenshotBtn = e.target.closest('#screenshot-button');
    if (screenshotBtn) {
      e.preventDefault();
      e.stopPropagation();
      startNativeScreenshot();
    }
  }, true);
}

// 启动外部截屏程序
export async function startNativeScreenshot() {
  try {
    console.log('启动外部截屏程序...');
    await invoke('launch_external_screenshot');
  } catch (error) {
    console.error('启动外部截屏程序失败:', error);
    showNotification(`启动外部截屏程序失败: ${error}`, 'error');
  }
}
