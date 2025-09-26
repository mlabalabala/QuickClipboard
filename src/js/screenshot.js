import { invoke } from '@tauri-apps/api/core';
import { showNotification } from './notificationManager.js';

// 启动内置截屏窗口
export async function startBuiltinScreenshot() {
  try {
    console.log('启动内置截屏窗口...');
    await invoke('start_builtin_screenshot');
  } catch (error) {
    console.error('启动内置截屏窗口失败:', error);
    showNotification(`启动内置截屏窗口失败: ${error}`, 'error');
  }
}

// 启动外部截屏程序
export async function startNativeScreenshot() {
  try {
    console.log('启动外部截屏程序...');
    setTimeout(async () => {
      // 前端调用时隐藏窗口
      await invoke('launch_external_screenshot', { hideWindow: true });
    }, 600);
  } catch (error) {
    console.error('启动外部截屏程序失败:', error);
    showNotification(`启动外部截屏程序失败: ${error}`, 'error');
  }
}

// 默认使用内置截屏
export async function startScreenshot() {
  await startBuiltinScreenshot();
}
