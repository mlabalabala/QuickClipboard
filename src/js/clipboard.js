import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import {
  clipboardHistory,
  setClipboardHistory,
  activeItemIndex,
  setActiveItemIndex,
  isDragging,
  currentFilter,
  searchInput
} from './config.js';
import { showNotification } from './ui.js';
import { showContextMenu } from './contextMenu.js';
import {
  shouldTranslateText,
  safeTranslateAndInputText,
  showTranslationIndicator,
  hideTranslationIndicator
} from './aiTranslation.js';
import { escapeHtml, formatTimestamp } from './utils/formatters.js';

import { VirtualList } from './virtualList.js';

// 图片缓存
const imageCache = new Map();
const thumbnailCache = new Map();

// 虚拟列表实例
let clipboardVirtualList = null;

// 生成剪贴板项目HTML字符串
function generateClipboardItemHTML(item, index) {
  const isImage = item.is_image || item.text.startsWith('data:image/') || item.text.startsWith('image:');
  const contentType = isImage ? 'image' : getContentType(item.text);

  let contentHTML = '';

  // 生成内容HTML
  if (isImage) {
    contentHTML = generateImageHTML(item);
  } else if (contentType === 'files') {
    contentHTML = generateFilesHTML(item);
  } else {
    contentHTML = `<div class="clipboard-text">${escapeHtml(item.text)}</div>`;
  }

  // 生成序号和快捷键
  const numberHTML = `<div class="clipboard-number">${index + 1}</div>`;
  const shortcutHTML = index < 9 ?
    `<div class="clipboard-index">Ctrl+${index + 1}</div>` : '';

  // 生成操作按钮
  let actionsHTML = '<div class="clipboard-actions">';

  actionsHTML += '</div>';

  // 生成日期时间HTML - 优先使用created_at，如果为空则使用timestamp
  // 对于文件类型，时间戳会在文件HTML内部显示，所以这里不显示
  const timeValue = item.created_at || item.timestamp;
  const timestampHTML = contentType === 'files' ? '' : `<div class="clipboard-timestamp">${formatTimestamp(timeValue)}</div>`;

  // 组合完整的HTML
  const activeClass = index === activeItemIndex ? ' active' : '';
  const noShortcutClass = index >= 9 ? ' no-shortcut' : '';

  return `
    <div class="clipboard-item${activeClass}${noShortcutClass}" draggable="true" data-index="${index}">
      ${timestampHTML}
      ${contentHTML}
      ${numberHTML}
      ${shortcutHTML}
      ${actionsHTML}
    </div>
  `;
}



// 生成图片HTML
function generateImageHTML(item) {
  // 为图片元素生成唯一ID，用于后续异步加载
  const imgId = `img-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

  if (item.image_id) {
    // 新格式：使用image_id字段
    return `<img id="${imgId}" class="clipboard-image" src="" alt="剪贴板图片" data-image-id="${item.image_id}" data-needs-load="true" loading="lazy">`;
  } else if (item.text.startsWith('image:')) {
    // 从text中提取image_id
    const imageId = item.text.substring(6);
    return `<img id="${imgId}" class="clipboard-image" src="" alt="剪贴板图片" data-image-id="${imageId}" data-needs-load="true" loading="lazy">`;
  } else if (item.text.startsWith('data:image/')) {
    // 旧格式的完整图片数据
    return `<img class="clipboard-image" src="${item.text}" alt="剪贴板图片" loading="lazy">`;
  } else {
    // 未知格式，显示占位符
    return `<div class="clipboard-image" style="background-color: #e0e0e0; display: flex; align-items: center; justify-content: center; color: #666;">图片加载失败</div>`;
  }
}

// 生成文件图标HTML字符串
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

// 生成文件HTML
function generateFilesHTML(item) {
  try {
    const filesJson = item.text.substring(6);
    const filesData = JSON.parse(filesJson);

    // 格式化时间
    const timeStr = formatTimestamp(item.timestamp);

    // 顶部显示：时间和文件数量
    let filesHTML = `<div class="file-summary">${timeStr} • ${filesData.files.length} 个文件</div>`;
    filesHTML += '<div class="clipboard-files">';

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
    return `<div class="clipboard-text">文件数据解析错误</div>`;
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

// 刷新剪贴板历史
export async function refreshClipboardHistory() {
  let retries = 3;

  while (retries > 0) {
    try {
      const history = await invoke('get_clipboard_history');

      // 如果历史记录有变化，更新UI
      if (JSON.stringify(history) !== JSON.stringify(clipboardHistory)) {
        setClipboardHistory(history);

        // 将剪贴板历史设置到全局作用域，供虚拟列表拖拽使用
        window.clipboardHistory = history;

        renderClipboardItems();
      }
      return; // 成功获取，退出重试循环
    } catch (error) {
      console.error('刷新剪贴板历史失败:', error);
      retries--;
      if (retries > 0) {
        await new Promise(resolve => setTimeout(resolve, 200)); // 等待200ms后重试
      }
    }
  }

}

// 获取内容类型
export function getContentType(text) {
  // 图片类型 - 支持新的 image: 格式和旧的 data:image/ 格式
  if (text.startsWith('data:image/') || text.startsWith('image:')) {
    return 'image';
  }

  // 文件类型 - 检测 files: 格式
  if (text.startsWith('files:')) {
    return 'files';
  }

  // 链接类型 - 检测URL模式
  const urlPattern = /^(https?:\/\/|ftp:\/\/|www\.)[^\s]+$/i;
  const simpleUrlPattern = /^[a-zA-Z0-9-]+\.[a-zA-Z]{2,}([\/\w\-._~:/?#[\]@!$&'()*+,;=]*)?$/;

  if (urlPattern.test(text.trim()) || simpleUrlPattern.test(text.trim())) {
    return 'link';
  }

  // 检测文本中是否包含链接
  const containsUrlPattern = /(https?:\/\/|ftp:\/\/|www\.)[^\s]+/i;
  if (containsUrlPattern.test(text)) {
    return 'link';
  }

  // 默认为文本类型
  return 'text';
}

// 打开链接
async function openLink(url) {
  try {
    // 如果URL不包含协议，添加https://
    if (!url.match(/^https?:\/\//i)) {
      url = 'https://' + url;
    }

    await openUrl(url);
    showNotification('已在浏览器中打开链接', 'success', 2000);
  } catch (error) {
    console.error('打开链接失败:', error);
    showNotification('打开链接失败', 'error');
  }
}

// 设置活动项目
export function setActiveItem(index) {
  setActiveItemIndex(index);
  renderClipboardItems();
}

// 过滤剪贴板项目
export function filterClipboardItems() {
  renderClipboardItems();
}

// 将剪贴板内容添加到常用
export async function addClipboardToFavorites(index) {
  try {
    const result = await invoke('add_clipboard_to_favorites', { index });
    console.log('成功添加到常用:', result);

    // 显示成功提示（可选）
    showNotification('已添加到常用文本', 'success');

    // 返回结果，让调用者决定是否刷新常用文本列表
    return result;
  } catch (error) {
    console.error('添加到常用失败:', error);
    showNotification(error, 'error');
    throw error;
  }
}

// 更新剪贴板历史顺序
export async function updateClipboardOrder(oldIndex, newIndex) {
  try {
    const filteredData = getFilteredClipboardData();

    if (oldIndex >= filteredData.length || newIndex >= filteredData.length) {
      return;
    }

    const movedItem = filteredData[oldIndex];
    const targetItem = filteredData[newIndex];

    const originalOldIndex = clipboardHistory.findIndex(item =>
      item.text === movedItem.text && item.timestamp === movedItem.timestamp
    );
    const originalNewIndex = clipboardHistory.findIndex(item =>
      item.text === targetItem.text && item.timestamp === targetItem.timestamp
    );

    if (originalOldIndex === -1 || originalNewIndex === -1) {
      return;
    }

    await invoke('move_clipboard_item', {
      fromIndex: originalOldIndex,
      toIndex: originalNewIndex
    });

    await refreshClipboardHistory();

  } catch (error) {
    console.error('更新剪贴板顺序失败:', error);
    await refreshClipboardHistory();
  }
}

// 初始化虚拟列表
function initClipboardVirtualList() {

  if (clipboardVirtualList) {
    clipboardVirtualList.destroy();
  }

  clipboardVirtualList = new VirtualList({
    scrollId: 'clipboard-list',
    contentId: 'clipboard-content',
    data: getFilteredClipboardData(),
    renderItem: generateClipboardItemHTML,
    onSort: updateClipboardOrder,
    onItemClick: handleClipboardItemClick,
    onItemContextMenu: handleClipboardItemContextMenu,
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
  if (!window.clipboardModule) {
    window.clipboardModule = {};
  }
  window.clipboardModule.clipboardVirtualList = clipboardVirtualList;
}

// 获取过滤后的剪贴板数据
function getFilteredClipboardData() {
  const searchTerm = searchInput.value.toLowerCase();
  const filterType = currentFilter;

  return clipboardHistory.filter((item) => {
    // 使用新的数据结构判断类型
    const isImage = item.is_image || item.text.startsWith('data:image/') || item.text.startsWith('image:');
    const contentType = isImage ? 'image' : getContentType(item.text);

    // 类型筛选
    if (filterType !== 'all' && contentType !== filterType) {
      return false;
    }

    // 搜索过滤：支持文本、链接和文件类型
    if (searchTerm) {
      let shouldShow = false;

      if (contentType === 'files') {
        // 文件类型：搜索文件名和路径
        try {
          const filesJson = item.text.substring(6); // 去掉 "files:" 前缀
          const filesData = JSON.parse(filesJson);
          const searchableText = filesData.files.map(file =>
            `${file.name} ${file.path} ${file.file_type}`
          ).join(' ').toLowerCase();
          shouldShow = searchableText.includes(searchTerm);
        } catch (error) {
          shouldShow = false;
        }
      } else if (contentType === 'image') {
        // 图片类型：暂不支持搜索
        shouldShow = false;
      } else {
        // 文本和链接类型：搜索内容
        shouldShow = item.text.toLowerCase().includes(searchTerm);
      }

      return shouldShow;
    }

    return true;
  });
}

// 渲染剪贴板项目
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

  // 加载剪贴板图片
  const clipboardImages = document.querySelectorAll('.clipboard-image[data-needs-load="true"]');

  for (const img of clipboardImages) {
    const imageId = img.getAttribute('data-image-id');
    if (imageId) {
      try {
        await loadImageById(img, imageId, true); // 先加载缩略图
        img.removeAttribute('data-needs-load');
        img.removeAttribute('data-image-id');
      } catch (error) {
        console.warn('加载剪贴板图片失败:', error);
        img.alt = '图片加载失败';
        img.style.backgroundColor = '#e0e0e0';
      }
    }
  }
}

export function renderClipboardItems() {
  if (!clipboardVirtualList) {
    initClipboardVirtualList();
  } else {
    const filteredData = getFilteredClipboardData();
    clipboardVirtualList.updateData(filteredData);
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

// 处理剪贴板项目点击事件
function handleClipboardItemClick(index, event) {
  if (isDragging) return;

  // 获取过滤后的数据，因为虚拟列表使用的是过滤后的数据
  const filteredData = getFilteredClipboardData();
  const item = filteredData[index];
  if (!item) return;

  // 找到在原始数组中的索引
  const originalIndex = clipboardHistory.findIndex(originalItem => originalItem === item);
  if (originalIndex === -1) return;

  // 处理主要的点击事件（粘贴）
  const clipboardItem = event.target.closest('.clipboard-item');
  handleClipboardItemPaste(item, originalIndex, clipboardItem);
}

// 处理剪贴板项目右键菜单
function handleClipboardItemContextMenu(index, event) {
  event.preventDefault();

  // 获取过滤后的数据，因为虚拟列表使用的是过滤后的数据
  const filteredData = getFilteredClipboardData();
  const item = filteredData[index];
  if (!item) return;

  // 找到在原始数组中的索引
  const originalIndex = clipboardHistory.findIndex(originalItem => originalItem === item);
  if (originalIndex === -1) return;

  showClipboardContextMenu(event, item, originalIndex);
}

// 处理剪贴板项目粘贴
async function handleClipboardItemPaste(item, index, element = null) {
  try {
    // 检查是否需要AI翻译
    const contentType = getContentType(item.text);
    const isTextContent = contentType === 'text';
    const translationCheck = isTextContent ? shouldTranslateText(item.text, 'paste') : { should: false, reason: '非文本内容' };
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
      console.log('开始AI翻译:', item.text, '原因:', translationCheck.reason);
      showTranslationIndicator('正在翻译...');

      // 定义降级回调函数
      const fallbackPaste = async () => {
        const params = {
          content: item.text,
          one_time: false
        };
        await invoke('paste_content', { params });
      };

      try {
        const result = await safeTranslateAndInputText(item.text, fallbackPaste);

        setActiveItem(index);

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
      // 不需要翻译，直接粘贴
      const params = {
        content: item.text,
        one_time: false
      };
      await invoke('paste_content', { params });
      setActiveItem(index);

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
    console.error('粘贴失败:', error);
    hidePasteLoading(element, false, '粘贴失败');
    showNotification('粘贴失败', 'error');
  }
}



// 删除剪贴板项目
async function deleteClipboardItem(index) {
  try {
    await invoke('delete_clipboard_item', {
      id: index
    });
    // 刷新剪贴板历史
    await refreshClipboardHistory();
    showNotification('项目已删除', 'success');
  } catch (error) {
    console.error('删除剪贴板项目失败:', error);
    showNotification('删除失败', 'error');
  }
}

// 清空剪贴板历史
async function clearClipboardHistory() {
  const { showConfirmModal } = await import('./ui.js');

  showConfirmModal(
    '确认清空',
    '确定要清空所有剪贴板历史记录吗？此操作不可撤销。',
    async () => {
      try {
        await invoke('clear_clipboard_history');
        showNotification('已清空剪贴板历史', 'success');
        // 刷新列表
        await refreshClipboardHistory();
      } catch (error) {
        console.error('清空剪贴板历史失败:', error);
        showNotification('清空失败', 'error');
      }
    }
  );
}

// 显示剪贴板右键菜单
function showClipboardContextMenu(event, item, index) {
  const menuItems = [];

  // 检查内容类型
  const isImage = item.is_image || item.text.startsWith('data:image/') || item.text.startsWith('image:');
  const contentType = isImage ? 'image' : getContentType(item.text);

  // 根据内容类型添加特有菜单项
  if (contentType === 'image') {
    // 图片类型菜单
    menuItems.push(
      {
        icon: 'ti-eye',
        text: '查看原图',
        onClick: () => {
          viewOriginalImageFromClipboard(item);
        }
      },
      {
        icon: 'ti-download',
        text: '另存为图片',
        onClick: () => {
          saveImageAsFromClipboard(item);
        }
      }
    );
  } else if (contentType === 'files') {
    // 文件类型菜单
    menuItems.push(
      {
        icon: 'ti-external-link',
        text: '使用默认程序打开',
        onClick: () => {
          openFileWithDefaultProgram(item);
        }
      },
      {
        icon: 'ti-folder-open',
        text: '打开文件位置',
        onClick: () => {
          openFileLocationFromClipboard(item);
        }
      },
      {
        icon: 'ti-copy',
        text: '复制文件路径',
        onClick: () => {
          copyFilePathsFromClipboard(item);
        }
      }
    );
  } else if (contentType === 'text' || contentType === 'link') {
    // 文本和链接类型菜单
    menuItems.push({
      icon: 'ti-edit',
      text: '编辑',
      onClick: async () => {
        await openTextEditor(item, index);
      }
    });
  }

  // 通用菜单项
  menuItems.push(
    {
      icon: 'ti-star',
      text: '添加到常用文本',
      onClick: async () => {
        try {
          await invoke('add_clipboard_to_favorites', { index });
          showNotification('已添加到常用文本', 'success');

          // 触发常用文本列表刷新
          await invoke('emit_quick_texts_updated');
        } catch (error) {
          console.error('添加到常用文本失败:', error);
          showNotification('添加失败', 'error');
        }
      }
    },
    {
      icon: 'ti-trash',
      text: '删除当前项',
      onClick: async () => {
        await deleteClipboardItem(index);
      }
    },
    { type: 'separator' },
    {
      icon: 'ti-trash-x',
      text: '清空剪贴板',
      style: { color: '#ff4d4f' },
      onClick: async () => {
        await clearClipboardHistory();
      }
    }
  );

  showContextMenu(event, {
    content: item.text,
    items: menuItems
  });
}

// 打开文本编辑器
async function openTextEditor(item, index) {
  try {
    // 打开文本编辑窗口
    await invoke('open_text_editor_window');

    // 准备编辑数据
    const editorData = {
      index: index,
      content: item.text,
      title: `剪贴板项目 #${index + 1}`,
      timestamp: item.timestamp
    };

    // 延迟发送数据，确保窗口已完全加载
    setTimeout(async () => {
      try {
        // 获取编辑器窗口并发送数据
        const { emit } = await import('@tauri-apps/api/event');
        await emit('editor-data', editorData);
        console.log('已发送编辑数据到文本编辑器');
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



// 根据图片ID加载图片
export async function loadImageById(imgElement, imageId, useThumbnail = true) {
  try {
    const cacheKey = `${imageId}_${useThumbnail ? 'thumb' : 'full'}`;
    const cache = useThumbnail ? thumbnailCache : imageCache;

    // 检查缓存
    if (cache.has(cacheKey)) {
      imgElement.src = cache.get(cacheKey);
      return;
    }

    // 从后端获取图片数据
    const command = useThumbnail ? 'get_image_thumbnail_url' : 'get_image_data_url';
    const dataUrl = await invoke(command, { imageId });

    // 缓存图片数据
    cache.set(cacheKey, dataUrl);

    // 设置图片源
    imgElement.src = dataUrl;

    // 如果是缩略图，添加点击事件加载完整图片
    if (useThumbnail) {
      imgElement.style.cursor = 'pointer';
      imgElement.addEventListener('click', async (e) => {
        e.stopPropagation(); // 防止触发父元素的点击事件
        await loadImageById(imgElement, imageId, false);
      });
    }

  } catch (error) {
    console.error('加载图片失败:', error);
    imgElement.alt = '图片加载失败';
    imgElement.style.backgroundColor = '#ffebee';
    imgElement.style.color = '#c62828';
    imgElement.textContent = '图片加载失败';
  }
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

// 从剪贴板查看原图
function viewOriginalImageFromClipboard(item) {
  try {
    if (item.text.startsWith('image:')) {
      // 新格式：image:id，需要通过后端获取完整图片
      const imageId = item.text.substring(6);
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
    } else if (item.text.startsWith('data:image/')) {
      // 旧格式：完整的data URL
      const newWindow = window.open('', '_blank');
      newWindow.document.write(`
        <html>
          <head><title>查看原图</title></head>
          <body style="margin:0;padding:20px;background:#000;display:flex;justify-content:center;align-items:center;min-height:100vh;">
            <img src="${item.text}" style="max-width:100%;max-height:100%;object-fit:contain;" alt="原图" />
          </body>
        </html>
      `);
    }
  } catch (error) {
    console.error('查看原图失败:', error);
    showNotification('查看原图失败', 'error');
  }
}

// 从剪贴板另存为图片
async function saveImageAsFromClipboard(item) {
  try {
    if (!item.text.startsWith('data:image/') && !item.text.startsWith('image:')) {
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
      content: item.text,
      filePath: filePath
    });

    showNotification('图片已保存', 'success');
  } catch (error) {
    console.error('保存图片失败:', error);
    showNotification('保存图片失败', 'error');
  }
}

// 使用默认程序打开文件
async function openFileWithDefaultProgram(item) {
  try {
    const filesJson = item.text.substring(6); // 去掉 "files:" 前缀
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

// 从剪贴板打开文件位置
async function openFileLocationFromClipboard(item) {
  try {
    const filesJson = item.text.substring(6); // 去掉 "files:" 前缀
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

// 从剪贴板复制文件路径
async function copyFilePathsFromClipboard(item) {
  try {
    const filesJson = item.text.substring(6); // 去掉 "files:" 前缀
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
