8/**
 * 虚拟滚动组件
 * 支持动态高度、缓冲区、平滑滚动等功能
 */
export class VirtualScroll {
  constructor(container, options = {}) {
    this.container = container;
    this.options = {
      itemHeight: 60, // 默认项目高度
      bufferSize: 5, // 缓冲区大小
      overscan: 3, // 额外渲染的项目数量
      estimatedItemHeight: 60, // 估算项目高度
      ...options
    };

    // 数据和状态
    this.data = [];
    this.itemHeights = new Map(); // 存储每个项目的实际高度
    this.scrollTop = 0;
    this.containerHeight = 0;
    this.totalHeight = 0;
    this.startIndex = 0;
    this.endIndex = 0;
    this.visibleItems = [];

    // DOM 元素
    this.viewport = null;
    this.scrollContainer = null;
    this.spacerBefore = null;
    this.spacerAfter = null;

    // 回调函数
    this.renderItem = options.renderItem || (() => document.createElement('div'));
    this.onScroll = options.onScroll || (() => { });
    this.onItemClick = options.onItemClick || (() => { });

    // 性能优化
    this.isScrolling = false;
    this.scrollTimer = null;
    this.resizeTimer = null;
    this.resizeObserver = null;

    this.init();
  }

  init() {
    this.createDOM();
    this.bindEvents();
    this.updateContainerHeight();
  }

  createDOM() {
    // 保存原有的样式和属性
    const originalStyles = window.getComputedStyle(this.container);
    const originalDirection = originalStyles.direction;
    const originalOverflow = originalStyles.overflow;

    // 清空容器但保持原有样式
    this.container.innerHTML = '';

    // 如果原容器没有设置position，则设置为relative
    if (originalStyles.position === 'static') {
      this.container.style.position = 'relative';
    }

    // 创建滚动容器，继承原有的overflow和direction样式
    this.scrollContainer = document.createElement('div');
    this.scrollContainer.style.cssText = `
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      overflow-y: auto;
      overflow-x: hidden;
      direction: ${originalDirection};
    `;

    // 创建视口
    this.viewport = document.createElement('div');
    this.viewport.style.cssText = `
      position: relative;
      width: 100%;
      direction: ltr;
    `;

    // 创建前置占位符
    this.spacerBefore = document.createElement('div');
    this.spacerBefore.style.cssText = `
      height: 0px;
      width: 100%;
      direction: ltr;
    `;

    // 创建后置占位符
    this.spacerAfter = document.createElement('div');
    this.spacerAfter.style.cssText = `
      height: 0px;
      width: 100%;
      direction: ltr;
    `;

    this.viewport.appendChild(this.spacerBefore);
    this.viewport.appendChild(this.spacerAfter);
    this.scrollContainer.appendChild(this.viewport);
    this.container.appendChild(this.scrollContainer);
  }

  bindEvents() {
    // 滚动事件
    this.scrollContainer.addEventListener('scroll', this.handleScroll.bind(this), { passive: true });

    // 监听容器大小变化
    if (window.ResizeObserver) {
      this.resizeObserver = new ResizeObserver(() => {
        // 使用防抖机制，避免频繁触发
        if (this.resizeTimer) {
          clearTimeout(this.resizeTimer);
        }
        this.resizeTimer = setTimeout(() => {
          this.updateContainerHeight();
          if (this.data.length > 0) {
            this.updateVisibleRange();
          }
        }, 100);
      });
      this.resizeObserver.observe(this.container);
    }

    // 监听窗口大小变化
    window.addEventListener('resize', () => {
      this.updateContainerHeight();
      this.updateVisibleRange();
    });
  }

  handleScroll(event) {
    this.scrollTop = this.scrollContainer.scrollTop;
    this.isScrolling = true;

    // 清除之前的定时器
    if (this.scrollTimer) {
      clearTimeout(this.scrollTimer);
    }

    // 立即更新可见范围
    this.updateVisibleRange();

    // 设置滚动结束定时器
    this.scrollTimer = setTimeout(() => {
      this.isScrolling = false;
      this.onScroll(this.scrollTop);
    }, 150);
  }

  updateContainerHeight() {
    const rect = this.container.getBoundingClientRect();
    const newHeight = rect.height;

    const wasZero = this.containerHeight === 0;
    const heightChanged = Math.abs(this.containerHeight - newHeight) > 10;

    // 防止容器高度被意外设置为0（除非是初始状态）
    if (newHeight === 0 && this.containerHeight > 0) {
      // 容器可能被隐藏了，不更新高度，保持之前的值
      return;
    }

    this.containerHeight = newHeight;

    if ((wasZero && newHeight > 0) || heightChanged) {
      if (this.data.length > 0) {
        this.updateVisibleRange();
      }
    }
  }

  setData(data) {
    this.data = data;
    this.itemHeights.clear();
    this.calculateTotalHeight();

    this.updateContainerHeight();

    if (!this.containerHeight) {
      requestAnimationFrame(() => {
        this.updateContainerHeight();
        this.updateVisibleRange();
      });

      setTimeout(() => {
        this.updateContainerHeight();
        this.updateVisibleRange();
      }, 50);

      setTimeout(() => {
        this.updateContainerHeight();
        this.updateVisibleRange();
      }, 200);
    } else {
      this.updateVisibleRange();
    }
  }

  calculateTotalHeight() {
    let height = 0;
    for (let i = 0; i < this.data.length; i++) {
      height += this.getItemHeight(i);
    }
    this.totalHeight = height;
    this.viewport.style.height = `${this.totalHeight}px`;
  }

  getItemHeight(index) {
    if (this.itemHeights.has(index)) {
      return this.itemHeights.get(index);
    }
    return this.options.estimatedItemHeight;
  }

  setItemHeight(index, height) {
    const oldHeight = this.getItemHeight(index);
    if (oldHeight !== height) {
      this.itemHeights.set(index, height);
      this.totalHeight += (height - oldHeight);
      this.viewport.style.height = `${this.totalHeight}px`;
    }
  }

  getItemOffset(index) {
    let offset = 0;
    for (let i = 0; i < index; i++) {
      offset += this.getItemHeight(i);
    }
    return offset;
  }

  updateVisibleRange() {
    if (!this.data.length) {
      this.renderItems();
      return;
    }

    // 检查容器是否可见
    const rect = this.container.getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0) {
      // 容器不可见，不进行渲染
      return;
    }

    if (!this.containerHeight) {
      const estimatedHeight = 400;
      const visibleCount = Math.ceil(estimatedHeight / this.estimatedItemHeight) + this.overscan;
      this.startIndex = 0;
      this.endIndex = Math.min(visibleCount - 1, this.data.length - 1);
      this.renderItems();
      return;
    }

    const scrollTop = this.scrollTop;
    const scrollBottom = scrollTop + this.containerHeight;

    // 计算可见范围
    let startIndex = 0;
    let endIndex = this.data.length - 1;

    // 二分查找起始索引
    let low = 0;
    let high = this.data.length - 1;
    while (low <= high) {
      const mid = Math.floor((low + high) / 2);
      const offset = this.getItemOffset(mid);
      if (offset < scrollTop) {
        low = mid + 1;
      } else {
        high = mid - 1;
      }
    }
    startIndex = Math.max(0, high);

    // 从起始索引开始查找结束索引
    let currentOffset = this.getItemOffset(startIndex);
    endIndex = startIndex;
    while (endIndex < this.data.length && currentOffset < scrollBottom) {
      currentOffset += this.getItemHeight(endIndex);
      endIndex++;
    }
    endIndex = Math.min(this.data.length - 1, endIndex);

    // 添加缓冲区
    this.startIndex = Math.max(0, startIndex - this.options.overscan);
    this.endIndex = Math.min(this.data.length - 1, endIndex + this.options.overscan);

    this.renderItems();
  }

  renderItems() {
    // 清空当前渲染的项目
    this.visibleItems.forEach(item => {
      if (item.element && item.element.parentNode) {
        item.element.parentNode.removeChild(item.element);
      }
    });
    this.visibleItems = [];

    if (!this.data.length) {
      this.spacerBefore.style.height = '0px';
      this.spacerAfter.style.height = '0px';
      return;
    }

    // 计算占位符高度
    const beforeHeight = this.getItemOffset(this.startIndex);
    const afterHeight = this.totalHeight - this.getItemOffset(this.endIndex + 1);

    this.spacerBefore.style.height = `${beforeHeight}px`;
    this.spacerAfter.style.height = `${afterHeight}px`;

    for (let i = this.startIndex; i <= this.endIndex; i++) {
      const item = this.data[i];
      const element = this.renderItem(item, i);

      if (element) {
        // 设置项目样式
        element.style.position = 'relative';
        element.style.width = '100%';
        element.style.direction = 'ltr'; // 确保内容方向正确

        // 添加点击事件
        element.addEventListener('click', (e) => {
          this.onItemClick(item, i, e);
        });

        // 插入到正确位置
        this.viewport.insertBefore(element, this.spacerAfter);

        this.visibleItems.push({
          index: i,
          element: element,
          data: item
        });

        // 测量实际高度
        requestAnimationFrame(() => {
          const rect = element.getBoundingClientRect();
          if (rect.height > 0) {
            this.setItemHeight(i, rect.height);
          }
        });
      }
    }
  }

  scrollToIndex(index, behavior = 'smooth') {
    if (index < 0 || index >= this.data.length) return;

    const offset = this.getItemOffset(index);
    this.scrollContainer.scrollTo({
      top: offset,
      behavior: behavior
    });
  }

  scrollToTop(behavior = 'smooth') {
    this.scrollContainer.scrollTo({
      top: 0,
      behavior: behavior
    });
  }

  refresh() {
    this.calculateTotalHeight();
    this.updateVisibleRange();
  }

  destroy() {
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
    }
    if (this.scrollTimer) {
      clearTimeout(this.scrollTimer);
    }
    if (this.resizeTimer) {
      clearTimeout(this.resizeTimer);
    }
    this.container.innerHTML = '';
  }

  // 获取当前滚动位置
  getScrollTop() {
    return this.scrollTop;
  }

  // 获取可见项目数量
  getVisibleItemCount() {
    return this.visibleItems.length;
  }

  // 获取总项目数量
  getTotalItemCount() {
    return this.data.length;
  }
}
