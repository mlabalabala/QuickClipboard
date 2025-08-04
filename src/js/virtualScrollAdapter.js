/**
 * 虚拟滚动适配器
 * 为剪贴板和常用文本列表提供虚拟滚动功能
 */
import { VirtualScroll } from './virtualScroll.js';
import { loadImageById } from './clipboard.js';

export class ClipboardVirtualScroll extends VirtualScroll {
  constructor(container, options = {}) {
    const defaultOptions = {
      estimatedItemHeight: 80,
      overscan: 3,
      ...options
    };

    super(container, defaultOptions);

    this.searchTerm = '';
    this.filterType = 'all';
    this.onItemContextMenu = options.onItemContextMenu || (() => { });
    this.onItemDelete = options.onItemDelete || (() => { });

    this.renderItem = this.renderClipboardItem.bind(this);
  }

  setFilter(searchTerm, filterType) {
    this.searchTerm = searchTerm;
    this.filterType = filterType;
    this.applyFilter();
  }

  applyFilter() {
    if (!this.originalData) {
      this.originalData = [...this.data];
    }

    let filteredData = this.originalData;

    if (this.filterType !== 'all') {
      filteredData = filteredData.filter(item => {
        const isImage = item.is_image || item.text.startsWith('data:image/') || item.text.startsWith('image:');
        const contentType = isImage ? 'image' : this.getContentType(item.text);
        return contentType === this.filterType;
      });
    }

    if (this.searchTerm) {
      const searchLower = this.searchTerm.toLowerCase();
      filteredData = filteredData.filter(item => {
        return item.text.toLowerCase().includes(searchLower);
      });
    }

    this.setData(filteredData);
  }

  getContentType(text) {
    if (text.startsWith('http://') || text.startsWith('https://')) {
      return 'link';
    }
    if (text.startsWith('files:')) {
      return 'files';
    }
    return 'text';
  }

  setOriginalData(data) {
    this.originalData = [...data];
    this.applyFilter();
  }

  renderClipboardItem(item, index) {
    const clipboardItem = document.createElement('div');
    clipboardItem.className = 'clipboard-item';
    clipboardItem.dataset.index = index;

    // 创建内容容器
    const contentContainer = document.createElement('div');
    contentContainer.className = 'clipboard-content';

    // 根据内容类型渲染不同内容
    const isImage = item.is_image || item.text.startsWith('data:image/') || item.text.startsWith('image:');

    if (isImage) {
      this.renderImageContent(contentContainer, item);
    } else if (item.text.startsWith('files:')) {
      this.renderFilesContent(contentContainer, item);
    } else {
      this.renderTextContent(contentContainer, item);
    }

    clipboardItem.appendChild(contentContainer);

    const numberElement = document.createElement('div');
    numberElement.className = 'clipboard-number';
    numberElement.textContent = index + 1;
    clipboardItem.appendChild(numberElement);

    if (index < 9) {
      const indexElement = document.createElement('div');
      indexElement.className = 'clipboard-index';
      indexElement.textContent = `Ctrl+${index + 1}`;
      clipboardItem.appendChild(indexElement);
    } else {
      clipboardItem.classList.add('no-shortcut');
    }

    clipboardItem.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      this.onItemContextMenu(e, item, index);
    });

    return clipboardItem;
  }

  renderImageContent(container, item) {
    const imgElement = document.createElement('img');
    imgElement.className = 'clipboard-image';
    imgElement.draggable = false;

    if (item.text.startsWith('image:')) {
      const imageId = item.text.substring(6);
      loadImageById(imgElement, imageId, true);
    } else if (item.text.startsWith('data:image/')) {
      imgElement.src = item.text;
    } else {
      imgElement.alt = '图片加载失败';
      imgElement.style.backgroundColor = '#e0e0e0';
    }

    imgElement.onerror = () => {
      imgElement.style.display = 'none';
      const errorDiv = document.createElement('div');
      errorDiv.textContent = '图片加载失败';
      errorDiv.style.color = '#999';
      errorDiv.style.padding = '20px';
      errorDiv.style.textAlign = 'center';
      container.appendChild(errorDiv);
    };

    container.appendChild(imgElement);
  }

  renderFilesContent(container, item) {
    try {
      const filesJson = item.text.substring(6);
      const filesData = JSON.parse(filesJson);

      const filesContainer = document.createElement('div');
      filesContainer.className = 'files-container';

      const fileCount = document.createElement('div');
      fileCount.className = 'file-count';
      fileCount.textContent = `${filesData.files.length} 个文件`;
      filesContainer.appendChild(fileCount);

      filesData.files.forEach((file) => {
        const fileItem = document.createElement('div');
        fileItem.className = 'file-item';

        // 文件图标
        const iconElement = document.createElement('div');
        iconElement.className = 'file-icon';
        if (window.createFileIconElement) {
          const icon = window.createFileIconElement(file, 'small');
          iconElement.appendChild(icon);
        }

        // 文件信息
        const infoElement = document.createElement('div');
        infoElement.className = 'file-info';

        const nameElement = document.createElement('div');
        nameElement.className = 'file-name';
        nameElement.textContent = file.name;
        nameElement.title = file.path;

        const detailsElement = document.createElement('div');
        detailsElement.className = 'file-details';
        detailsElement.textContent = `${file.file_type} • ${this.formatFileSize(file.size)}`;

        infoElement.appendChild(nameElement);
        infoElement.appendChild(detailsElement);

        fileItem.appendChild(iconElement);
        fileItem.appendChild(infoElement);
        filesContainer.appendChild(fileItem);
      });

      container.appendChild(filesContainer);
    } catch (error) {
      console.error('解析文件数据失败:', error);
      container.textContent = '文件数据解析失败';
    }
  }

  renderTextContent(container, item) {
    const textElement = document.createElement('div');
    textElement.className = 'clipboard-text';
    textElement.textContent = item.text;
    container.appendChild(textElement);
  }

  formatFileSize(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
}

export class QuickTextsVirtualScroll extends VirtualScroll {
  constructor(container, options = {}) {
    const defaultOptions = {
      estimatedItemHeight: 90,
      overscan: 3,
      ...options
    };

    super(container, defaultOptions);

    this.searchTerm = '';
    this.filterType = 'all';
    this.onItemContextMenu = options.onItemContextMenu || (() => { });
    this.onItemEdit = options.onItemEdit || (() => { });
    this.isDragging = false;

    this.renderItem = this.renderQuickTextItem.bind(this);
  }

  setFilter(searchTerm, filterType) {
    this.searchTerm = searchTerm;
    this.filterType = filterType;
    this.applyFilter();
  }

  applyFilter() {
    if (!this.originalData) {
      this.originalData = [...this.data];
    }

    let filteredData = this.originalData;

    // 应用类型筛选
    if (this.filterType !== 'all') {
      filteredData = filteredData.filter(text => {
        const contentType = this.getContentType(text.content);
        return contentType === this.filterType;
      });
    }

    // 应用搜索筛选
    if (this.searchTerm) {
      const searchLower = this.searchTerm.toLowerCase();
      filteredData = filteredData.filter(text => {
        const contentType = this.getContentType(text.content);

        if (contentType === 'files') {
          try {
            const filesJson = text.content.substring(6);
            const filesData = JSON.parse(filesJson);
            const searchableText = filesData.files.map(file =>
              `${file.name} ${file.path} ${file.file_type}`
            ).join(' ').toLowerCase();
            return text.title.toLowerCase().includes(searchLower) ||
              searchableText.includes(searchLower);
          } catch (error) {
            return text.title.toLowerCase().includes(searchLower);
          }
        } else if (contentType === 'image') {
          return text.title.toLowerCase().includes(searchLower);
        } else {
          return text.title.toLowerCase().includes(searchLower) ||
            text.content.toLowerCase().includes(searchLower);
        }
      });
    }

    this.setData(filteredData);
  }

  getContentType(content) {
    if (content.startsWith('data:image/') || content.startsWith('image:')) {
      return 'image';
    }
    if (content.startsWith('files:')) {
      return 'files';
    }
    if (content.startsWith('http://') || content.startsWith('https://')) {
      return 'link';
    }
    return 'text';
  }

  setOriginalData(data) {
    this.originalData = [...data];
    this.applyFilter();
  }

  renderQuickTextItem(text, index) {
    const quickTextItem = document.createElement('div');
    quickTextItem.className = 'quick-text-item';
    quickTextItem.dataset.index = index;

    // 创建标题
    const titleElement = document.createElement('div');
    titleElement.className = 'quick-text-title';
    titleElement.textContent = text.title;

    // 创建内容
    const contentElement = document.createElement('div');
    contentElement.className = 'quick-text-content';

    // 根据内容类型显示不同内容
    const contentType = this.getContentType(text.content);
    if (contentType === 'image') {
      this.renderImageContent(contentElement, text);
    } else if (contentType === 'files') {
      this.renderFilesContent(contentElement, text);
    } else {
      contentElement.textContent = text.content;
    }

    quickTextItem.appendChild(titleElement);
    quickTextItem.appendChild(contentElement);

    // 添加右键菜单事件
    quickTextItem.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      this.onItemContextMenu(e, text);
    });

    // 设置拖拽属性
    this.setupDragAndDrop(quickTextItem, text);

    return quickTextItem;
  }

  renderImageContent(container, text) {
    const imgElement = document.createElement('img');
    imgElement.className = 'quick-text-image';
    imgElement.draggable = false;

    if (text.content.startsWith('image:')) {
      const imageId = text.content.substring(6);
      loadImageById(imgElement, imageId, true);
    } else if (text.content.startsWith('data:image/')) {
      imgElement.src = text.content;
    } else {
      imgElement.alt = '图片加载失败';
      imgElement.style.backgroundColor = '#e0e0e0';
    }

    container.appendChild(imgElement);
  }

  renderFilesContent(container, text) {
    try {
      const filesJson = text.content.substring(6);
      const filesData = JSON.parse(filesJson);

      const filesContainer = document.createElement('div');
      filesContainer.className = 'files-container';

      const fileCount = document.createElement('div');
      fileCount.className = 'file-count';
      fileCount.textContent = `${filesData.files.length} 个文件`;
      filesContainer.appendChild(fileCount);

      filesData.files.slice(0, 3).forEach((file) => {
        const fileItem = document.createElement('div');
        fileItem.className = 'file-item';

        const iconElement = document.createElement('div');
        iconElement.className = 'file-icon';
        if (window.createFileIconElement) {
          const icon = window.createFileIconElement(file, 'small');
          iconElement.appendChild(icon);
        }

        const infoElement = document.createElement('div');
        infoElement.className = 'file-info';

        const nameElement = document.createElement('div');
        nameElement.className = 'file-name';
        nameElement.textContent = file.name;

        const detailsElement = document.createElement('div');
        detailsElement.className = 'file-details';
        detailsElement.textContent = `${file.file_type}`;

        infoElement.appendChild(nameElement);
        infoElement.appendChild(detailsElement);

        fileItem.appendChild(iconElement);
        fileItem.appendChild(infoElement);
        filesContainer.appendChild(fileItem);
      });

      if (filesData.files.length > 3) {
        const moreElement = document.createElement('div');
        moreElement.className = 'file-more';
        moreElement.textContent = `还有 ${filesData.files.length - 3} 个文件...`;
        filesContainer.appendChild(moreElement);
      }

      container.appendChild(filesContainer);
    } catch (error) {
      console.error('解析文件数据失败:', error);
      container.textContent = '文件数据解析失败';
    }
  }

  setupDragAndDrop(element, text) {
    element.draggable = true;

    element.addEventListener('dragstart', (e) => {
      this.isDragging = true;
      const dragData = JSON.stringify({
        type: 'quicktext',
        id: text.id,
        title: text.title,
        content: text.content
      });

      e.dataTransfer.setData('application/x-quickclipboard', dragData);
      e.dataTransfer.setData('text/plain', dragData);
      e.dataTransfer.effectAllowed = 'move';

      document.querySelector('.tab-content.active').classList.add('dragging');
      const sidebar = document.getElementById('groups-sidebar');
      if (sidebar && !sidebar.classList.contains('pinned')) {
        sidebar.classList.add('show');
      }
    });

    element.addEventListener('dragend', () => {
      this.isDragging = false;
      document.querySelector('.tab-content.active').classList.remove('dragging');
      const sidebar = document.getElementById('groups-sidebar');
      if (sidebar && !sidebar.classList.contains('pinned')) {
        sidebar.classList.remove('show');
      }
    });
  }
}
