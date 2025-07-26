import { invoke } from '@tauri-apps/api/core';
import {
  isPinned,
  setIsPinned,
  pinButton
} from './config.js';

// 切换窗口固定状态（控制粘贴后是否隐藏窗口）
export async function togglePin() {
  const newPinState = !isPinned;
  setIsPinned(newPinState);

  try {
    await invoke('set_window_pinned', {
      pinned: newPinState
    });

    // 更新按钮样式
    if (newPinState) {
      pinButton.style.color = '#108FEB'
      pinButton.innerHTML = '<i class="ti ti-pinned"></i>'
      pinButton.title = '取消固定';
    } else {
      pinButton.style.color = ''
      pinButton.innerHTML = '<i class="ti ti-pin"></i>'
      pinButton.title = '固定窗口';
    }
  } catch (error) {
    console.error('设置窗口固定状态失败:', error);
    // 如果设置失败，恢复原状态
    setIsPinned(!newPinState);
  }
}

// 设置窗口控制按钮
export function setupWindowControls() {
  if (pinButton) {
    pinButton.addEventListener('click', togglePin);
  }

  // 设置按钮事件
  const settingsButton = document.getElementById('settings-button');
  if (settingsButton) {
    settingsButton.addEventListener('click', openSettingsWindow);
  }
}

// 打开设置窗口
async function openSettingsWindow() {
  try {
    await invoke('open_settings_window');
  } catch (error) {
    console.error('打开设置窗口失败:', error);
  }
}
