import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
  appWindow,
  clipboardHistory,
  alertModal,
  settingsModal,
  quickTextModal,
  confirmModal
} from './config.js';
import { refreshClipboardHistory, copyToClipboard, setActiveItem } from './clipboard.js';
import { hideAlertModal, hideConfirmModal } from './ui.js';
import { hideQuickTextModal } from './quickTexts.js';

// 检查是否有输入框获得焦点
function isInputFocused() {
  const activeElement = document.activeElement;
  return activeElement && (
    activeElement.tagName === 'INPUT' ||
    activeElement.tagName === 'TEXTAREA' ||
    activeElement.contentEditable === 'true'
  );
}

// 设置剪贴板变化事件监听
export async function setupClipboardEventListener() {
  try {
    // 监听来自后端的剪贴板变化事件
    await listen('clipboard-changed', async () => {
      console.log('收到剪贴板变化通知');

      // 刷新剪贴板历史
      refreshClipboardHistory();

      // 检查是否需要复制时翻译
      try {
        // 首先检查是否正在粘贴状态，避免循环翻译
        const isPasting = await invoke('is_currently_pasting');
        if (isPasting) {
          console.log('当前处于粘贴状态，跳过复制时翻译检查');
          return;
        }

        // 获取最新的剪贴板内容
        const clipboardText = await invoke('get_clipboard_text');
        if (clipboardText && clipboardText.trim()) {
          // 动态导入AI翻译模块并执行复制时翻译
          const { translateAndInputOnCopy } = await import('./aiTranslation.js');
          await translateAndInputOnCopy(clipboardText);
        }
      } catch (error) {
        // 复制时翻译失败不应该影响正常的剪贴板功能
        console.warn('复制时翻译检查失败:', error);
      }
    });

    // 监听常用文本刷新事件
    await listen('refreshQuickTexts', () => {
      console.log('收到常用文本刷新通知');
      import('../js/quickTexts.js').then(module => {
        module.refreshQuickTexts();
      });
    });

    console.log('剪贴板和常用文本事件监听器已设置');
  } catch (error) {
    console.error('设置事件监听失败:', error);
  }
}

// 设置托盘事件监听
export async function setupTrayEventListeners() {
  try {
    // 监听来自托盘的打开设置事件
    await listen('open-settings', async () => {
      try {
        await invoke('open_settings_window');
      } catch (error) {
        console.error('打开设置窗口失败:', error);
      }
    });
  } catch (error) {
    console.error('设置托盘事件监听失败:', error);
  }
}

// 设置键盘快捷键
export function setupKeyboardShortcuts() {
  document.addEventListener('keydown', (e) => {
    // ESC键处理
    if (e.key === 'Escape') {
      // 如果有模态框打开，先关闭模态框
      if (alertModal.classList.contains('active')) {
        hideAlertModal();
        return;
      }

      if (quickTextModal.classList.contains('active')) {
        hideQuickTextModal();
        return;
      }
      if (confirmModal.classList.contains('active')) {
        hideConfirmModal();
        return;
      }
      // 否则关闭窗口
      appWindow.hide();
    }

    // Enter键处理
    if (e.key === 'Enter') {
      if (alertModal.classList.contains('active')) {
        hideAlertModal();
        return;
      }
      if (confirmModal.classList.contains('active')) {
        document.getElementById('confirm-ok-btn').click();
        return;
      }
    }

    // 数字键1-9选择剪贴板项目（只在没有模态框打开且没有输入框获得焦点时生效）
    if (e.key >= '1' && e.key <= '9' &&
      !alertModal.classList.contains('active') &&
      !settingsModal.classList.contains('active') &&
      !quickTextModal.classList.contains('active') &&
      !confirmModal.classList.contains('active') &&
      !isInputFocused()) {
      const index = parseInt(e.key) - 1;
      if (index < clipboardHistory.length) {
        copyToClipboard(clipboardHistory[index]);
        setActiveItem(index);
      }
    }
  });
}

// 设置窗口拖拽事件
// export function setupWindowDragEvents() {
//   const titleBar = document.querySelector('.title-bar');
//   let isDragging = false;

//   // 监听拖动开始/结束
//   titleBar.addEventListener('mousedown', () => {
//     isDragging = true;
//   });

//   window.addEventListener('mouseup', () => {
//     isDragging = false;
//   });

//   // 禁止双击标题栏最大化窗口
//   titleBar.addEventListener('dblclick', (e) => {
//     e.preventDefault();
//     e.stopPropagation();
//   });

//   return { isDragging };
// }

// 自定义窗口拖拽
export async function setupCustomWindowDrag() {
  document.getElementById('titlebar')?.addEventListener('mousedown', async (e) => {
    try {
      await invoke('restore_last_focus');
      console.log('恢复工具窗口模式');
    } catch (error) {
      console.error('恢复工具窗口模式失败:', error);
    }
    if (e.buttons === 1) {
      e.detail === 2
        ? appWindow.toggleMaximize()
        : appWindow.startDragging();
    }
  });
}

// 设置右键菜单禁用
export function setupContextMenuDisable() {
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault();
  });
}

// 设置搜索输入框事件
export function setupSearchEvents() {
  const searchInput = document.querySelector('#search-input');

  searchInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      restoreFocus();
      // 可选：searchInput.blur();
    }
  });
}

// 恢复焦点的辅助函数
function restoreFocus() {
  // 这个函数可以根据需要实现焦点恢复逻辑
  console.log('恢复焦点');
}
