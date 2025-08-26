// HTML处理工具函数

// 安全处理HTML内容，防止链接跳转和页面劫持
export function processHTMLImages(htmlContent) {
  // 创建一个临时DOM来处理HTML
  const tempDiv = document.createElement('div');
  tempDiv.innerHTML = htmlContent;
  
  // 1. 移除所有危险的元素和属性
  sanitizeHTML(tempDiv);
  
  // 2. 处理图片元素
  const images = tempDiv.querySelectorAll('img');
  images.forEach((img, index) => {
    // 为每个图片添加唯一ID以便调试
    img.setAttribute('data-img-id', `html-img-${Date.now()}-${index}`);
    
    // 保存原始src用于调试
    const originalSrc = img.getAttribute('src');
    if (originalSrc) {
      img.setAttribute('data-original-src', originalSrc);
    }
    
    // 使用 inline onerror 以确保字符串注入后仍能生效
    img.setAttribute('onerror', 'this.onerror=null; window.handleHtmlImageError && window.handleHtmlImageError(this);');
    img.setAttribute('loading', 'lazy');
    
    // 设置基本样式
    img.style.maxWidth = '100%';
    img.style.height = 'auto';
    img.style.display = 'inline-block';
  });
  
  return tempDiv.innerHTML;
}

// HTML安全清理函数
function sanitizeHTML(element) {
  // 移除所有脚本标签
  const scripts = element.querySelectorAll('script');
  scripts.forEach(script => script.remove());
  
  // 移除所有链接的跳转功能
  const links = element.querySelectorAll('a');
  links.forEach(link => {
    // 移除href属性，保留样式
    link.removeAttribute('href');
    link.removeAttribute('target');
    link.style.cursor = 'default';
    link.style.textDecoration = 'none';
    // 防止点击事件
    link.setAttribute('onclick', 'return false;');
  });
  
  // 移除所有form表单
  const forms = element.querySelectorAll('form');
  forms.forEach(form => form.remove());
  
  // 移除所有iframe和embed
  const iframes = element.querySelectorAll('iframe, embed, object');
  iframes.forEach(frame => frame.remove());
  
  // 移除危险的事件属性
  const dangerousAttributes = [
    'onload', 'onclick', 'onmouseover', 'onmouseout', 'onfocus', 'onblur',
    'onchange', 'onsubmit', 'onreset', 'onkeydown', 'onkeyup', 'onkeypress',
    'onerror', 'onabort', 'oncanplay', 'oncanplaythrough', 'ondurationchange',
    'onemptied', 'onended', 'onloadeddata', 'onloadedmetadata', 'onloadstart',
    'onpause', 'onplay', 'onplaying', 'onprogress', 'onratechange', 'onseeked',
    'onseeking', 'onstalled', 'onsuspend', 'ontimeupdate', 'onvolumechange',
    'onwaiting', 'onanimationend', 'onanimationiteration', 'onanimationstart',
    'ontransitionend'
  ];
  
  // 递归处理所有元素
  function cleanElement(el) {
    // 移除危险属性
    dangerousAttributes.forEach(attr => {
      if (el.hasAttribute(attr)) {
        el.removeAttribute(attr);
      }
    });
    
    // 处理CSS类名，移除可能导致全局影响的类
    const className = el.getAttribute('class');
    if (className) {
      // 移除可能的Bootstrap或其他框架的浮动/定位类
      const cleanClassName = className
        .replace(/\b(fixed|sticky|absolute|float-\w+|position-\w+)\b/gi, '')
        .replace(/\s+/g, ' ')
        .trim();
      
      if (cleanClassName) {
        el.setAttribute('class', cleanClassName);
      } else {
        el.removeAttribute('class');
      }
    }
    
    // 处理style属性中的危险内容和浮动定位
    const style = el.getAttribute('style');
    if (style) {
      // 移除可能的javascript:前缀和expression()
      let cleanStyle = style
        .replace(/javascript:/gi, '')
        .replace(/expression\s*\(/gi, '')
        .replace(/url\s*\(\s*javascript:/gi, '');
      
      // 限制定位属性，防止元素脱离容器范围
      cleanStyle = cleanStyle
        .replace(/position\s*:\s*(fixed|sticky)/gi, 'position: relative')
        .replace(/position\s*:\s*absolute/gi, 'position: relative')
        // 移除可能导致溢出的属性
        .replace(/z-index\s*:\s*[^;]+/gi, '')
        .replace(/top\s*:\s*[^;]+/gi, '')
        .replace(/left\s*:\s*[^;]+/gi, '')
        .replace(/right\s*:\s*[^;]+/gi, '')
        .replace(/bottom\s*:\s*[^;]+/gi, '')
        // 限制浮动
        .replace(/float\s*:\s*[^;]+/gi, '');
      
      el.setAttribute('style', cleanStyle);
    }
    
    // 递归处理子元素
    Array.from(el.children).forEach(child => cleanElement(child));
  }
  
  cleanElement(element);
}

// 全局图片错误处理函数（如果还没有定义的话）
if (!window.handleImageError) {
  window.handleImageError = function(imgElement) {
    // 防止无限重试
    if (imgElement.hasAttribute('data-error-handled')) {
      return;
    }
    
    imgElement.setAttribute('data-error-handled', 'true');
    
    // 替换为占位图
    imgElement.src = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgZmlsbD0iI2VlZSIvPjx0ZXh0IHg9IjUwJSIgeT0iNTAlIiBmb250LXNpemU9IjEyIiBmaWxsPSIjOTk5IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBkeT0iLjNlbSI+5Zu+54mH5Yqg6L295aSx6LSlPC90ZXh0Pjwvc3ZnPg==';
    imgElement.alt = '图片加载失败';
    imgElement.title = '图片加载失败';
    
    // 可选：输出调试信息
    const imgId = imgElement.getAttribute('data-img-id');
    console.log(`Image load error for ${imgId}:`, imgElement.getAttribute('data-original-src') || 'unknown src');
  };
}

// 代理失败后回退处理
if (!window.handleHtmlImageError) {
  window.handleHtmlImageError = async function(imgElement) {
    try {
      const src = imgElement.getAttribute('src');
      if (src && /^https?:\/\//i.test(src) && !imgElement.hasAttribute('data-proxied')) {
        const { invoke } = await import('@tauri-apps/api/core');
        const dataUrl = await invoke('fetch_image_as_data_url', { url: src });
        imgElement.setAttribute('data-proxied', 'true');
        imgElement.src = dataUrl;
        return;
      }
    } catch (_) {}
    window.handleImageError(imgElement);
  };
}
