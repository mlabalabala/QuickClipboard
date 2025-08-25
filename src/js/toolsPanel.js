// 可展开工具面板模块
import { setPasteWithFormat, getPasteWithFormat } from './config.js';
import { invoke } from '@tauri-apps/api/core';

let toolsPanelToggle = null;
let toolsPanel = null;
let isPanelOpen = false;
let formatToggleButton = null;

// 初始化工具面板
export function initToolsPanel() {
  toolsPanelToggle = document.getElementById('tools-panel-toggle');
  toolsPanel = document.getElementById('tools-panel');
  formatToggleButton = document.getElementById('format-toggle-button');

  if (!toolsPanelToggle || !toolsPanel) {
    console.warn('工具面板元素未找到');
    return;
  }

  // 点击切换按钮
  toolsPanelToggle.addEventListener('click', (e) => {
    e.stopPropagation();
    togglePanel();
  });

  // 点击面板外部关闭
  document.addEventListener('click', (e) => {
    if (isPanelOpen && !toolsPanel.contains(e.target) && !toolsPanelToggle.contains(e.target)) {
      closePanel();
    }
  });

  // 阻止面板内部点击事件冒泡
  toolsPanel.addEventListener('click', (e) => {
    e.stopPropagation();
  });

  // 格式切换按钮事件
  if (formatToggleButton) {
    formatToggleButton.addEventListener('click', (e) => {
      e.stopPropagation();
      toggleFormatMode();
    });
    
    // 初始化按钮状态
    updateFormatToggleButton();
  }

  // 键盘事件
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && isPanelOpen) {
      closePanel();
    }
  });

  console.log('工具面板已初始化');
}

// 切换面板显示状态
function togglePanel() {
  if (isPanelOpen) {
    closePanel();
  } else {
    openPanel();
  }
}

// 打开面板
function openPanel() {
  if (!toolsPanel) return;
  
  toolsPanel.classList.add('show');
  isPanelOpen = true;
  
  // 更新按钮状态
  if (toolsPanelToggle) {
    toolsPanelToggle.classList.add('active');
  }
}

// 关闭面板
function closePanel() {
  if (!toolsPanel) return;
  
  toolsPanel.classList.remove('show');
  isPanelOpen = false;
  
  // 更新按钮状态
  if (toolsPanelToggle) {
    toolsPanelToggle.classList.remove('active');
  }
}

// 获取面板状态
export function isToolsPanelOpen() {
  return isPanelOpen;
}

// 强制关闭面板（供其他模块调用）
export function forceClosePanel() {
  closePanel();
}

// 强制打开面板（供其他模块调用）
export function forceOpenPanel() {
  openPanel();
}

// 切换格式模式
async function toggleFormatMode() {
  const currentFormat = getPasteWithFormat();
  const newFormat = !currentFormat;
  
  setPasteWithFormat(newFormat);
  updateFormatToggleButton();
  
  // 保存设置到后端
  try {
    await invoke('save_settings', {
      settings: {
        pasteWithFormat: newFormat
      }
    });
  } catch (error) {
    console.error('保存格式设置失败:', error);
  }
  
  // 触发列表重新渲染以应用格式变化
  window.dispatchEvent(new CustomEvent('format-mode-changed', { 
    detail: { withFormat: newFormat } 
  }));
  
}

// 更新格式切换按钮状态
function updateFormatToggleButton() {
  if (!formatToggleButton) return;
  
  const withFormat = getPasteWithFormat();
  
  if (withFormat) {
    formatToggleButton.classList.add('active');
    formatToggleButton.title = '格式切换 - 当前：带格式粘贴，点击切换到纯文本';
  } else {
    formatToggleButton.classList.remove('active');
    formatToggleButton.title = '格式切换 - 当前：纯文本粘贴，点击切换到带格式';
  }
}

// 获取格式模式状态（供其他模块调用）
export function getFormatModeStatus() {
  return getPasteWithFormat();
}

// 更新格式按钮状态（供其他模块调用）
export function updateFormatButtonStatus() {
  updateFormatToggleButton();
}