// 文本渲染器模块 - 提供文本HTML生成的共享逻辑

import { escapeHtml } from '../../utils/formatters.js';
import { 
  highlightMultipleSearchTermsWithPosition, 
  highlightMultipleSearchTermsInHTML, 
  getCurrentSearchTerms 
} from '../../utils/highlight.js';
import { processHTMLImages } from '../../utils/htmlProcessor.js';
import { isLinkContent } from '../../utils/linkUtils.js';
import { detectColor, generateColorPreviewHTML } from '../../utils/colorUtils.js';

// 生成文本HTML
export function generateTextHTML(item, options = {}) {
  const {
    showTitle = false,
    cssClass = 'clipboard-text',
    pasteWithFormat = false
  } = options;

  const searchTerms = getCurrentSearchTerms();

  // 如果有HTML内容且开启格式显示，但不是纯链接内容
  if (item.html_content && pasteWithFormat && !isLinkContent(item)) {
    return generateRichTextHTML(item, { showTitle, searchTerms });
  }

  // 纯文本内容
  return generatePlainTextHTML(item, { showTitle, cssClass, searchTerms });
}

// 生成富文本HTML
function generateRichTextHTML(item, options = {}) {
  const { showTitle = false, searchTerms = [] } = options;

  let displayHTML = item.html_content;

  // 对HTML内容应用搜索高亮
  if (searchTerms.length > 0) {
    displayHTML = highlightMultipleSearchTermsInHTML(displayHTML, searchTerms);
  }

  // 处理HTML内容中的图片
  displayHTML = processHTMLImages(displayHTML);

  // 如果需要显示标题（常用文本）
  if (showTitle && item.title) {
    const titleResult = highlightMultipleSearchTermsWithPosition(item.title, searchTerms);
    return `
      <div class="quick-text-title">${titleResult.html}</div>
      <div class="quick-text-content quick-text-html"><div>${displayHTML}</div></div>
    `;
  }

  // 剪贴板项
  return `<div class="clipboard-text clipboard-html"><div>${displayHTML}</div></div>`;
}

// 生成纯文本HTML
function generatePlainTextHTML(item, options = {}) {
  const { 
    showTitle = false, 
    cssClass = 'clipboard-text',
    searchTerms = [] 
  } = options;

  // 检测是否为颜色值
  const colorInfo = detectColor(item.content);
  
  if (showTitle && item.title) {
    // 常用文本格式
    return generateQuickTextPlainHTML(item, { colorInfo, searchTerms });
  } else {
    // 剪贴板格式
    return generateClipboardPlainHTML(item, { cssClass, colorInfo, searchTerms });
  }
}

// 生成常用文本的纯文本HTML
function generateQuickTextPlainHTML(item, options = {}) {
  const { colorInfo, searchTerms = [] } = options;

  const titleResult = highlightMultipleSearchTermsWithPosition(item.title, searchTerms);
  
  let displayContent;
  let contentDataAttr = '';
  
  if (colorInfo) {
    // 是颜色值，生成颜色预览
    displayContent = generateColorPreviewHTML(colorInfo);
  } else {
    // 不是颜色值，正常处理高亮
    const contentResult = highlightMultipleSearchTermsWithPosition(item.content, searchTerms);
    displayContent = contentResult.html;
    
    // 如果有搜索关键字，添加滚动定位功能
    if (searchTerms.length > 0 && contentResult.firstKeywordPosition !== -1) {
      contentDataAttr = `data-first-keyword="${contentResult.firstKeywordPosition}"`;
    }
  }
  
  // 构建完整的 HTML
  const titleDataAttr = searchTerms.length > 0 && titleResult.firstKeywordPosition !== -1 
    ? `data-first-keyword="${titleResult.firstKeywordPosition}"` 
    : '';
  const titleClass = titleDataAttr ? 'quick-text-title searchable' : 'quick-text-title';
  const contentClass = contentDataAttr ? 'quick-text-content searchable' : 'quick-text-content';
  
  return `
    <div class="${titleClass}" ${titleDataAttr}>${titleResult.html}</div>
    <div class="${contentClass}" ${contentDataAttr}><div>${displayContent}</div></div>
  `;
}

// 生成剪贴板的纯文本HTML
function generateClipboardPlainHTML(item, options = {}) {
  const { cssClass, colorInfo, searchTerms = [] } = options;

  let displayText;
  let dataAttr = '';
  
  if (colorInfo) {
    // 是颜色值，生成颜色预览
    displayText = generateColorPreviewHTML(colorInfo);
  } else {
    // 不是颜色值，正常处理高亮
    const highlightResult = highlightMultipleSearchTermsWithPosition(item.content, searchTerms);
    displayText = highlightResult.html;
    
    // 如果有搜索关键字，添加滚动定位功能
    if (searchTerms.length > 0 && highlightResult.firstKeywordPosition !== -1) {
      dataAttr = `data-first-keyword="${highlightResult.firstKeywordPosition}"`;
    }
  }
  
  const className = dataAttr ? `${cssClass} searchable` : cssClass;
  return `<div class="${className}" ${dataAttr}><div>${displayText}</div></div>`;
}

