// 常用文本管理器 - 完全替代quickTexts.js的新实现

import { invoke } from '@tauri-apps/api/core';
import { QuickTextListManager } from './list/quickTextListManager.js';
import { loadImageById as loadImage } from './list/index.js';
import {
  quickTexts,
  setQuickTexts,
  currentQuickTextsFilter,
  isOneTimePaste,
  editingQuickTextId,
  setEditingQuickTextId,
  quickTextsSearch,
  quickTextModal,
  modalTitle,
  quickTextTitleInput,
  quickTextContentInput,
  quickTextGroupSelect,
  pasteWithFormat
} from './config.js';
import { showNotification } from './notificationManager.js';
import { showAlertModal } from './ui.js';
import { getCurrentGroupId, updateGroupSelects } from './groups.js';
import { focusWindowImmediately } from './focus.js';

// 常用文本列表管理器实例
let quickTextListManager = null;

// 初始化常用文本列表管理器
function ensureListManager() {
  if (!quickTextListManager) {
    quickTextListManager = new QuickTextListManager({
      scrollId: 'quick-texts-list',
      contentId: 'quick-texts-content',
      isOneTimePaste,
      pasteWithFormat,
      currentGroupId: getCurrentGroupId(),
      
      onDataChange: (newData) => {
        setQuickTexts(newData);
        window.quickTexts = newData;
      }
    });
    
    // 设置初始数据
    quickTextListManager.setData(quickTexts);
  }
  return quickTextListManager;
}

// =================== 导出的公共API ===================

// 刷新常用文本列表
export async function refreshQuickTexts() {
  const manager = ensureListManager();
  
  // 更新当前分组
  manager.setCurrentGroup(getCurrentGroupId());
  
  await manager.refreshFromBackend();
  
  // 设置懒加载
  setupLazyImageLoading();
}

// 过滤常用文本
export function filterQuickTexts() {
  const manager = ensureListManager();
  
  // 更新过滤器状态
  manager.isOneTimePaste = isOneTimePaste;
  manager.pasteWithFormat = pasteWithFormat;
  manager.currentGroupId = getCurrentGroupId();
  
  const searchTerm = quickTextsSearch ? quickTextsSearch.value : '';
  manager.setFilter(currentQuickTextsFilter);
  manager.setSearch(searchTerm);
  manager.applyFilter();
  
  // 设置懒加载
  setupLazyImageLoading();
}

// 显示常用文本模态框
export async function showQuickTextModal(text = null) {
  setEditingQuickTextId(text ? text.id : null);

  // 更新分组选择下拉框
  updateGroupSelects();

  if (text) {
    modalTitle.textContent = '编辑常用文本';
    quickTextTitleInput.value = text.title;
    quickTextContentInput.value = text.content;
    quickTextGroupSelect.value = text.group_name || 'all';
  } else {
    modalTitle.textContent = '添加常用文本';
    quickTextTitleInput.value = '';
    quickTextContentInput.value = '';
    quickTextGroupSelect.value = getCurrentGroupId();
  }

  quickTextModal.classList.add('active');
  
  await focusWindowImmediately();
  quickTextTitleInput.focus();
}

// 隐藏常用文本模态框
export function hideQuickTextModal() {
  quickTextModal.classList.remove('active');
  setEditingQuickTextId(null);
}

// 编辑常用文本
export async function editQuickText(text) {
  try {
    // 打开文本编辑窗口
    await invoke('open_text_editor_window');

    // 准备编辑数据
    const editorData = {
      type: 'quick-text',
      id: text.id,
      title: text.title,
      content: text.content,
      groupId: text.group_name || text.groupId || '',
      timestamp: text.timestamp
    };

    // 延迟发送数据，确保窗口已完全加载
    setTimeout(async () => {
      try {
        const { emit } = await import('@tauri-apps/api/event');
        await emit('editor-data', editorData);
      } catch (error) {
        console.error('发送编辑数据失败:', error);
        showNotification('打开编辑器失败', 'error');
      }
    }, 500);

  } catch (error) {
    console.error('打开文本编辑器失败:', error);
    showNotification('打开编辑器失败', 'error');
  }
}

// 保存常用文本
export async function saveQuickText() {
  const title = quickTextTitleInput.value.trim();
  const content = quickTextContentInput.value.trim();
  const groupId = quickTextGroupSelect.value;

  if (!title || !content) {
    showAlertModal('提示', '请填写标题和内容');
    return;
  }

  try {
    // 直接传递分组名称
    const finalGroupName = groupId || '全部';

    if (editingQuickTextId) {
      // 更新
      await invoke('update_quick_text', {
        id: editingQuickTextId,
        title,
        content,
        groupName: finalGroupName
      });
    } else {
      // 添加
      await invoke('add_quick_text', {
        title,
        content,
        groupName: finalGroupName
      });
    }

    hideQuickTextModal();
    await refreshQuickTexts();

    // 显示成功提示
    const action = editingQuickTextId ? '更新' : '创建';
    showNotification(`${action}常用文本成功`, 'success');
  } catch (error) {
    console.error('保存常用文本失败:', error);
    // 如果后端还没有分组功能，回退到原来的方式
    try {
      if (editingQuickTextId) {
        await invoke('update_quick_text', {
          id: editingQuickTextId,
          title,
          content,
          groupName: null
        });
      } else {
        await invoke('add_quick_text', {
          title,
          content,
          groupName: null
        });
      }
      hideQuickTextModal();
      await refreshQuickTexts();

      // 显示成功提示
      const action = editingQuickTextId ? '更新' : '创建';
      showNotification(`${action}常用文本成功`, 'success');
    } catch (fallbackError) {
      console.error('保存常用文本失败（回退）:', fallbackError);
      showNotification('保存失败，请重试', 'error');
    }
  }
}

// 删除常用文本
export async function deleteQuickText(id) {
  const manager = ensureListManager();
  await manager.deleteItem(id);
}

// 更新常用文本顺序
export async function updateQuickTextsOrder(oldIndex, newIndex) {
  const manager = ensureListManager();
  await manager.handleSort(oldIndex, newIndex);
  
  // 设置懒加载
  setupLazyImageLoading();
}

// 设置常用文本功能
export function setupQuickTexts() {
  // 添加按钮
  document.getElementById('add-quick-text-btn').addEventListener('click', () => {
    showQuickTextModal();
  });

  // 模态框关闭按钮
  document.getElementById('modal-close-btn').addEventListener('click', hideQuickTextModal);
  document.getElementById('modal-cancel-btn').addEventListener('click', hideQuickTextModal);

  // 保存按钮
  document.getElementById('modal-save-btn').addEventListener('click', saveQuickText);

  // 在模态框中按Enter键保存
  quickTextTitleInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      saveQuickText();
    }
  });

  quickTextContentInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && e.ctrlKey) {
      saveQuickText();
    }
  });

  // 点击遮罩关闭模态框
  quickTextModal.addEventListener('click', (e) => {
    if (e.target === quickTextModal) {
      hideQuickTextModal();
    }
  });

  setupModalInputClickHandling();
}

// 渲染常用文本
export function renderQuickTexts() {
  const manager = ensureListManager();
  
  // 更新状态
  manager.isOneTimePaste = isOneTimePaste;
  manager.pasteWithFormat = pasteWithFormat;
  manager.currentGroupId = getCurrentGroupId();
  
  manager.render();
  
  // 设置懒加载
  setupLazyImageLoading();
}

// 加载图片
export async function loadImageById(imgElement, imageId) {
  await loadImage(imgElement, imageId);
}

// =================== 内部辅助函数 ===================

// 模态框输入框点击处理
function setupModalInputClickHandling() {
  const modalInputs = [quickTextTitleInput, quickTextContentInput];
  
  modalInputs.forEach(input => {
    if (!input) return;

    input.addEventListener('mousedown', async () => {
      if (!quickTextModal.classList.contains('active')) return;
      await focusWindowImmediately();
    });
  });
}

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

    document.querySelectorAll('#quick-texts-list img.lazy').forEach(img => {
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
  if (quickTextListManager) {
    quickTextListManager.pasteWithFormat = pasteWithFormat;
    renderQuickTexts();
  }
});

// 暴露到全局供导航系统使用
if (!window.quickTextsModule) {
  window.quickTextsModule = {};
}
Object.defineProperty(window.quickTextsModule, 'quickTextsVirtualList', {
  get() {
    return getVirtualList();
  }
});

