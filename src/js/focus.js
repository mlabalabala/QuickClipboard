import { invoke } from '@tauri-apps/api/core';
import {
  searchInput,
  quickTextsSearch,
  quickTextTitleInput,
  quickTextContentInput,
  groupNameInput
} from './config.js';

// 初始化输入框焦点管理
export function initInputFocusManagement() {
  // 获取所有需要管理焦点的输入框
  const inputElements = [
    searchInput,
    quickTextsSearch,
    quickTextTitleInput,
    quickTextContentInput,
    groupNameInput
  ];

  inputElements.forEach(input => {
    if (input) {
      // 获得焦点时临时启用窗口焦点
      input.addEventListener('focus', async () => {
        try {
          await invoke('focus_clipboard_window');
          console.log('临时启用窗口焦点');
        } catch (error) {
          console.error('启用窗口焦点失败:', error);
        }
      });

      // 失去焦点时恢复工具窗口模式
      input.addEventListener('blur', async () => {
        try {
          await invoke('restore_last_focus');
          console.log('恢复工具窗口模式');
        } catch (error) {
          console.error('恢复工具窗口模式失败:', error);
        }
      });
    }
  });

  // 为主题设置的单选按钮添加焦点管理
  const themeRadios = document.querySelectorAll('input[name="theme"]');
  themeRadios.forEach(radio => {
    radio.addEventListener('focus', async () => {
      try {
        await invoke('enable_window_focus_temp');
        console.log('临时启用窗口焦点 (主题设置)');
      } catch (error) {
        console.error('启用窗口焦点失败:', error);
      }
    });

    radio.addEventListener('blur', async () => {
      try {
        await invoke('disable_window_focus_temp');
        console.log('恢复工具窗口模式 (主题设置)');
      } catch (error) {
        console.error('恢复工具窗口模式失败:', error);
      }
    });
  });
}

// 手动启用窗口焦点（用于特殊情况）
export async function enableWindowFocus() {
  try {
    await invoke('enable_window_focus_temp');
  } catch (error) {
    console.error('启用窗口焦点失败:', error);
  }
}

// 手动禁用窗口焦点（用于特殊情况）
export async function disableWindowFocus() {
  try {
    await invoke('disable_window_focus_temp');
  } catch (error) {
    console.error('禁用窗口焦点失败:', error);
  }
}

// 恢复焦点
export async function restoreFocus() {
  try {
    await invoke('restore_last_focus');
  } catch (error) {
    console.error('恢复焦点失败:', error);
  }
}