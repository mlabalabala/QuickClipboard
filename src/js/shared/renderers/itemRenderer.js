// 列表项渲染器 - 统一的列表项HTML生成逻辑

import { formatTimestamp } from '../../utils/formatters.js';
import { generateImageHTML } from './imageRenderer.js';
import { generateFilesHTML } from './fileRenderer.js';
import { generateTextHTML } from './textRenderer.js';

// 生成列表项的内容HTML
export function generateItemContentHTML(item, options = {}) {
  const {
    showTitle = false,
    pasteWithFormat = false
  } = options;

  const contentType = item.content_type || 'text';

  // 根据内容类型生成对应的HTML
  if (contentType === 'image') {
    return generateImageHTML(item, {
      showTitle,
      cssClass: showTitle ? 'quick-text-image' : 'clipboard-image'
    });
  } else if (contentType === 'file') {
    return generateFilesHTML(item, {
      showTitle,
      showTime: showTitle  // 常用文本显示时间在文件列表内部
    });
  } else {
    // text, link, rich_text
    return generateTextHTML(item, {
      showTitle,
      cssClass: showTitle ? 'quick-text-content' : 'clipboard-text',
      pasteWithFormat
    });
  }
}

// 生成剪贴板列表项HTML
export function generateClipboardItemHTML(item, index, options = {}) {
  const {
    activeItemIndex = -1,
    isSearching = false,
    isFiltering = false,
    pasteWithFormat = false
  } = options;

  const contentType = item.content_type || 'text';

  // 生成内容HTML
  const contentHTML = generateItemContentHTML(item, {
    showTitle: false,
    pasteWithFormat
  });

  // 生成序号
  const numberHTML = `<div class="clipboard-number">${index + 1}</div>`;
  
  // 只在正常列表状态（非搜索、非筛选）下显示快捷键提示
  const shouldShowShortcut = !isSearching && !isFiltering && index < 9;
  const shortcutHTML = shouldShowShortcut ?
    `<div class="clipboard-index">Ctrl+${index + 1}</div>` : '';

  // 生成操作按钮
  const actionsHTML = '<div class="clipboard-actions"></div>';

  // 生成时间戳 - 对于文件类型，时间戳会在文件HTML内部显示
  const timeValue = item.created_at || item.timestamp;
  const timestampHTML = contentType === 'file' ? '' : 
    `<div class="clipboard-timestamp">${formatTimestamp(timeValue)}</div>`;

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

// 生成常用文本列表项HTML
export function generateQuickTextItemHTML(item, index, options = {}) {
  const {
    pasteWithFormat = false,
    groupBadgeHTML = ''
  } = options;

  const contentType = item.content_type || 'text';

  // 生成内容HTML
  const contentHTML = generateItemContentHTML(item, {
    showTitle: true,
    pasteWithFormat
  });

  // 生成时间戳 - 对于文件类型，时间戳会在文件HTML内部显示
  const timestampHTML = contentType === 'file' ? '' : 
    `<div class="quick-text-timestamp">${formatTimestamp(item.created_at)}</div>`;

  return `
    <div class="quick-text-item" draggable="true" data-index="${index}">
      ${timestampHTML}
      ${groupBadgeHTML}
      ${contentHTML}
    </div>
  `;
}

