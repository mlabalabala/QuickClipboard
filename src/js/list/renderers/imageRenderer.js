// 图片渲染器模块 - 提供图片HTML生成的共享逻辑

import { escapeHtml } from '../../utils/formatters.js';

// 生成图片HTML
export function generateImageHTML(item, options = {}) {
  const {
    showTitle = false,
    cssClass = 'clipboard-image'
  } = options;

  // 为图片元素生成唯一ID，用于后续异步加载
  const imgId = `img-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  const placeholderSrc = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgZmlsbD0iI2YwZjBmMCIvPjwvc3ZnPg==';

  let imageHTML = '';

  // 处理不同的图片内容格式
  if (item.image_id) {
    // 使用image_id字段（剪贴板项）
    imageHTML = `<img id="${imgId}" class="${cssClass} lazy image-loading" src="${placeholderSrc}" alt="图片" data-image-id="${item.image_id}" decoding="async">`;
  } else if (item.content && item.content.startsWith('image:')) {
    // 从content中提取image_id（常用文本项）
    const imageId = item.content.substring(6);
    imageHTML = `<img id="${imgId}" class="${cssClass} lazy image-loading" src="${placeholderSrc}" alt="图片" data-image-id="${imageId}" decoding="async">`;
  } else if (item.content && item.content.startsWith('data:image/')) {
    // 旧格式的完整图片数据
    imageHTML = `<img class="${cssClass}" src="${item.content}" alt="图片" decoding="async">`;
  } else {
    // 未知格式，显示占位符
    imageHTML = `<div class="${cssClass}" style="background-color: #e0e0e0; display: flex; align-items: center; justify-content: center; color: #666;">图片加载失败</div>`;
  }

  // 如果需要显示标题（常用文本）
  if (showTitle && item.title) {
    return `
      <div class="quick-text-title">${escapeHtml(item.title)}</div>
      ${imageHTML}
    `;
  }

  return imageHTML;
}

// 异步加载图片
export async function loadImageById(imgElement, imageId) {
  try {
    const { invoke, convertFileSrc } = await import('@tauri-apps/api/core');
    const filePath = await invoke('get_image_file_path', { content: `image:${imageId}` });
    const assetUrl = convertFileSrc(filePath, 'asset');
    imgElement.src = assetUrl;
    imgElement.classList.remove('image-loading');
  } catch (error) {
    console.error('加载图片失败:', error);
    imgElement.alt = '图片加载失败';
    imgElement.style.backgroundColor = '#ffebee';
    imgElement.style.color = '#c62828';
    imgElement.textContent = '图片加载失败';
    imgElement.classList.remove('image-loading');
    imgElement.classList.add('image-error');
  }
}

