// 剪贴板管理器 - 完全替代clipboard.js的新实现

import { invoke } from '@tauri-apps/api/core';
import { ClipboardListManager } from './list/clipboardListManager.js';
import { loadImageById as loadImage } from './list/index.js';
import {
  clipboardHistory,
  setClipboardHistory,
  activeItemIndex,
  setActiveItemIndex,
  currentFilter,
  searchInput,
  isOneTimePaste,
  pasteWithFormat
} from './config.js';

// 剪贴板列表管理器实例
let clipboardListManager = null;

// 初始化剪贴板列表管理器
function ensureListManager() {
  if (!clipboardListManager) {
    clipboardListManager = new ClipboardListManager({
      scrollId: 'clipboard-list',
      contentId: 'clipboard-content',
      activeItemIndex,
      isOneTimePaste,
      pasteWithFormat,
      currentFilter,
      
      onActiveItemChange: (index) => {
        setActiveItemIndex(index);
      },
      
      onDataChange: (newData) => {
        setClipboardHistory(newData);
        window.clipboardHistory = newData;
      }
    });
    
    // 设置初始数据
    clipboardListManager.setData(clipboardHistory);
  }
  return clipboardListManager;
}

// =================== 导出的公共API ===================

// 读取剪贴板文本
export async function readClipboardText() {
  try {
    return await invoke('get_clipboard_text');
  } catch (error) {
    console.error('读取剪贴板文本失败:', error);
    return null;
  }
}

// 写入剪贴板文本
export async function writeClipboardText(text) {
  try {
    await invoke('set_clipboard_text', { text });
    return true;
  } catch (error) {
    console.error('写入剪贴板文本失败:', error);
    return false;
  }
}

// 写入剪贴板图片
export async function writeClipboardImage(dataUrl) {
  try {
    await invoke('set_clipboard_image', { data_url: dataUrl });
    return true;
  } catch (error) {
    console.error('写入剪贴板图片失败:', error);
    return false;
  }
}

// 增量添加剪贴板项
export function addClipboardItemIncremental(item, isNew) {
  const manager = ensureListManager();
  manager.addItemIncremental(item, isNew);
  
  // 设置懒加载
  setupLazyImageLoading();
}

// 刷新剪贴板历史
export async function refreshClipboardHistory() {
  const manager = ensureListManager();
  await manager.refreshFromBackend();
  
  // 设置懒加载
  setupLazyImageLoading();
}

// 设置活动项目
export function setActiveItem(index) {
  const manager = ensureListManager();
  manager.setActiveItem(index);
}

// 过滤剪贴板项目
export function filterClipboardItems() {
  const manager = ensureListManager();
  
  // 更新过滤器状态
  manager.isOneTimePaste = isOneTimePaste;
  manager.pasteWithFormat = pasteWithFormat;
  manager.currentFilter = currentFilter;
  
  const searchTerm = searchInput ? searchInput.value : '';
  manager.setFilter(currentFilter);
  manager.setSearch(searchTerm);
  manager.applyFilter();
  
  // 设置懒加载
  setupLazyImageLoading();
}

// 添加到常用文本
export async function addClipboardToFavorites(index) {
  try {
    const result = await invoke('add_clipboard_to_favorites', { index });
    const { showNotification } = await import('./notificationManager.js');
    showNotification('已添加到常用文本', 'success');
    return result;
  } catch (error) {
    console.error('添加到常用失败:', error);
    const { showNotification } = await import('./notificationManager.js');
    showNotification(error, 'error');
    throw error;
  }
}

// 更新剪贴板历史顺序
export async function updateClipboardOrder(oldIndex, newIndex) {
  const manager = ensureListManager();
  await manager.handleSort(oldIndex, newIndex);
  
  // 设置懒加载
  setupLazyImageLoading();
}

// 渲染剪贴板项目
export function renderClipboardItems() {
  const manager = ensureListManager();
  
  // 更新状态
  manager.activeItemIndex = activeItemIndex;
  manager.isOneTimePaste = isOneTimePaste;
  manager.pasteWithFormat = pasteWithFormat;
  manager.currentFilter = currentFilter;
  
  manager.render();
  
  // 设置懒加载
  setupLazyImageLoading();
}

// 加载图片
export async function loadImageById(imgElement, imageId) {
  await loadImage(imgElement, imageId);
}

// 获取剪贴板中的文件列表
export async function getClipboardFiles() {
  try {
    const files = await invoke('get_clipboard_files');
    return files;
  } catch (error) {
    console.error('获取剪贴板文件失败:', error);
    return [];
  }
}

// =================== 内部辅助函数 ===================

// 设置懒加载图片
function setupLazyImageLoading() {
  // 延迟执行以确保DOM已更新
  setTimeout(() => {
    const observer = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          const img = entry.target;
          
          if (img.dataset.imageId) {
            // 剪贴板/常用文本图片
            loadImageById(img, img.dataset.imageId);
            observer.unobserve(img);
          } else if (img.dataset.src) {
            // 文件图标
            img.src = img.dataset.src;
            img.classList.remove('image-loading');
            observer.unobserve(img);
          }
        }
      });
    });

    document.querySelectorAll('#clipboard-list img.lazy').forEach(img => {
      observer.observe(img);
    });
  }, 0);
}

// 获取虚拟列表实例（供导航系统使用）
export function getVirtualList() {
  const manager = ensureListManager();
  return manager.getVirtualList();
}

// 监听格式模式变化
window.addEventListener('format-mode-changed', () => {
  if (clipboardListManager) {
    clipboardListManager.pasteWithFormat = pasteWithFormat;
    renderClipboardItems();
  }
});

// 暴露到全局供导航系统使用
if (!window.clipboardModule) {
  window.clipboardModule = {};
}
Object.defineProperty(window.clipboardModule, 'clipboardVirtualList', {
  get() {
    return getVirtualList();
  }
});

