// 文件渲染器模块 - 提供文件HTML生成的共享逻辑

import { convertFileSrc } from '@tauri-apps/api/core';
import { getCurrentSettings } from '../../../settings/js/settingsManager.js';
import { escapeHtml, formatTimestamp } from '../../utils/formatters.js';

const IMAGE_FILE_EXTENSIONS = ['PNG', 'JPG', 'JPEG', 'GIF', 'BMP', 'WEBP', 'ICO'];

// 格式化文件大小
export function formatFileSize(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

// 生成文件图标HTML
export function generateFileIconHTML(file, size = 'medium') {
  const sizeMap = {
    small: '16px',
    medium: '20px',
    large: '24px'
  };

  const iconSize = sizeMap[size] || sizeMap.medium;
  const alt = file.file_type || '文件';

  // 默认占位图标
  const placeholderSrc = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQiIGhlaWdodD0iMjQiIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMyIgeT0iMyIgd2lkdGg9IjE4IiBoZWlnaHQ9IjE4IiBmaWxsPSIjQ0NDQ0NDIi8+Cjwvc3ZnPgo=';

  // 检查是否是图片文件且启用了预览
  const settings = getCurrentSettings();
  const isImageFile = IMAGE_FILE_EXTENSIONS.includes(file.file_type?.toUpperCase());
  
  if (isImageFile && settings.showImagePreview && file.path) {
    // 使用文件路径，启用懒加载
    const iconSrc = convertFileSrc(file.path, 'asset');
    const iconStyle = 'object-fit: cover; border-radius: 2px;';
    return `<img class="file-icon lazy image-loading" src="${placeholderSrc}" data-src="${iconSrc}" alt="${escapeHtml(alt)}" style="width: ${iconSize}; height: ${iconSize}; ${iconStyle}" decoding="async">`;
  } else if (file.icon_data) {
    // 使用图标数据（base64）
    const iconStyle = 'object-fit: contain; border-radius: 0;';
    return `<img class="file-icon" src="${file.icon_data}" alt="${escapeHtml(alt)}" style="width: ${iconSize}; height: ${iconSize}; ${iconStyle}">`;
  } else {
    // 使用默认图标
    const iconStyle = 'object-fit: contain; border-radius: 0;';
    return `<img class="file-icon" src="${placeholderSrc}" alt="${escapeHtml(alt)}" style="width: ${iconSize}; height: ${iconSize}; ${iconStyle}">`;
  }
}

// 生成文件列表HTML
export function generateFilesHTML(item, options = {}) {
  const {
    showTitle = false,
    showTime = false
  } = options;

  try {
    const filesJson = item.content.substring(6); // 去掉 "files:" 前缀
    const filesData = JSON.parse(filesJson);

    let filesHTML = '';

    // 如果需要显示标题（常用文本）
    if (showTitle && item.title) {
      filesHTML += `<div class="quick-text-title">${escapeHtml(item.title)}</div>`;
    }

    // 显示时间和文件数量摘要
    if (showTime || !showTitle) {
      // 格式化时间 - 优先使用created_at
      const timeValue = item.created_at || item.timestamp;
      const timeStr = formatTimestamp(timeValue);
      filesHTML += `<div class="file-summary">${timeStr} • ${filesData.files.length} 个文件</div>`;
    } else if (showTitle) {
      // 只显示文件数量
      filesHTML += `<div class="file-summary">${filesData.files.length} 个文件</div>`;
    }

    // 文件列表容器
    const containerClass = showTitle ? 'files-container' : 'clipboard-files';
    filesHTML += `<div class="${containerClass}">`;
    filesHTML += '<div class="clipboard-files-inner">';

    // 生成每个文件的HTML
    filesData.files.forEach(file => {
      const iconHTML = generateFileIconHTML(file, 'medium');
      const fileSize = formatFileSize(file.size || 0);
      filesHTML += `
        <div class="file-item" data-path="${escapeHtml(file.path)}">
          ${iconHTML}
          <div class="file-info">
            <div class="file-name">${escapeHtml(file.name)} <span class="file-size">${fileSize}</span></div>
            <div class="file-path">${escapeHtml(file.path)}</div>
          </div>
        </div>
      `;
    });

    filesHTML += '</div>';
    filesHTML += '</div>';
    return filesHTML;
  } catch (error) {
    console.error('解析文件数据失败:', error);
    const errorContent = showTitle && item.title 
      ? `<div class="quick-text-title">${escapeHtml(item.title)}</div><div class="quick-text-content">文件数据解析错误</div>`
      : `<div class="clipboard-text">文件数据解析错误</div>`;
    return errorContent;
  }
}

// 检查文件列表中是否包含图片文件
export function checkIfHasImageFile(item) {
  try {
    const filesJson = item.content.substring(6); 
    const filesData = JSON.parse(filesJson);

    if (filesData.files && filesData.files.length > 0) {
      return filesData.files.some(file => 
        IMAGE_FILE_EXTENSIONS.includes(file.file_type?.toUpperCase())
      );
    }
  } catch (error) {
    console.error('检查图片文件失败:', error);
  }
  return false;
}

// 获取文件列表中的第一个图片文件
export function getFirstImageFile(item) {
  try {
    const filesJson = item.content.substring(6);
    const filesData = JSON.parse(filesJson);

    if (filesData.files && filesData.files.length > 0) {
      return filesData.files.find(file => 
        IMAGE_FILE_EXTENSIONS.includes(file.file_type?.toUpperCase())
      );
    }
  } catch (error) {
    console.error('获取图片文件失败:', error);
  }
  return null;
}

// 获取文件列表数据
export function getFilesData(item) {
  try {
    const filesJson = item.content.substring(6);
    return JSON.parse(filesJson);
  } catch (error) {
    console.error('解析文件数据失败:', error);
    return null;
  }
}

