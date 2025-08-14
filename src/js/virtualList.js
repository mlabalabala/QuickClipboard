import Clusterize from 'clusterize.js';
import Sortable from 'sortablejs';

/**
 * 虚拟滚动列表类
 * 结合 Clusterize.js 和 SortableJS 实现高性能的可拖拽虚拟滚动列表
 */
export class VirtualList {
  constructor(options) {
    this.scrollId = options.scrollId;
    this.contentId = options.contentId;
    this.data = options.data || [];
    this.renderItem = options.renderItem;
    this.onSort = options.onSort;
    this.onItemClick = options.onItemClick;
    this.onItemContextMenu = options.onItemContextMenu;
    this.sortableOptions = options.sortableOptions || {};

    this.clusterize = null;
    this.sortable = null;
    this.isDragging = false;

    this.init();
  }

  init() {
    // 初始化 Clusterize
    this.clusterize = new Clusterize({
      rows: this.generateRows(),
      scrollId: this.scrollId,
      contentId: this.contentId,
      rows_in_block: 15, //块大小
      blocks_in_cluster: 4, // 缓冲区
      show_no_data_row: true,
      no_data_text: '暂无数据',
      no_data_class: 'clusterize-no-data'
    });

    // 初始化 Sortable
    this.initSortable();

    // 绑定事件
    this.bindEvents();

    // 监听行高变化
    this.bindRowHeightListener();

    // 初始化时触发图片加载
    this.triggerImageLoad();
  }

  initSortable() {
    const contentElement = document.getElementById(this.contentId);
    if (!contentElement) {
      return;
    }

    const defaultOptions = {
      animation: 150,
      ghostClass: 'sortable-ghost',
      chosenClass: 'sortable-chosen',
      dragClass: 'sortable-drag',
      onStart: (evt) => {
        this.isDragging = true;
        this.setDragData(evt);

        if (this.sortableOptions.onStart) {
          this.sortableOptions.onStart(evt);
        }
      },
      onEnd: (evt) => {
        setTimeout(() => {
          this.isDragging = false;
        }, 100);

        if (this.sortableOptions.onEnd) {
          this.sortableOptions.onEnd(evt);
        }

        if (evt.oldIndex !== evt.newIndex && this.onSort) {
          const realOldIndex = parseInt(evt.item.getAttribute('data-index'));
          let realNewIndex = realOldIndex;

          // 找到新位置的非拖拽元素来确定真实索引
          const elements = Array.from(evt.to.children);
          const isForward = evt.newIndex < evt.oldIndex;

          // 根据拖拽方向查找参考元素
          const searchStart = isForward ? evt.newIndex : evt.newIndex + 1;
          const searchEnd = isForward ? elements.length : 0;
          const searchStep = isForward ? 1 : -1;

          for (let i = searchStart; isForward ? i < searchEnd : i >= searchEnd; i += searchStep) {
            if (elements[i] && elements[i] !== evt.item) {
              const refIndex = parseInt(elements[i].getAttribute('data-index'));
              realNewIndex = isForward ? refIndex : refIndex + 1;
              break;
            }
          }

          if (realOldIndex !== realNewIndex) {
            console.log('oldIndex:', realOldIndex, 'newIndex:', realNewIndex);
            this.onSort(realOldIndex, realNewIndex);

            // 拖拽完成后设置该项为激活状态
            setTimeout(() => {
              this.setDraggedItemActive(evt.item, realNewIndex);
            }, 50); // 延迟一点时间确保DOM更新完成
          }
        }
      }
    };

    const userOnEnd = this.sortableOptions.onEnd;
    const userOnStart = this.sortableOptions.onStart;

    const finalOptions = {
      ...this.sortableOptions,
      ...defaultOptions,
      onStart: defaultOptions.onStart,
      onEnd: defaultOptions.onEnd
    };

    this.sortableOptions.onEnd = userOnEnd;
    this.sortableOptions.onStart = userOnStart;

    this.sortable = Sortable.create(contentElement, finalOptions);
  }

  // 设置拖拽数据
  setDragData(evt) {
    const draggedElement = evt.item;
    const index = parseInt(draggedElement.getAttribute('data-index'));
    this.setDragDataForElement(evt.originalEvent, index);
  }

  // 为指定元素设置拖拽数据
  setDragDataForElement(event, index) {
    if (index >= 0 && index < this.data.length) {
      const item = this.data[index];
      let dragData = {};

      // 根据列表类型设置不同的拖拽数据
      if (this.scrollId === 'clipboard-list') {
        // 剪贴板列表 - 需要找到在原始数组中的索引
        const originalIndex = this.findOriginalIndex(item);
        dragData = {
          type: 'clipboard',
          index: originalIndex,
          content: item.text
        };
      } else if (this.scrollId === 'quick-texts-list') {
        // 常用文本列表
        dragData = {
          type: 'quicktext',
          id: item.id,
          content: item.content
        };
      }

      // 设置拖拽数据
      const dragDataString = JSON.stringify(dragData);

      // 使用自定义MIME类型和text/plain作为备用
      if (event && event.dataTransfer) {
        event.dataTransfer.setData('application/x-quickclipboard', dragDataString);
        event.dataTransfer.setData('text/plain', dragDataString);
      }
    }
  }

  // 查找项目在原始数组中的索引
  findOriginalIndex(item) {
    if (this.scrollId === 'clipboard-list') {
      // 通过时间戳和内容匹配找到原始索引
      try {
        // 尝试从全局作用域获取clipboardHistory
        const clipboardHistory = window.clipboardHistory || [];
        return clipboardHistory.findIndex(originalItem =>
          originalItem.text === item.text &&
          originalItem.timestamp === item.timestamp
        );
      } catch (error) {
        console.warn('无法获取原始剪贴板历史数组:', error);
        return -1;
      }
    }
    return -1;
  }

  // 设置拖拽项为激活状态
  setDraggedItemActive(draggedElement, newIndex) {
    try {
      // 动态导入navigation模块以避免循环依赖
      import('./navigation.js').then(navigationModule => {
        // 更新拖拽元素的data-index属性为新索引
        draggedElement.setAttribute('data-index', newIndex.toString());

        // 调用navigation模块的syncClickedItem函数设置激活状态
        navigationModule.syncClickedItem(draggedElement);
      }).catch(error => {
        console.warn('设置拖拽项激活状态失败:', error);
      });
    } catch (error) {
      console.warn('设置拖拽项激活状态失败:', error);
    }
  }



  bindEvents() {
    const contentElement = document.getElementById(this.contentId);
    if (!contentElement) return;

    // 使用事件委托处理点击事件
    contentElement.addEventListener('click', (e) => {
      if (this.isDragging) return;

      const item = e.target.closest('[data-index]');
      if (item && this.onItemClick) {
        const index = parseInt(item.getAttribute('data-index'));
        this.onItemClick(index, e);
      }
    });

    // 使用事件委托处理右键菜单
    contentElement.addEventListener('contextmenu', (e) => {
      if (this.isDragging) return;

      const item = e.target.closest('[data-index]');
      if (item && this.onItemContextMenu) {
        const index = parseInt(item.getAttribute('data-index'));
        this.onItemContextMenu(index, e);
      }
    });

    // 使用事件委托处理拖拽开始事件
    contentElement.addEventListener('dragstart', (e) => {
      const item = e.target.closest('[data-index]');
      if (item) {
        const index = parseInt(item.getAttribute('data-index'));
        this.setDragDataForElement(e, index);
      }
    });

    // 监听滚动事件，触发图片加载
    const scrollElement = document.getElementById(this.scrollId);
    if (scrollElement) {
      let scrollTimeout;
      scrollElement.addEventListener('scroll', () => {
        // 防抖处理，避免频繁触发
        clearTimeout(scrollTimeout);
        scrollTimeout = setTimeout(() => {
          this.triggerImageLoad();
        }, 100);
      });
    }
  }

  generateRows() {
    if (!this.data || this.data.length === 0) {
      return [];
    }

    return this.data.map((item, index) => {
      const html = this.renderItem(item, index);
      // renderItem应该已经包含了正确的data-index属性，不需要重复添加
      return html;
    });
  }

  updateData(newData) {
    this.data = newData;
    const rows = this.generateRows();
    this.clusterize.update(rows);

    // 更新数据后触发图片加载
    this.triggerImageLoad();
  }

  appendData(newItems) {
    this.data = [...this.data, ...newItems];
    const newRows = newItems.map((item, index) => {
      const actualIndex = this.data.length - newItems.length + index;
      const html = this.renderItem(item, actualIndex);
      return html;
    });
    this.clusterize.append(newRows);
  }

  prependData(newItems) {
    this.data = [...newItems, ...this.data];
    const newRows = newItems.map((item, index) => {
      const html = this.renderItem(item, index);
      return html;
    });
    this.clusterize.prepend(newRows);

    // 更新所有现有项目的索引
    this.updateData(this.data);
  }

  clear() {
    this.data = [];
    this.clusterize.clear();
  }

  refresh() {
    this.clusterize.refresh();
  }

  destroy() {
    if (this.sortable) {
      this.sortable.destroy();
    }
    if (this.clusterize) {
      this.clusterize.destroy();
    }
    
    // 清理行高变化监听器
    if (this.rowHeightChangeHandler) {
      window.removeEventListener('row-height-changed', this.rowHeightChangeHandler);
    }
    
    // 清理标签页切换监听器
    if (this.tabSwitchHandler) {
      window.removeEventListener('tab-switched', this.tabSwitchHandler);
    }
  }

  getRowsAmount() {
    return this.clusterize ? this.clusterize.getRowsAmount() : 0;
  }

  getScrollProgress() {
    return this.clusterize ? this.clusterize.getScrollProgress() : 0;
  }

  // 滚动到指定索引
  scrollToIndex(index) {
    if (!this.clusterize || index < 0 || index >= this.data.length) {
      return false; // 返回是否成功滚动
    }

    const scrollElement = document.getElementById(this.scrollId);
    if (!scrollElement) return false;

    // 估算每个项目的高度
    const contentElement = document.getElementById(this.contentId);
    let itemHeight = this.getCurrentRowHeight(); // 根据当前行高设置获取高度
    // 尝试从已渲染的项目中获取实际高度
    const renderedItems = contentElement ? contentElement.querySelectorAll('[data-index]') : [];
    if (renderedItems.length > 0) {
      // 计算平均高度，以处理高度不一致的情况
      let totalHeight = 0;
      let count = 0;
      for (let item of renderedItems) {
        const height = item.offsetHeight;
        if (height > 0) { // 只计算有效高度
          totalHeight += height;
          count++;
        }
      }
      if (count > 0) {
        itemHeight = totalHeight / count;
      }
    }

    // 计算目标滚动位置
    const targetScrollTop = index * itemHeight;

    // 获取容器高度
    const containerHeight = scrollElement.clientHeight;
    const currentScrollTop = scrollElement.scrollTop;

    // 计算可视区域
    const viewTop = currentScrollTop;
    const viewBottom = currentScrollTop + containerHeight;
    const itemTop = targetScrollTop;
    const itemBottom = targetScrollTop + itemHeight;

    // 减少缓冲区，让滚动更精确
    const buffer = itemHeight * 0.2;

    // 检查是否需要滚动
    let needScroll = false;
    let newScrollTop = currentScrollTop;

    if (itemTop < viewTop + buffer) {
      // 项目在视口上方，向上滚动
      newScrollTop = Math.max(0, itemTop - buffer);
      needScroll = true;
    } else if (itemBottom > viewBottom - buffer) {
      // 项目在视口下方，向下滚动
      newScrollTop = Math.min(
        scrollElement.scrollHeight - containerHeight,
        itemBottom - containerHeight + buffer
      );
      needScroll = true;
    }

    if (needScroll) {
      // 使用instant行为，确保立即滚动
      scrollElement.scrollTo({
        top: newScrollTop,
        behavior: 'instant'
      });
      return true;
    }

    return false; // 不需要滚动
  }

  // 获取当前数据长度
  getDataLength() {
    return this.data ? this.data.length : 0;
  }

  // 根据当前行高设置获取项目高度
  getCurrentRowHeight() {
    const currentRowHeight = localStorage.getItem('app-row-height') || 'medium';
    
    switch (currentRowHeight) {
      case 'large':
        return 120; // 大
      case 'medium':
        return 90;  // 中
      case 'small':
        return 50;  // 小
      default:
        return 90;  // 默认中等
    }
  }

  // 绑定行高变化监听器
  bindRowHeightListener() {
    this.rowHeightChangeHandler = (event) => {
      // 行高改变时刷新虚拟列表，确保滚动计算正确
      setTimeout(() => {
        // 先强制重新计算布局
        const scrollElement = document.getElementById(this.scrollId);
        const contentElement = document.getElementById(this.contentId);
        
        if (scrollElement && contentElement) {
          // 保存当前滚动位置
          const currentScrollTop = scrollElement.scrollTop;
          
          // 刷新虚拟列表
          this.refresh();
          
          // 重新设置滚动位置，避免跳动
          setTimeout(() => {
            scrollElement.scrollTop = currentScrollTop;
          }, 10);
        }
      }, 150);
    };
    
    // 监听标签页切换事件，在切换时刷新虚拟列表并回到顶部
    this.tabSwitchHandler = () => {
      setTimeout(() => {
        // 刷新虚拟列表
        this.refresh();
        
        // 滚动到顶部
        const scrollElement = document.getElementById(this.scrollId);
        if (scrollElement) {
          scrollElement.scrollTo({
            top: 0,
            behavior: 'instant'
          });
        }
      }, 50);
    };
    
    window.addEventListener('row-height-changed', this.rowHeightChangeHandler);
    // 监听标签页切换
    window.addEventListener('tab-switched', this.tabSwitchHandler);
  }

  // 触发图片加载
  triggerImageLoad() {
    // 延迟执行，确保DOM已更新
    setTimeout(() => {
      // 根据列表类型触发相应的图片加载
      if (this.scrollId === 'clipboard-list') {
        // 触发剪贴板图片加载
        this.loadClipboardImages();
      } else if (this.scrollId === 'quick-texts-list') {
        // 触发常用文本图片加载
        this.loadQuickTextImages();
      }
    }, 50);
  }

  // 加载剪贴板图片
  async loadClipboardImages() {
    try {
      // 动态导入clipboard模块以避免循环依赖
      const clipboardModule = await import('./clipboard.js');

      // 加载文件图标
      const fileIcons = document.querySelectorAll('.file-icon[data-needs-load="true"]');
      for (const icon of fileIcons) {
        const filePath = icon.getAttribute('data-file-path');
        if (filePath) {
          try {
            const { invoke } = await import('@tauri-apps/api/core');
            const dataUrl = await invoke('read_image_file', { filePath });
            icon.src = dataUrl;
            icon.style.objectFit = 'cover';
            icon.style.borderRadius = '2px';
            icon.removeAttribute('data-needs-load');
            icon.removeAttribute('data-file-path');
          } catch (error) {
            console.warn('加载文件图标失败:', error);
          }
        }
      }

      // 加载剪贴板图片
      const clipboardImages = document.querySelectorAll('.clipboard-image[data-needs-load="true"]');
      for (const img of clipboardImages) {
        const imageId = img.getAttribute('data-image-id');
        if (imageId) {
          try {
            await clipboardModule.loadImageById(img, imageId, true);
            img.removeAttribute('data-needs-load');
            img.removeAttribute('data-image-id');
          } catch (error) {
            console.warn('加载剪贴板图片失败:', error);
            img.alt = '图片加载失败';
            img.style.backgroundColor = '#e0e0e0';
          }
        }
      }
    } catch (error) {
      console.warn('加载剪贴板图片模块失败:', error);
    }
  }

  // 加载常用文本图片
  async loadQuickTextImages() {
    try {
      // 动态导入quickTexts模块以避免循环依赖
      const quickTextsModule = await import('./quickTexts.js');

      // 加载常用文本中的图片
      const quickTextImages = document.querySelectorAll('.quick-text-image[data-needs-load="true"]');
      for (const img of quickTextImages) {
        const imageId = img.getAttribute('data-image-id');
        if (imageId) {
          try {
            await quickTextsModule.loadImageById(img, imageId, true);
            img.removeAttribute('data-needs-load');
            img.removeAttribute('data-image-id');
          } catch (error) {
            console.warn('加载常用文本图片失败:', error);
            img.alt = '图片加载失败';
            img.style.backgroundColor = '#e0e0e0';
          }
        }
      }

      // 加载常用文本中的文件图标
      const fileIcons = document.querySelectorAll('.file-icon[data-needs-load="true"]');
      for (const icon of fileIcons) {
        const filePath = icon.getAttribute('data-file-path');
        if (filePath) {
          try {
            const { invoke } = await import('@tauri-apps/api/core');
            const dataUrl = await invoke('read_image_file', { filePath });
            icon.src = dataUrl;
            icon.style.objectFit = 'cover';
            icon.style.borderRadius = '2px';
            icon.removeAttribute('data-needs-load');
            icon.removeAttribute('data-file-path');
          } catch (error) {
            console.warn('加载文件图标失败:', error);
          }
        }
      }
    } catch (error) {
      console.warn('加载常用文本图片模块失败:', error);
    }
  }
}
