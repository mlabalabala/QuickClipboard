import { invoke } from '@tauri-apps/api/core';
import { screenshotButton } from './config.js';
import { showNotification } from './ui.js';

// 初始化截屏功能
export function initScreenshot() {
  if (screenshotButton) {
    screenshotButton.addEventListener('click', openScreenshotWindow);
  }
}

// 打开截屏窗口
export async function openScreenshotWindow() {
  try {
    await invoke('open_screenshot_window');
  } catch (error) {
    console.error('打开截屏窗口失败:', error);
    showNotification('打开截屏窗口失败', 'error');
  }
}

// 关闭截屏窗口
export async function closeScreenshotWindow() {
  try {
    await invoke('close_screenshot_window');
  } catch (error) {
    console.error('关闭截屏窗口失败:', error);
  }
}

// 区域截屏
export async function takeScreenshot(x, y, width, height) {
  try {
    await invoke('take_screenshot', { x, y, width, height });
    showNotification('截屏已保存到剪贴板', 'success', 2000);
  } catch (error) {
    console.error('截屏失败:', error);
    showNotification('截屏失败', 'error');
  }
}

// 全屏截屏
export async function takeFullscreenScreenshot() {
  try {
    await invoke('take_fullscreen_screenshot');
    showNotification('全屏截屏已保存到剪贴板', 'success', 2000);
  } catch (error) {
    console.error('全屏截屏失败:', error);
    showNotification('全屏截屏失败', 'error');
  }
}
