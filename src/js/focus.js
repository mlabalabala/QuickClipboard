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
        } catch (error) {
          console.error('启用窗口焦点失败:', error);
        }
      });

      // 失去焦点时恢复工具窗口模式
      input.addEventListener('blur', async () => {
        try {
          await invoke('restore_last_focus');
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
      } catch (error) {
        console.error('启用窗口焦点失败:', error);
      }
    });

    radio.addEventListener('blur', async () => {
      try {
        await invoke('disable_window_focus_temp');
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

// 自动聚焦搜索框（根据设置）
export async function autoFocusSearchIfEnabled() {
  try {
    // 获取设置
    const settings = await invoke('get_settings');
    
    if (!settings.autoFocusSearch) {
      return false; // 如果没有启用自动聚焦，返回false
    }
    
    // 使用 navigation.js 中的 focusSearchBox 函数
    const { focusSearchBox } = await import('./navigation.js');
    await focusSearchBox();
    return true;
  } catch (error) {
    console.error('自动聚焦搜索框失败:', error);
    return false;
  }
}

// 移除搜索框焦点
export function blurSearchInputs() {
  const searchInputs = [
    document.getElementById('search-input'),
    document.getElementById('quick-texts-search')
  ];
  
  searchInputs.forEach(input => {
    if (input && document.activeElement === input) {
      input.blur();
    }
  });
}