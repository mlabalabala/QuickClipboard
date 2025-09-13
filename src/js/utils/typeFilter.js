// 类型筛选工具函数
export function matchesFilter(contentType, filterType) {
  if (filterType === 'all') {
    return true;
  }
  
  // 将rich_text也归类到text筛选器中
  if (filterType === 'text' && (contentType === 'text' || contentType === 'rich_text')) {
    return true;
  }
  
  return contentType === filterType;
}

// 搜索内容匹配
export function matchesSearch(item, searchTerm, contentType) {
  if (!searchTerm) {
    return true;
  }

  const term = searchTerm.toLowerCase();
  
  if (contentType === 'file') {
    // 文件类型：搜索文件名和路径
    try {
      const filesJson = item.content.substring(6); // 去掉 "files:" 前缀
      const filesData = JSON.parse(filesJson);
      const searchableText = filesData.files.map(file =>
        `${file.name} ${file.path} ${file.file_type || ''}`
      ).join(' ').toLowerCase();
      
      // 对于常用文本，还要搜索标题
      if (item.title) {
        return item.title.toLowerCase().includes(term) || searchableText.includes(term);
      }
      return searchableText.includes(term);
    } catch (error) {
      return item.title ? item.title.toLowerCase().includes(term) : false;
    }
  } else if (contentType === 'image') {
    // 图片类型：只搜索标题（如果有）
    return item.title ? item.title.toLowerCase().includes(term) : false;
  } else {
    // 文本、富文本和链接类型：搜索内容和标题
    const contentMatch = item.content.toLowerCase().includes(term);
    const titleMatch = item.title ? item.title.toLowerCase().includes(term) : false;
    return contentMatch || titleMatch;
  }
}
