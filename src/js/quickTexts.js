import { invoke } from '@tauri-apps/api/core';
import {
  quickTexts,
  setQuickTexts,
  isDragging,
  currentQuickTextsFilter,
  isOneTimePaste,
  editingQuickTextId,
  setEditingQuickTextId,
  quickTextsSearch,
  quickTextsList,
  quickTextModal,
  modalTitle,
  quickTextTitleInput,
  quickTextContentInput,
  quickTextGroupSelect
} from './config.js';
import { getContentType, loadImageById } from './clipboard.js';
import { showAlertModal, showConfirmModal, showNotification } from './ui.js';
import { getCurrentGroupId, updateGroupSelects } from './groups.js';
import { showContextMenu } from './contextMenu.js';



// 刷新常用文本列表
export async function refreshQuickTexts() {
  let retries = 3;

  while (retries > 0) {
    try {
      const currentGroupId = getCurrentGroupId();
      let texts;

      if (currentGroupId === 'all') {
        texts = await invoke('get_quick_texts');
      } else {
        try {
          texts = await invoke('get_quick_texts_by_group', { groupId: currentGroupId });
        } catch (groupError) {
          console.warn('按分组获取常用文本失败，回退到获取全部:', groupError);
          texts = await invoke('get_quick_texts');
        }
      }

      setQuickTexts(texts);
      renderQuickTexts();
      return; // 成功获取，退出重试循环
    } catch (error) {
      console.error('获取常用文本失败:', error);
      retries--;
      if (retries > 0) {
        await new Promise(resolve => setTimeout(resolve, 200)); // 等待200ms后重试
      }
    }
  }

  // 如果完全失败，设置空数组
  setQuickTexts([]);
  renderQuickTexts();
}

// 过滤常用文本
export function filterQuickTexts() {
  renderQuickTexts();
}

// 显示常用文本模态框（用于添加新文本）
export function showQuickTextModal(text = null) {
  setEditingQuickTextId(text ? text.id : null);

  // 更新分组选择下拉框
  updateGroupSelects();

  if (text) {
    modalTitle.textContent = '编辑常用文本';
    quickTextTitleInput.value = text.title;
    quickTextContentInput.value = text.content;
    quickTextGroupSelect.value = text.group_id || 'all';
  } else {
    modalTitle.textContent = '添加常用文本';
    quickTextTitleInput.value = '';
    quickTextContentInput.value = '';
    quickTextGroupSelect.value = getCurrentGroupId();
  }

  quickTextModal.classList.add('active');
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
      groupId: text.group_id || text.groupId || '',
      timestamp: text.timestamp
    };

    // 延迟发送数据，确保窗口已完全加载
    setTimeout(async () => {
      try {
        // 获取编辑器窗口并发送数据
        const { emit } = await import('@tauri-apps/api/event');
        await emit('editor-data', editorData);
        console.log('已发送常用文本编辑数据到文本编辑器');
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
    // 直接传递分组ID，就像拖拽功能一样
    const finalGroupId = groupId || 'all';

    if (editingQuickTextId) {
      // 更新
      await invoke('update_quick_text', {
        id: editingQuickTextId,
        title,
        content,
        groupId: finalGroupId
      });
    } else {
      // 添加
      await invoke('add_quick_text', {
        title,
        content,
        groupId: finalGroupId
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
          group_id: null
        });
      } else {
        await invoke('add_quick_text', {
          title,
          content,
          group_id: null
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
  showConfirmModal('确认删除', '确定要删除这个常用文本吗？', async () => {
    try {
      await invoke('delete_quick_text', { id });
      await refreshQuickTexts();
      showNotification('已删除常用文本', 'success');
    } catch (error) {
      console.error('删除常用文本失败:', error);
      showNotification('删除失败，请重试', 'error');
    }
  });
}

// 更新常用文本顺序
export async function updateQuickTextsOrder(oldIndex, newIndex) {
  try {
    // 获取当前显示的项目（考虑搜索过滤）
    const searchTerm = quickTextsSearch.value.toLowerCase();
    let visibleItems = [];

    if (searchTerm) {
      // 如果有搜索，只处理可见的项目
      visibleItems = quickTexts.filter(text => {
        return text.title.toLowerCase().includes(searchTerm) ||
          text.content.toLowerCase().includes(searchTerm);
      });
    } else {
      // 没有搜索，处理所有项目
      visibleItems = [...quickTexts];
    }

    // 重新排列可见项目
    const [movedItem] = visibleItems.splice(oldIndex, 1);
    visibleItems.splice(newIndex, 0, movedItem);

    if (searchTerm) {
      // 如果有搜索过滤，需要将重新排序的结果合并回完整列表
      let filteredIndex = 0;
      const newTexts = quickTexts.map(text => {
        const matches = text.title.toLowerCase().includes(searchTerm) ||
          text.content.toLowerCase().includes(searchTerm);
        if (!matches) {
          return text;
        } else {
          return visibleItems[filteredIndex++];
        }
      });
      setQuickTexts(newTexts);
    } else {
      // 没有搜索过滤，直接使用重新排序的结果
      setQuickTexts(visibleItems);
    }

    // 调用后端更新顺序
    await invoke('reorder_quick_texts', {
      items: quickTexts.map(text => ({
        id: text.id,
        title: text.title,
        content: text.content,
        created_at: text.created_at,
        updated_at: text.updated_at,
        group_id: text.group_id || 'all'  // 确保包含 group_id
      }))
    });

    // 重新渲染列表
    renderQuickTexts();
  } catch (error) {
    console.error('更新常用文本顺序失败:', error);
    // 如果更新失败，重新获取常用文本
    await refreshQuickTexts();
  }
}

// 设置常用文本功能
export function setupQuickTexts() {
  // 添加按钮 - 仍然使用模态框
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
}

// 渲染常用文本列表
export function renderQuickTexts() {
  quickTextsList.innerHTML = '';

  const searchTerm = quickTextsSearch.value.toLowerCase();
  const filterType = currentQuickTextsFilter;

  // 过滤常用文本
  const filteredTexts = quickTexts.filter(text => {
    const contentType = getContentType(text.content);
    const isImage = contentType === 'image';

    // 类型筛选
    if (filterType !== 'all' && contentType !== filterType) {
      return false;
    }

    // 搜索过滤：仅匹配文本和链接条目
    if (searchTerm) {
      if (isImage) return false;
      return text.title.toLowerCase().includes(searchTerm) ||
        text.content.toLowerCase().includes(searchTerm);
    }

    return true;
  });

  if (filteredTexts.length === 0) {
    const emptyMessage = document.createElement('div');
    emptyMessage.className = 'empty-state';
    emptyMessage.innerHTML = searchTerm ?
      '<div class="empty-icon">🔍</div><div class="empty-text">没有匹配的常用文本</div>' :
      '<div class="empty-icon">📝</div><div class="empty-text">暂无常用文本</div><div class="empty-hint">点击添加按钮创建第一个常用文本</div>';
    quickTextsList.appendChild(emptyMessage);
    return;
  }

  filteredTexts.forEach(text => {
    const quickTextItem = document.createElement('div');
    quickTextItem.className = 'quick-text-item';

    // 创建标题
    const titleElement = document.createElement('div');
    titleElement.className = 'quick-text-title';
    titleElement.textContent = text.title;

    // 创建内容
    const contentElement = document.createElement('div');
    contentElement.className = 'quick-text-content';

    // 根据内容类型显示不同内容
    const contentType = getContentType(text.content);
    if (contentType === 'image') {
      const imgElement = document.createElement('img');
      imgElement.className = 'quick-text-image';

      // 禁用图片元素的拖拽，避免与父容器拖拽冲突
      imgElement.draggable = false;

      // 处理不同格式的图片内容
      if (text.content.startsWith('image:')) {
        // 新格式：image:id，需要通过loadImageById加载
        const imageId = text.content.substring(6); // 去掉 "image:" 前缀
        loadImageById(imgElement, imageId, true); // 使用缩略图
      } else if (text.content.startsWith('data:image/')) {
        // 旧格式：完整的data URL
        imgElement.src = text.content;
      } else {
        // 未知格式，显示占位符
        imgElement.alt = '图片加载失败';
        imgElement.style.backgroundColor = '#e0e0e0';
        imgElement.textContent = '图片加载失败';
      }

      contentElement.appendChild(imgElement);
    } else {
      contentElement.textContent = text.content;
    }

    // 添加右键菜单
    quickTextItem.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      showQuickTextContextMenu(e, text);
    });

    // 设置拖拽属性
    quickTextItem.draggable = true;
    quickTextItem.addEventListener('dragstart', (e) => {
      const dragData = JSON.stringify({
        type: 'quicktext',
        id: text.id,
        title: text.title,
        content: text.content
      });

      // 使用自定义MIME类型避免与默认HTML拖拽冲突
      e.dataTransfer.setData('application/x-quickclipboard', dragData);
      e.dataTransfer.setData('text/plain', dragData);

      // 设置拖拽效果
      e.dataTransfer.effectAllowed = 'move';

      // 添加拖拽状态类
      document.querySelector('.tab-content.active').classList.add('dragging');
    });

    quickTextItem.addEventListener('dragend', () => {
      // 移除拖拽状态类
      document.querySelector('.tab-content.active').classList.remove('dragging');
    });

    // 添加点击事件（粘贴）
    quickTextItem.addEventListener('click', async () => {
      // 如果正在拖拽，不执行点击事件
      if (isDragging) return;

      // 检查是否正在处理中
      if (quickTextItem.classList.contains('processing')) {
        return;
      }

      try {
        // 如果是图片，显示加载状态
        const isImage = getContentType(text.content) === 'image';
        if (isImage) {
          quickTextItem.classList.add('processing');
          const loadingIndicator = document.createElement('div');
          loadingIndicator.className = 'loading-indicator';
          loadingIndicator.innerHTML = '<div class="spinner"></div><span>准备中...</span>';
          quickTextItem.appendChild(loadingIndicator);
        }

        const params = {
          id: text.id,
          one_time: isOneTimePaste
        };
        await invoke('paste_quick_text', { params });
        // 粘贴逻辑已在Rust端处理窗口显示/隐藏

        // 如果是一次性粘贴，刷新常用文本列表
        if (isOneTimePaste) {
          await refreshQuickTexts();
        }
      } catch (error) {
        console.error('粘贴常用文本失败:', error);
        // 显示错误提示
        showNotification('粘贴失败: ' + error, 'error');
      } finally {
        // 清理加载状态
        quickTextItem.classList.remove('processing');
        const loadingIndicator = quickTextItem.querySelector('.loading-indicator');
        if (loadingIndicator) {
          loadingIndicator.remove();
        }
      }
    });

    quickTextItem.appendChild(titleElement);
    quickTextItem.appendChild(contentElement);
    quickTextsList.appendChild(quickTextItem);
  });
}



// 显示常用文本右键菜单
function showQuickTextContextMenu(event, text) {
  const menuItems = [
    {
      icon: 'ti-edit',
      text: '编辑',
      onClick: () => {
        editQuickText(text);
      }
    },
    {
      icon: 'ti-trash',
      text: '删除',
      style: { color: '#ff4d4f' },
      onClick: () => {
        deleteQuickText(text.id);
      }
    }
  ];

  showContextMenu(event, {
    content: text.content,
    items: menuItems
  });
}
