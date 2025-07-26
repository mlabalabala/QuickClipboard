import { invoke } from '@tauri-apps/api/core';
import {
  clipboardHistory,
  setClipboardHistory,
  activeItemIndex,
  setActiveItemIndex,
  isDragging,
  currentFilter,
  searchInput,
  clipboardList
} from './config.js';
import { showNotification } from './ui.js';
import { showContextMenu } from './contextMenu.js';
import {
  shouldTranslateText,
  safeTranslateAndInputText,
  showTranslationIndicator,
  hideTranslationIndicator
} from './aiTranslation.js';

// 图片缓存
const imageCache = new Map();
const thumbnailCache = new Map();

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
      // 获取历史记录（不需要调用 refresh_clipboard，因为后台监听器已经处理了）
      const history = await invoke('get_clipboard_history');

      // 如果历史记录有变化，更新UI
      if (JSON.stringify(history) !== JSON.stringify(clipboardHistory)) {
        setClipboardHistory(history);
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

// 复制到剪贴板（文本或图片）
export async function copyToClipboard(item) {
  try {
    const contentType = getContentType(item.text);
    if (contentType === 'image') {
      // 处理图片内容
      if (item.text.startsWith('image:')) {
        // 新格式：image:id，需要先获取完整的data URL
        const imageId = item.text.substring(6); // 去掉 "image:" 前缀
        try {
          const dataUrl = await invoke('get_image_data_url', { imageId });
          await writeClipboardImage(dataUrl);
        } catch (error) {
          console.error('获取图片数据失败:', error);
          // 回退到直接使用引用格式
          await writeClipboardText(item.text);
        }
      } else if (item.text.startsWith('data:image/')) {
        // 旧格式：完整的data URL
        await writeClipboardImage(item.text);
      } else {
        // 未知格式，当作文本处理
        await writeClipboardText(item.text);
      }
      showNotification('已复制图片', 'success', 2000);
    } else {
      await writeClipboardText(item.text);
      showNotification('已复制文本', 'success', 2000);
    }
    refreshClipboardHistory();
  } catch (error) {
    console.error('复制到剪贴板失败:', error);
    showNotification('复制失败，请重试', 'error');
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
    // 获取当前显示的项目（考虑搜索过滤）
    const searchTerm = searchInput.value.toLowerCase();
    let visibleItems = [];

    if (searchTerm) {
      // 如果有搜索，只处理可见的项目
      visibleItems = clipboardHistory.filter(item => {
        const isImage = item.text.startsWith('data:image/');
        if (isImage) return false;
        return item.text.toLowerCase().includes(searchTerm);
      });
    } else {
      // 没有搜索，处理所有项目
      visibleItems = [...clipboardHistory];
    }

    // 重新排列可见项目
    const [movedItem] = visibleItems.splice(oldIndex, 1);
    visibleItems.splice(newIndex, 0, movedItem);

    if (searchTerm) {
      // 如果有搜索过滤，需要将重新排序的结果合并回完整列表
      let filteredIndex = 0;
      const newHistory = clipboardHistory.map(item => {
        const isImage = item.text.startsWith('data:image/');
        if (isImage || !item.text.toLowerCase().includes(searchTerm)) {
          return item;
        } else {
          return visibleItems[filteredIndex++];
        }
      });
      setClipboardHistory(newHistory);
    } else {
      // 没有搜索过滤，直接使用重新排序的结果
      setClipboardHistory(visibleItems);
    }

    // 调用后端更新顺序
    await invoke('reorder_clipboard_history', {
      items: clipboardHistory.map(item => item.text)
    });

    // 重新渲染列表
    renderClipboardItems();
  } catch (error) {
    console.error('更新剪贴板顺序失败:', error);
    // 如果更新失败，重新获取历史记录
    await refreshClipboardHistory();
  }
}

// 渲染剪贴板项目
export function renderClipboardItems() {
  // 清空列表
  clipboardList.innerHTML = '';

  // 获取搜索关键词和筛选类型
  const searchTerm = searchInput.value.toLowerCase();
  const filterType = currentFilter;

  // 过滤并渲染项目
  clipboardHistory.forEach((item, index) => {
    // 使用新的数据结构判断类型
    const isImage = item.is_image || item.text.startsWith('data:image/') || item.text.startsWith('image:');
    const contentType = isImage ? 'image' : getContentType(item.text);

    // 类型筛选
    if (filterType !== 'all' && contentType !== filterType) {
      return;
    }

    // 搜索过滤：仅匹配文本和链接条目
    if (searchTerm) {
      if (isImage) return;
      if (!item.text.toLowerCase().includes(searchTerm)) {
        return;
      }
    }

    // 创建项目元素
    const clipboardItem = document.createElement('div');
    clipboardItem.className = 'clipboard-item';
    if (index === activeItemIndex) {
      clipboardItem.classList.add('active');
    }

    // 创建内容
    if (isImage) {
      createImageElement(clipboardItem, item);
    } else {
      const textElement = document.createElement('div');
      textElement.className = 'clipboard-text';
      textElement.textContent = item.text;
      clipboardItem.appendChild(textElement);
    }

    // 创建索引标签
    if (index < 9) {
      const indexElement = document.createElement('div');
      indexElement.className = 'clipboard-index';
      indexElement.textContent = `Ctrl+${index + 1}`;
      clipboardItem.appendChild(indexElement);
    }

    // 创建操作按钮容器（所有内容都支持添加到常用）
    const actionsElement = document.createElement('div');
    actionsElement.className = 'clipboard-actions';

    // 如果是链接类型，添加打开链接按钮
    if (getContentType(item.text) === 'link') {
      const openLinkButton = document.createElement('button');
      openLinkButton.className = 'action-button open-link';
      openLinkButton.innerHTML = '<i class="ti ti-external-link"></i>';
      openLinkButton.title = '在浏览器中打开';
      openLinkButton.style.display = 'none';
      openLinkButton.addEventListener('click', async (e) => {
        e.stopPropagation();
        await openLink(item.text);
      });
      actionsElement.appendChild(openLinkButton);
    }

    // 添加到常用按钮
    const addToFavoritesButton = document.createElement('button');
    addToFavoritesButton.className = 'action-button add-to-favorites';
    addToFavoritesButton.innerHTML = '<svg  xmlns="http://www.w3.org/2000/svg"  width="24"  height="24"  viewBox="0 0 24 24"  fill="none"  stroke="currentColor"  stroke-width="2"  stroke-linecap="round"  stroke-linejoin="round"  class="icon icon-tabler icons-tabler-outline icon-tabler-star"><path stroke="none" d="M0 0h24v24H0z" fill="none"/><path d="M12 17.75l-6.172 3.245l1.179 -6.873l-5 -4.867l6.9 -1l3.086 -6.253l3.086 6.253l6.9 1l-5 4.867l1.179 6.873z" /></svg>';
    addToFavoritesButton.title = '添加到常用';
    addToFavoritesButton.style.display = 'none';
    // 阻止事件冒泡，避免触发剪贴板项的点击事件
    addToFavoritesButton.addEventListener('click', async (e) => {
      e.stopPropagation();
      try {
        await addClipboardToFavorites(index);
        // 通知外部刷新常用文本列表
        window.dispatchEvent(new CustomEvent('refreshQuickTexts'));
      } catch (error) {
        console.error('添加到常用失败:', error);
      }
    });

    actionsElement.appendChild(addToFavoritesButton);
    clipboardItem.appendChild(actionsElement);

    // 设置拖拽属性
    clipboardItem.draggable = true;
    clipboardItem.addEventListener('dragstart', (e) => {
      const dragData = JSON.stringify({
        type: 'clipboard',
        index: index,
        text: item.text
      });

      // 使用自定义MIME类型存储完整数据（用于内部拖拽）
      e.dataTransfer.setData('application/x-quickclipboard', dragData);

      // 对于外部拖拽，只设置纯文本内容（不包含快捷键提示等）
      e.dataTransfer.setData('text/plain', item.text);

      // 设置拖拽效果
      e.dataTransfer.effectAllowed = 'move';

      // 添加拖拽状态类
      document.querySelector('.tab-content.active').classList.add('dragging');
    });

    clipboardItem.addEventListener('dragend', () => {
      // 移除拖拽状态类
      document.querySelector('.tab-content.active').classList.remove('dragging');
    });

    // 添加点击事件（单击即粘贴）
    clipboardItem.addEventListener('click', async () => {
      // 如果正在拖拽，不执行点击事件
      if (isDragging) return;

      // 检查是否正在处理中
      if (clipboardItem.classList.contains('processing')) {
        return;
      }

      try {
        // 检查是否需要AI翻译
        const contentType = getContentType(item.text);
        const isTextContent = contentType === 'text';
        const translationCheck = isTextContent ? shouldTranslateText(item.text, 'paste') : { should: false, reason: '非文本内容' };
        const needsTranslation = translationCheck.should;

        if (needsTranslation) {
          // 使用AI翻译并流式输入
          console.log('开始AI翻译:', item.text, '原因:', translationCheck.reason);
          showTranslationIndicator('正在翻译...');

          // 定义降级回调函数
          const fallbackPaste = async () => {
            const params = {
              index,
              one_time: false
            };
            await invoke('paste_history_item', { params });
          };

          try {
            const result = await safeTranslateAndInputText(item.text, fallbackPaste);

            setActiveItem(index);

            if (result.success) {
              if (result.method === 'translation') {
                console.log('AI翻译成功完成');
              } else if (result.method === 'fallback') {
                // 降级处理的通知已在aiTranslation.js中处理
                console.log('使用降级处理完成粘贴:', result.error);
              }

              // 翻译完成后隐藏窗口（如果需要）
              if (!window.isPinned) {
                await invoke('hide_main_window_if_auto_shown');
              }
            } else {
              showNotification(`翻译和粘贴都失败了: ${result.error}`, 'error');
            }
          } finally {
            hideTranslationIndicator();
          }
        } else {
          // 原始粘贴逻辑
          // 如果是图片，显示加载状态
          const isImage = contentType === 'image';
          if (isImage) {
            clipboardItem.classList.add('processing');
            const loadingIndicator = document.createElement('div');
            loadingIndicator.className = 'loading-indicator';
            loadingIndicator.innerHTML = '<div class="spinner"></div><span>准备中...</span>';
            clipboardItem.appendChild(loadingIndicator);
          }

          const params = {
            index,
            one_time: false // 剪贴板历史不支持一次性粘贴
          };
          await invoke('paste_history_item', { params });
          setActiveItem(index);
          // 粘贴逻辑已在Rust端处理窗口显示/隐藏
        }
      } catch (error) {
        console.error('粘贴剪贴板内容失败:', error);
        // 显示错误提示
        showErrorToast('粘贴失败: ' + error);
        hideTranslationIndicator();
      } finally {
        // 清理加载状态
        clipboardItem.classList.remove('processing');
        const loadingIndicator = clipboardItem.querySelector('.loading-indicator');
        if (loadingIndicator) {
          loadingIndicator.remove();
        }
      }
    });

    // 添加双击事件
    clipboardItem.addEventListener('dblclick', async () => {
      copyToClipboard(item);
      await invoke('toggle_window_visibility');
    });

    // 添加右键菜单（所有类型）
    clipboardItem.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      showClipboardContextMenu(e, item, index);
    });

    // 组装元素
    clipboardList.appendChild(clipboardItem);
  });

  // 如果没有项目，显示提示
  if (clipboardList.children.length === 0) {
    const emptyMessage = document.createElement('div');
    emptyMessage.textContent = searchTerm ? '没有匹配的剪贴板内容' : '剪贴板历史为空';
    emptyMessage.style.padding = '20px';
    emptyMessage.style.textAlign = 'center';
    emptyMessage.style.color = '#999';
    clipboardList.appendChild(emptyMessage);
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

  // 检查是否为文本类型（非图片）
  const isImage = item.is_image || item.text.startsWith('data:image/') || item.text.startsWith('image:');
  const contentType = isImage ? 'image' : getContentType(item.text);

  // 只对文本类型显示编辑选项
  if (contentType === 'text' || contentType === 'link') {
    menuItems.push({
      icon: 'ti-edit',
      text: '编辑',
      onClick: async () => {
        await openTextEditor(item, index);
      }
    });
  }

  menuItems.push(
    {
      icon: 'ti-star',
      text: '添加到常用文本',
      onClick: async () => {
        try {
          await invoke('add_clipboard_to_favorites', { index });
          showNotification('已添加到常用文本', 'success');
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

// 创建图片元素（支持延迟加载）
function createImageElement(container, item) {
  const imgElement = document.createElement('img');
  imgElement.className = 'clipboard-image';

  // 添加加载状态
  imgElement.style.pointerEvents = 'none';
  imgElement.style.backgroundColor = '#f0f0f0';
  imgElement.style.minHeight = '100px';
  imgElement.style.display = 'flex';
  imgElement.style.alignItems = 'center';
  imgElement.style.justifyContent = 'center';

  // 如果是新格式的图片引用
  if (item.image_id) {
    loadImageById(imgElement, item.image_id, true); // 先加载缩略图
  } else if (item.text.startsWith('image:')) {
    // 从text中提取image_id
    const imageId = item.text.substring(6); // 去掉 "image:" 前缀
    loadImageById(imgElement, imageId, true);
  } else if (item.text.startsWith('data:image/')) {
    // 旧格式的完整图片数据
    imgElement.src = item.text;
  } else {
    // 未知格式，显示占位符
    imgElement.alt = '图片加载失败';
    imgElement.style.backgroundColor = '#e0e0e0';
  }

  container.appendChild(imgElement);
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

// 显示错误提示
function showErrorToast(message) {
  // 移除现有的错误提示
  const existingToast = document.querySelector('.error-toast');
  if (existingToast) {
    existingToast.remove();
  }

  // 创建新的错误提示
  const toast = document.createElement('div');
  toast.className = 'error-toast';
  toast.textContent = message;
  document.body.appendChild(toast);

  // 3秒后自动移除
  setTimeout(() => {
    if (toast.parentNode) {
      toast.remove();
    }
  }, 3000);
}
