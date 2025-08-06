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
      rows_in_block: 20, // 每个块的行数 - 减少以提高平滑度
      blocks_in_cluster: 6, // 每个集群的块数 - 增加缓冲区
      show_no_data_row: true,
      no_data_text: '暂无数据',
      no_data_class: 'clusterize-no-data'
    });

    // 初始化 Sortable
    this.initSortable();

    // 绑定事件
    this.bindEvents();
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
          this.onSort(evt.oldIndex, evt.newIndex);
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
  }

  generateRows() {
    if (!this.data || this.data.length === 0) {
      return [];
    }

    return this.data.map((item, index) => {
      const html = this.renderItem(item, index);
      // 确保每个项目都有 data-index 属性用于事件委托
      return html.replace(/^<([^>]+)/, `<$1 data-index="${index}"`);
    });
  }

  updateData(newData) {
    this.data = newData;
    const rows = this.generateRows();
    this.clusterize.update(rows);
  }

  appendData(newItems) {
    this.data = [...this.data, ...newItems];
    const newRows = newItems.map((item, index) => {
      const actualIndex = this.data.length - newItems.length + index;
      const html = this.renderItem(item, actualIndex);
      return html.replace(/^<([^>]+)/, `<$1 data-index="${actualIndex}"`);
    });
    this.clusterize.append(newRows);
  }

  prependData(newItems) {
    this.data = [...newItems, ...this.data];
    const newRows = newItems.map((item, index) => {
      const html = this.renderItem(item, index);
      return html.replace(/^<([^>]+)/, `<$1 data-index="${index}"`);
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
  }

  getRowsAmount() {
    return this.clusterize ? this.clusterize.getRowsAmount() : 0;
  }

  getScrollProgress() {
    return this.clusterize ? this.clusterize.getScrollProgress() : 0;
  }
}
