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
import {
  setFileIcon,
  isImageFile,
  createFileIconElement
} from './fileIconUtils.js';
import { ClipboardVirtualScroll } from './virtualScrollAdapter.js';

// 图片缓存
const imageCache = new Map();
const thumbnailCache = new Map();

// 虚拟滚动实例
let clipboardVirtualScroll = null;

// 初始化虚拟滚动
export function initClipboardVirtualScroll() {
  if (!clipboardList) {
    console.error('剪贴板列表容器不存在');
    return;
  }

  // 销毁现有实例
  if (clipboardVirtualScroll) {
    clipboardVirtualScroll.destroy();
  }

  // 创建新的虚拟滚动实例
  clipboardVirtualScroll = new ClipboardVirtualScroll(clipboardList, {
    estimatedItemHeight: 80,
    overscan: 3,
    onItemClick: handleClipboardItemClick,
    onItemContextMenu: showClipboardContextMenu
  });

  // 设置初始数据
  if (clipboardHistory.length > 0) {
    clipboardVirtualScroll.setOriginalData(clipboardHistory);
  }
}

// 处理剪贴板项目点击
async function handleClipboardItemClick(item, index, event) {
  if (isDragging) return;

  try {
    // 设置活动项目索引
    setActiveItemIndex(index);

    // 检查内容类型
    const isImage = item.is_image || item.text.startsWith('data:image/') || item.text.startsWith('image:');
    const contentType = isImage ? 'image' : getContentType(item.text);
    const isFiles = contentType === 'files';
    const isText = contentType === 'text';

    // 对于文本内容，检查是否需要翻译
    if (isText) {
      const translationCheck = shouldTranslateText(item.text, 'paste');
      const needsTranslation = translationCheck.should;

      if (needsTranslation) {
        console.log('开始剪贴板AI翻译:', item.text, '原因:', translationCheck.reason);
        showTranslationIndicator('正在翻译...');

        const fallbackPaste = async () => {
          await invoke('paste_content', {
            params: {
              content: item.text,
              content_type: 'text'
            }
          });
        };

        await safeTranslateAndInputText(item.text, fallbackPaste);
        hideTranslationIndicator();
        return;
      }
    }

    // 直接粘贴内容
    await invoke('paste_content', {
      params: {
        content: item.text,
        content_type: contentType
      }
    });

    showNotification('已粘贴到剪贴板', 'success');
  } catch (error) {
    console.error('粘贴失败:', error);
    showNotification('粘贴失败', 'error');
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

// 渲染剪贴板项目（使用虚拟滚动）
export function renderClipboardItems() {
  // 如果虚拟滚动未初始化，先初始化
  if (!clipboardVirtualScroll) {
    initClipboardVirtualScroll();
  }

  // 更新数据
  if (clipboardVirtualScroll) {
    clipboardVirtualScroll.setOriginalData(clipboardHistory);

    // 应用当前的搜索和筛选
    const searchTerm = searchInput.value.toLowerCase();
    const filterType = currentFilter;
    clipboardVirtualScroll.setFilter(searchTerm, filterType);
  }

  // 通知导航系统列表已更新
  import('./navigation.js').then(module => {
    module.onListUpdate();
  }).catch(() => { });
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

// 创建文件元素
function createFilesElement(container, item) {
  try {
    // 解析文件数据
    const filesData = JSON.parse(item.text.substring(6)); // 去掉 "files:" 前缀

    const filesContainer = document.createElement('div');
    filesContainer.className = 'clipboard-files';

    // 创建文件列表
    filesData.files.forEach((file) => {
      const fileItem = document.createElement('div');
      fileItem.className = 'file-item';

      // 文件图标 - 使用新的工具函数
      const iconElement = createFileIconElement(file, 'large');

      // 文件信息
      const infoElement = document.createElement('div');
      infoElement.className = 'file-info';

      const nameElement = document.createElement('div');
      nameElement.className = 'file-name';
      nameElement.textContent = file.name;
      nameElement.title = file.path;

      const detailsElement = document.createElement('div');
      detailsElement.className = 'file-details';
      detailsElement.textContent = `${file.file_type} • ${formatFileSize(file.size)}`;

      infoElement.appendChild(nameElement);
      infoElement.appendChild(detailsElement);

      fileItem.appendChild(iconElement);
      fileItem.appendChild(infoElement);

      filesContainer.appendChild(fileItem);
    });

    // 文件容器不需要特殊的点击事件处理，让事件冒泡到父级 clipboard-item
    container.appendChild(filesContainer);

  } catch (error) {
    console.error('解析文件数据失败:', error);
    const errorElement = document.createElement('div');
    errorElement.className = 'clipboard-text';
    errorElement.textContent = '文件数据解析失败';
    errorElement.style.color = '#c62828';
    container.appendChild(errorElement);
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
function saveImageAsFromClipboard(item) {
  try {
    if (item.text.startsWith('data:image/')) {
      // 创建下载链接
      const link = document.createElement('a');
      link.href = item.text;
      link.download = `image_${Date.now()}.png`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      showNotification('图片已保存', 'success');
    } else {
      showNotification('此图片格式暂不支持直接保存', 'info');
    }
  } catch (error) {
    console.error('保存图片失败:', error);
    showNotification('保存图片失败', 'error');
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
