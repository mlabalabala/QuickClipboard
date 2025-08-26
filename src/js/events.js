import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
  appWindow
} from './config.js';
import { refreshClipboardHistory } from './clipboard.js';
import { hideContextMenu } from './contextMenu.js';
import { forceClosePanel } from './toolsPanel.js';


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

// 自定义窗口拖拽
export async function setupCustomWindowDrag() {
  const titlebar = document.getElementById('titlebar');
  const footer = document.getElementById('footer');
  const container = document.querySelector('.container');
  const controls = document.querySelector('.controls');

  // 公共拖拽处理函数
  const handleDrag = async (element, e) => {
    // 对于footer，允许子元素拖拽；对于其他元素，只允许元素本身拖拽
    if (element !== footer && e.target !== element) {
      return; // 点击的是子元素，不执行拖拽
    }
    try {
      await invoke('restore_last_focus');
      console.log('恢复工具窗口模式');
    } catch (error) {
      console.error('恢复工具窗口模式失败:', error);
    }
    if (e.buttons === 1) {
      // 拖拽开始时关闭工具面板
      forceClosePanel();
      appWindow.startDragging();
      hideContextMenu()
    }
  };

  // 标题栏拖拽
  titlebar?.addEventListener('mousedown', (e) => handleDrag(titlebar, e));

  // footer拖拽
  footer?.addEventListener('mousedown', (e) => handleDrag(footer, e));

  // container拖拽
  container?.addEventListener('mousedown', (e) => handleDrag(container, e));

  // controls拖拽
  controls?.addEventListener('mousedown', (e) => handleDrag(controls, e));
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
      searchInput.blur();
    }
  });
}

// 恢复焦点的辅助函数
async function restoreFocus() {
  console.log('恢复焦点');
  await invoke('restore_last_focus');
}
