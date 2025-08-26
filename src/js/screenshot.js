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

// 启动原生截屏
export async function startNativeScreenshot() {
  try {
    console.log('启动原生截屏...');
    await invoke('start_native_screenshot');
  } catch (error) {
    console.error('原生截屏失败:', error);
    showNotification(`原生截屏失败: ${error}`, 'error');
  }
}
