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
import { getCurrentGroupId, updateGroupSelects, getGroups } from './groups.js';
import { escapeHtml, formatTimestamp } from './utils/formatters.js';
import { VirtualList } from './virtualList.js';
import { shouldTranslateText, safeTranslateAndInputText, showTranslationIndicator, hideTranslationIndicator } from './aiTranslation.js';
import { showContextMenu } from './contextMenu.js';


// 虚拟列表实例
let quickTextsVirtualList = null;

// 生成常用文本项目HTML字符串
function generateQuickTextItemHTML(text, index) {
  const contentType = getContentType(text.content);

  let contentHTML = '';

  // 生成内容HTML
  if (contentType === 'image') {
    contentHTML = generateQuickTextImageHTML(text);
  } else if (contentType === 'files') {
    contentHTML = generateQuickTextFilesHTML(text);
  } else {
    contentHTML = `
      <div class="quick-text-title">${escapeHtml(text.title)}</div>
      <div class="quick-text-content">${escapeHtml(text.content)}</div>
    `;
  }

  // 生成日期时间HTML
  // 对于文件类型，时间戳会在文件HTML内部显示，所以这里不显示
  const timestampHTML = contentType === 'files' ? '' : `<div class="quick-text-timestamp">${formatTimestamp(text.created_at)}</div>`;

  // 在"全部"分组中显示分组标签
  const groupBadgeHTML = generateGroupBadgeHTML(text);

  return `
    <div class="quick-text-item" draggable="true" data-index="${index}">
      ${timestampHTML}
      ${groupBadgeHTML}
      ${contentHTML}
    </div>
  `;
}

// 生成分组标签HTML
function generateGroupBadgeHTML(text) {
  // 只在"全部"分组中显示分组标签
  const currentGroupId = getCurrentGroupId();
  if (currentGroupId !== 'all') {
    return '';
  }

  // 获取项目的分组信息
  const itemGroupId = text.group_id || 'all';
  if (itemGroupId === 'all') {
    return '';
  }

  try {
    const groups = getGroups();
    const group = groups.find(g => g.id === itemGroupId);

    if (group) {
      return `
        <div class="group-badge">
          <i class="${group.icon}"></i>
          <span>${escapeHtml(group.name)}</span>
        </div>
      `;
    }
  } catch (error) {
    console.warn('获取分组信息失败:', error);
  }

  return '';
}

// 生成常用文本图片HTML
function generateQuickTextImageHTML(text) {
  // 为图片元素生成唯一ID，用于后续异步加载
  const imgId = `img-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

  if (text.content.startsWith('image:')) {
    // 从content中提取image_id
    const imageId = text.content.substring(6);
    return `
      <img id="${imgId}" class="quick-text-image" src="" alt="常用图片" data-image-id="${imageId}" data-needs-load="true" loading="lazy">
    `;
  } else if (text.content.startsWith('data:image/')) {
    // 旧格式的完整图片数据
    return `
      <div class="quick-text-title">${escapeHtml(text.title)}</div>
      <img class="quick-text-image" src="${text.content}" alt="常用图片" loading="lazy">
    `;
  } else {
    // 未知格式，显示占位符
    return `
      <div class="quick-text-title">${escapeHtml(text.title)}</div>
      <div class="quick-text-image" style="background-color: #e0e0e0; display: flex; align-items: center; justify-content: center; color: #666;">图片加载失败</div>
    `;
  }
}

// 生成文件图标HTML字符串（复用clipboard.js中的逻辑）
function generateFileIconHTML(file, size = 'medium') {
  const sizeMap = {
    small: '16px',
    medium: '20px',
    large: '24px'
  };

  const iconSize = sizeMap[size] || sizeMap.medium;
  const alt = file.file_type || '文件';

  // 获取图标数据
  let iconSrc = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQiIGhlaWdodD0iMjQiIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMyIgeT0iMyIgd2lkdGg9IjE4IiBoZWlnaHQ9IjE4IiBmaWxsPSIjQ0NDQ0NDIi8+Cjwvc3ZnPgo=';
  let needsAsyncLoad = false;
  let filePath = '';

  if (file.icon_data) {
    if (file.icon_data.startsWith('image_file://')) {
      // 这是一个图片文件路径，需要异步加载
      filePath = file.icon_data.substring(13);
      needsAsyncLoad = true;
    } else {
      // 使用原有的base64数据
      iconSrc = file.icon_data;
    }
  }

  const dataAttributes = needsAsyncLoad ?
    `data-file-path="${escapeHtml(filePath)}" data-needs-load="true"` : '';

  return `<img class="file-icon" src="${iconSrc}" alt="${escapeHtml(alt)}" style="width: ${iconSize}; height: ${iconSize}; object-fit: cover; border-radius: 2px;" ${dataAttributes}>`;
}

// 生成常用文本文件HTML
function generateQuickTextFilesHTML(text) {
  try {
    const filesJson = text.content.substring(6);
    const filesData = JSON.parse(filesJson);

    // 格式化时间
    const timeStr = formatTimestamp(text.created_at);

    let filesHTML = `<div class="quick-text-title">${escapeHtml(text.title)}</div>`;
    // 顶部显示：时间和文件数量
    filesHTML += `<div class="file-summary">${timeStr} • ${filesData.files.length} 个文件</div>`;
    filesHTML += '<div class="files-container">';

    filesData.files.forEach(file => {
      const iconHTML = generateFileIconHTML(file, 'medium');
      const fileSize = formatFileSize(file.size || 0);
      filesHTML += `
        <div class="file-item">
          ${iconHTML}
          <div class="file-info">
            <div class="file-name">${escapeHtml(file.name)} <span class="file-size">${fileSize}</span></div>
            <div class="file-path">${escapeHtml(file.path)}</div>
          </div>
        </div>
      `;
    });

    filesHTML += '</div>';
    return filesHTML;
  } catch (error) {
    return `
      <div class="quick-text-title">${escapeHtml(text.title)}</div>
      <div class="quick-text-content">文件数据解析错误</div>
    `;
  }
}

// 格式化文件大小
function formatFileSize(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

// =================== 粘贴加载状态管理 ===================

// 显示粘贴加载状态
function showPasteLoading(element, message = '正在粘贴...') {
  // 给元素添加加载状态
  if (element) {
    element.classList.add('paste-loading');
  }

  // 显示全局加载指示器
  showPasteIndicator(message);
}

// 隐藏粘贴加载状态
function hidePasteLoading(element, success = true, message = null) {
  // 移除元素的加载状态
  if (element) {
    element.classList.remove('paste-loading');
  }

  // 显示结果状态
  if (success) {
    showPasteIndicator(message || '粘贴成功', 'success', 1500);
  } else {
    showPasteIndicator(message || '粘贴失败', 'error', 2000);
  }
}

// 显示粘贴指示器
function showPasteIndicator(message, type = 'loading', duration = 0) {
  // 移除现有的指示器
  const existingIndicator = document.querySelector('.paste-loading-indicator');
  if (existingIndicator) {
    existingIndicator.remove();
  }

  // 创建新的指示器
  const indicator = document.createElement('div');
  indicator.className = `paste-loading-indicator ${type}`;

  if (type === 'loading') {
    indicator.innerHTML = `
      <div class="loading-spinner"></div>
      <span>${message}</span>
    `;
  } else {
    indicator.innerHTML = `<span>${message}</span>`;
  }

  document.body.appendChild(indicator);

  // 显示动画
  setTimeout(() => {
    indicator.classList.add('show');
  }, 10);

  // 自动隐藏
  if (duration > 0) {
    setTimeout(() => {
      hidePasteIndicator();
    }, duration);
  }
}

// 隐藏粘贴指示器
function hidePasteIndicator() {
  const indicator = document.querySelector('.paste-loading-indicator');
  if (indicator) {
    indicator.classList.remove('show');
    setTimeout(() => {
      if (indicator.parentNode) {
        indicator.remove();
      }
    }, 300);
  }
}

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

// 计算在目标分组内的正确位置
function calculateTargetPositionInGroup(filteredData, newIndex, targetGroupId) {
  // 找到目标分组在filteredData中的所有项目
  const targetGroupItems = [];
  let targetIndexInGroup = 0;

  for (let i = 0; i < filteredData.length; i++) {
    const item = filteredData[i];
    const itemGroupId = item.group_id || 'all';

    if (itemGroupId === targetGroupId) {
      targetGroupItems.push({ item, originalIndex: i });

      // 如果当前索引小于等于newIndex，说明目标位置在这个项目之后
      if (i <= newIndex) {
        targetIndexInGroup = targetGroupItems.length;
      }
    }
  }

  return Math.max(0, Math.min(targetIndexInGroup - 1, targetGroupItems.length - 1));
}

// 更新常用文本顺序
export async function updateQuickTextsOrder(oldIndex, newIndex) {
  try {
    const filteredData = getFilteredQuickTextsData();

    if (oldIndex >= filteredData.length || newIndex >= filteredData.length) {
      return;
    }

    const movedItem = filteredData[oldIndex];
    const targetItem = filteredData[newIndex];

    if (!movedItem || !movedItem.id) {
      return;
    }

    // 在"全部"分组中，检查是否跨分组拖拽
    const currentGroupId = getCurrentGroupId();
    if (currentGroupId === 'all') {
      const movedItemGroupId = movedItem.group_id || 'all';
      const targetItemGroupId = targetItem ? (targetItem.group_id || 'all') : movedItemGroupId;

      if (movedItemGroupId !== targetItemGroupId) {
        // 跨分组拖拽：将项目移动到目标分组并排序到正确位置
        try {
          // 计算在目标分组内的正确位置
          const targetPositionInGroup = calculateTargetPositionInGroup(filteredData, newIndex, targetItemGroupId);

          // 先移动到目标分组
          await invoke('move_quick_text_to_group', {
            id: movedItem.id,
            groupId: targetItemGroupId
          });

          // 刷新数据以获取最新的分组内容
          await refreshQuickTexts();

          // 如果需要在分组内排序到特定位置
          if (targetPositionInGroup > 0) {
            // 获取目标分组的所有项目
            const targetGroupTexts = await invoke('get_quick_texts_by_group', {
              groupId: targetItemGroupId
            });

            // 找到刚移动的项目在目标分组中的当前位置（应该是第一个）
            const currentIndex = targetGroupTexts.findIndex(t => t.id === movedItem.id);

            if (currentIndex !== -1 && currentIndex !== targetPositionInGroup) {
              // 使用现有的move_quick_text_item命令在分组内排序
              await invoke('move_quick_text_item', {
                itemId: movedItem.id,
                toIndex: targetPositionInGroup
              });
            }
          }

          // 显示成功提示
          const { getGroups } = await import('./groups.js');
          const groups = getGroups();
          const targetGroupName = groups.find(g => g.id === targetItemGroupId)?.name || '分组';
          const { showNotification } = await import('./ui.js');
          showNotification(`已移动到 ${targetGroupName}`, 'success');

          await refreshQuickTexts();
          return;
        } catch (error) {
          console.error('跨分组移动失败:', error);
          const { showNotification } = await import('./ui.js');
          showNotification('移动到分组失败，请重试', 'error');
          return;
        }
      }
    }

    // 同分组内的排序
    await invoke('move_quick_text_item', {
      itemId: movedItem.id,
      toIndex: newIndex
    });

    await refreshQuickTexts();

  } catch (error) {
    console.error('更新常用文本顺序失败:', error);
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

// 初始化常用文本虚拟列表
function initQuickTextsVirtualList() {
  if (quickTextsVirtualList) {
    quickTextsVirtualList.destroy();
  }

  quickTextsVirtualList = new VirtualList({
    scrollId: 'quick-texts-list',
    contentId: 'quick-texts-content',
    data: getFilteredQuickTextsData(),
    renderItem: generateQuickTextItemHTML,
    onSort: updateQuickTextsOrder,
    onItemClick: handleQuickTextItemClick,
    onItemContextMenu: handleQuickTextItemContextMenu,
    sortableOptions: {
      onStart: () => {
        document.querySelector('.tab-content.active').classList.add('dragging');
        const sidebar = document.getElementById('groups-sidebar');
        if (sidebar && !sidebar.classList.contains('pinned')) {
          sidebar.classList.add('show');
        }
      },
      onEnd: () => {
        document.querySelector('.tab-content.active').classList.remove('dragging');
        const sidebar = document.getElementById('groups-sidebar');
        if (sidebar && !sidebar.classList.contains('pinned')) {
          sidebar.classList.remove('show');
        }
      }
    }
  });

  // 将虚拟列表实例暴露到全局，供导航系统使用
  if (!window.quickTextsModule) {
    window.quickTextsModule = {};
  }
  window.quickTextsModule.quickTextsVirtualList = quickTextsVirtualList;
}

// 获取过滤后的常用文本数据
function getFilteredQuickTextsData() {
  const searchTerm = quickTextsSearch.value.toLowerCase();
  const filterType = currentQuickTextsFilter;
  const currentGroupId = getCurrentGroupId();

  let filteredTexts = quickTexts.filter(text => {
    const contentType = getContentType(text.content);

    // 类型筛选
    if (filterType !== 'all' && contentType !== filterType) {
      return false;
    }

    // 搜索过滤：支持文本、链接和文件类型
    if (searchTerm) {
      if (contentType === 'files') {
        // 文件类型：搜索标题和文件内容
        try {
          const filesJson = text.content.substring(6); // 去掉 "files:" 前缀
          const filesData = JSON.parse(filesJson);
          const searchableText = filesData.files.map(file =>
            `${file.name} ${file.path} ${file.file_type}`
          ).join(' ').toLowerCase();
          return text.title.toLowerCase().includes(searchTerm) ||
            searchableText.includes(searchTerm);
        } catch (error) {
          return text.title.toLowerCase().includes(searchTerm);
        }
      } else if (contentType === 'image') {
        // 图片类型：只搜索标题
        return text.title.toLowerCase().includes(searchTerm);
      } else {
        // 文本和链接类型：搜索标题和内容
        return text.title.toLowerCase().includes(searchTerm) ||
          text.content.toLowerCase().includes(searchTerm);
      }
    }

    return true;
  });

  // 如果是"全部"分组，按分组顺序重新排列数据
  if (currentGroupId === 'all') {
    filteredTexts = sortTextsByGroupOrder(filteredTexts);
  }

  return filteredTexts;
}

// 按分组顺序排列文本数据
function sortTextsByGroupOrder(texts) {
  try {
    // 获取分组顺序
    const groupsOrder = getGroups();

    // 按group_id分组
    const textsByGroup = {};
    texts.forEach(text => {
      const groupId = text.group_id || 'all';
      if (!textsByGroup[groupId]) {
        textsByGroup[groupId] = [];
      }
      textsByGroup[groupId].push(text);
    });

    // 按分组顺序合并
    const sortedTexts = [];
    groupsOrder.forEach(group => {
      if (textsByGroup[group.id]) {
        // 每个分组内的数据已经是按顺序的（从数据库获取时）
        sortedTexts.push(...textsByGroup[group.id]);
      }
    });

    // 添加任何不在分组列表中的文本（防止遗漏）
    Object.keys(textsByGroup).forEach(groupId => {
      if (!groupsOrder.find(g => g.id === groupId)) {
        sortedTexts.push(...textsByGroup[groupId]);
      }
    });

    return sortedTexts;
  } catch (error) {
    console.warn('按分组顺序排列失败，使用原始顺序:', error);
    return texts;
  }
}

// 渲染常用文本列表
// 异步加载文件图标和图片
async function loadFileIcons() {
  // 加载文件图标
  const fileIcons = document.querySelectorAll('.file-icon[data-needs-load="true"]');

  for (const icon of fileIcons) {
    const filePath = icon.getAttribute('data-file-path');
    if (filePath) {
      try {
        const dataUrl = await invoke('read_image_file', { filePath });
        icon.src = dataUrl;
        icon.style.objectFit = 'cover';
        icon.style.borderRadius = '2px';
        icon.removeAttribute('data-needs-load');
        icon.removeAttribute('data-file-path');
      } catch (error) {
        console.warn('加载文件图标失败:', error);
        // 保持默认图标
      }
    }
  }

  // 加载常用文本图片
  const quickTextImages = document.querySelectorAll('.quick-text-image[data-needs-load="true"]');

  for (const img of quickTextImages) {
    const imageId = img.getAttribute('data-image-id');
    if (imageId) {
      try {
        await loadImageById(img, imageId, true); // 先加载缩略图
        img.removeAttribute('data-needs-load');
        img.removeAttribute('data-image-id');
      } catch (error) {
        console.warn('加载常用文本图片失败:', error);
        img.alt = '图片加载失败';
        img.style.backgroundColor = '#e0e0e0';
      }
    }
  }
}

export function renderQuickTexts() {
  if (!quickTextsVirtualList) {
    initQuickTextsVirtualList();
  } else {
    const filteredData = getFilteredQuickTextsData();
    quickTextsVirtualList.updateData(filteredData);
  }

  // 异步加载文件图标
  setTimeout(() => {
    loadFileIcons();
  }, 0);

  // 通知导航模块列表已更新
  import('./navigation.js').then(module => {
    module.onListUpdate();
  }).catch(() => { });
}

// 处理常用文本项目点击事件
function handleQuickTextItemClick(index, event) {
  if (isDragging) return;

  // 获取过滤后的数据，因为虚拟列表使用的是过滤后的数据
  const filteredData = getFilteredQuickTextsData();
  const text = filteredData[index];
  if (!text) return;

  // 处理主要的点击事件（粘贴）
  const quickTextItem = event.target.closest('.quick-text-item');
  handleQuickTextItemPaste(text, quickTextItem);
}

// 处理常用文本项目右键菜单
function handleQuickTextItemContextMenu(index, event) {
  event.preventDefault();

  // 获取过滤后的数据，因为虚拟列表使用的是过滤后的数据
  const filteredData = getFilteredQuickTextsData();
  const text = filteredData[index];
  if (text) {
    showQuickTextContextMenu(event, text);
  }
}



// 处理常用文本项目粘贴
async function handleQuickTextItemPaste(text, element = null) {
  try {
    // 检查是否需要AI翻译
    const contentType = getContentType(text.content);
    const isTextContent = contentType === 'text';
    const translationCheck = isTextContent ? shouldTranslateText(text.content, 'paste') : { should: false, reason: '非文本内容' };
    const needsTranslation = translationCheck.should;

    // 根据内容类型确定加载消息
    let loadingMessage = '正在粘贴...';
    if (contentType === 'files') {
      loadingMessage = '正在粘贴文件...';
    } else if (contentType === 'image') {
      loadingMessage = '正在粘贴图片...';
    } else if (needsTranslation) {
      loadingMessage = '正在翻译...';
    }

    // 显示加载状态
    showPasteLoading(element, loadingMessage);

    if (needsTranslation) {
      // 使用AI翻译并流式输入
      console.log('开始AI翻译:', text.content, '原因:', translationCheck.reason);
      showTranslationIndicator('正在翻译...');

      // 定义降级回调函数
      const fallbackPaste = async () => {
        await invoke('paste_content', {
          params: {
            content: text.content,
            one_time: text.one_time || false,
            quick_text_id: text.id
          }
        });
      };

      try {
        const result = await safeTranslateAndInputText(text.content, fallbackPaste);

        if (result.success) {
          if (result.method === 'translation') {
            console.log('AI翻译成功完成');
          } else if (result.method === 'fallback') {
            console.log('使用降级处理完成粘贴:', result.error);
          }

          hideTranslationIndicator();
          hidePasteLoading(element, true, '翻译粘贴成功');
        } else {
          console.error('AI翻译失败:', result.error);
          hideTranslationIndicator();
          hidePasteLoading(element, false, '翻译失败');
          showNotification('翻译失败，请重试', 'error');
        }
      } catch (error) {
        console.error('AI翻译过程中发生错误:', error);
        hideTranslationIndicator();
        hidePasteLoading(element, false, '翻译过程中发生错误');
        showNotification('翻译过程中发生错误', 'error');
      }
    } else {
      // 使用统一的粘贴命令
      await invoke('paste_content', {
        params: {
          content: text.content,
          one_time: text.one_time || false,
          quick_text_id: text.id
        }
      });

      // 根据内容类型显示成功消息
      let successMessage = '粘贴成功';
      if (contentType === 'files') {
        successMessage = '文件粘贴成功';
      } else if (contentType === 'image') {
        successMessage = '图片粘贴成功';
      }

      hidePasteLoading(element, true, successMessage);
    }
  } catch (error) {
    console.error('粘贴常用文本失败:', error);
    hidePasteLoading(element, false, '粘贴失败');
    showNotification('粘贴失败', 'error');
    hideTranslationIndicator();
  }
}



// 显示常用文本右键菜单
function showQuickTextContextMenu(event, text) {
  const contentType = getContentType(text.content);
  let menuItems = [];

  if (contentType === 'image') {
    // 图片类型菜单
    menuItems = [
      {
        icon: 'ti-eye',
        text: '查看原图',
        onClick: () => {
          viewOriginalImage(text);
        }
      },
      {
        icon: 'ti-download',
        text: '另存为图片',
        onClick: () => {
          saveImageAs(text);
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
  } else if (contentType === 'files') {
    // 文件类型菜单
    menuItems = [
      {
        icon: 'ti-external-link',
        text: '使用默认程序打开',
        onClick: () => {
          openFileWithDefaultProgram(text);
        }
      },
      {
        icon: 'ti-folder-open',
        text: '打开文件位置',
        onClick: () => {
          openFileLocation(text);
        }
      },
      {
        icon: 'ti-copy',
        text: '复制文件路径',
        onClick: () => {
          copyFilePaths(text);
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
  } else {
    // 文本和链接类型菜单
    menuItems = [
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
  }

  showContextMenu(event, {
    content: text.content,
    items: menuItems
  });
}



// 查看原图
function viewOriginalImage(text) {
  try {
    if (text.content.startsWith('image:')) {
      // 新格式：image:id，需要通过后端获取完整图片
      const imageId = text.content.substring(6);
      // 创建一个新窗口显示图片
      const newWindow = window.open('', '_blank');
      newWindow.document.write(`
        <html>
          <head><title>查看原图</title></head>
          <body style="margin:0;padding:20px;background:#000;display:flex;justify-content:center;align-items:center;min-height:100vh;">
            <img id="fullImage" style="max-width:100%;max-height:100%;object-fit:contain;" alt="原图" />
            <div id="loading" style="color:white;font-size:18px;">加载中...</div>
          </body>
        </html>
      `);

      // 加载完整图片
      loadImageById(newWindow.document.getElementById('fullImage'), imageId, false);
      newWindow.document.getElementById('loading').style.display = 'none';
    } else if (text.content.startsWith('data:image/')) {
      // 旧格式：完整的data URL
      const newWindow = window.open('', '_blank');
      newWindow.document.write(`
        <html>
          <head><title>查看原图</title></head>
          <body style="margin:0;padding:20px;background:#000;display:flex;justify-content:center;align-items:center;min-height:100vh;">
            <img src="${text.content}" style="max-width:100%;max-height:100%;object-fit:contain;" alt="原图" />
          </body>
        </html>
      `);
    }
  } catch (error) {
    console.error('查看原图失败:', error);
    showNotification('查看原图失败', 'error');
  }
}

// 另存为图片
async function saveImageAs(text) {
  try {
    if (!text.content.startsWith('data:image/') && !text.content.startsWith('image:')) {
      showNotification('此图片格式暂不支持直接保存', 'info');
      return;
    }

    // 使用文件对话框选择保存位置
    const { save } = await import('@tauri-apps/plugin-dialog');
    const filePath = await save({
      title: '保存图片',
      defaultPath: `image_${Date.now()}.png`,
      filters: [{
        name: '图片文件',
        extensions: ['png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp']
      }]
    });

    if (!filePath) {
      return; // 用户取消了操作
    }

    // 调用后端保存图片
    await invoke('save_image_to_file', {
      content: text.content,
      filePath: filePath
    });

    showNotification('图片已保存', 'success');
  } catch (error) {
    console.error('保存图片失败:', error);
    showNotification('保存图片失败', 'error');
  }
}
// 使用默认程序打开文件
async function openFileWithDefaultProgram(text) {
  try {
    const filesJson = text.content.substring(6); // 去掉 "files:" 前缀
    const filesData = JSON.parse(filesJson);

    if (filesData.files && filesData.files.length > 0) {
      const firstFilePath = filesData.files[0].path;
      await invoke('open_file_with_default_program', { filePath: firstFilePath });
      showNotification('已使用默认程序打开文件', 'success');
    }
  } catch (error) {
    console.error('打开文件失败:', error);
    showNotification('打开文件失败', 'error');
  }
}
// 打开文件位置
async function openFileLocation(text) {
  try {
    const filesJson = text.content.substring(6); // 去掉 "files:" 前缀
    const filesData = JSON.parse(filesJson);

    if (filesData.files && filesData.files.length > 0) {
      const firstFilePath = filesData.files[0].path;
      await invoke('open_file_location', { filePath: firstFilePath });
      showNotification('已打开文件位置', 'success');
    }
  } catch (error) {
    console.error('打开文件位置失败:', error);
    showNotification('打开文件位置失败', 'error');
  }
}

// 复制文件路径
async function copyFilePaths(text) {
  try {
    const filesJson = text.content.substring(6); // 去掉 "files:" 前缀
    const filesData = JSON.parse(filesJson);

    if (filesData.files && filesData.files.length > 0) {
      const paths = filesData.files.map(file => file.path).join('\n');
      await navigator.clipboard.writeText(paths);
      showNotification(`已复制 ${filesData.files.length} 个文件路径`, 'success');
    }
  } catch (error) {
    console.error('复制文件路径失败:', error);
    showNotification('复制文件路径失败', 'error');
  }
}
