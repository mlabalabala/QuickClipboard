// 列表管理器基类 - 提供列表的通用管理逻辑

import { VirtualList } from '../virtualList.js';
import { matchesFilter, matchesSearch } from '../utils/typeFilter.js';
import { checkFilesExistence } from './actions/fileActions.js';

export class BaseListManager {
  constructor(config) {
    this.scrollId = config.scrollId;
    this.contentId = config.contentId;
    this.data = [];
    this.virtualList = null;
    this.filterType = 'all';
    this.searchTerm = '';
    
    // 需要由子类实现的方法
    this.renderItemHTML = config.renderItemHTML;
    this.onItemClick = config.onItemClick;
    this.onItemContextMenu = config.onItemContextMenu;
    this.onSort = config.onSort;
    this.sortableOptions = config.sortableOptions || {};
  }

  /**
   * 初始化虚拟列表
   */
  initVirtualList() {
    if (this.virtualList) {
      this.virtualList.destroy();
    }

    this.virtualList = new VirtualList({
      scrollId: this.scrollId,
      contentId: this.contentId,
      data: this.getFilteredData(),
      renderItem: this.renderItemHTML,
      onSort: this.onSort,
      onItemClick: this.onItemClick,
      onItemContextMenu: this.onItemContextMenu,
      sortableOptions: this.sortableOptions
    });

    return this.virtualList;
  }

  /**
   * 设置数据
   */
  setData(data) {
    this.data = data;
  }

  /**
   * 获取数据
   */
  getData() {
    return this.data;
  }

  /**
   * 设置过滤类型
   */
  setFilter(filterType) {
    this.filterType = filterType;
  }

  /**
   * 设置搜索词
   */
  setSearch(searchTerm) {
    this.searchTerm = searchTerm.toLowerCase();
  }

  /**
   * 获取过滤后的数据
   */
  getFilteredData() {
    return this.data.filter(item => {
      const contentType = item.content_type || 'text';

      // 类型筛选
      if (!matchesFilter(contentType, this.filterType, item)) {
        return false;
      }

      // 搜索筛选
      return matchesSearch(item, this.searchTerm, contentType);
    });
  }

  /**
   * 渲染列表
   */
  render() {
    if (!this.virtualList) {
      this.initVirtualList();
    } else {
      const filteredData = this.getFilteredData();
      this.virtualList.updateData(filteredData);
    }

    // 异步检查文件是否存在
    setTimeout(() => {
      this.checkFiles();
    }, 0);

    // 通知导航模块列表已更新
    this.notifyNavigationUpdate();
  }

  /**
   * 检查文件是否存在
   */
  async checkFiles() {
    await checkFilesExistence(`#${this.scrollId}`);
  }

  /**
   * 通知导航模块列表已更新
   */
  notifyNavigationUpdate() {
    import('../navigation.js').then(module => {
      module.onListUpdate();
    }).catch(() => {});
  }

  /**
   * 更新数据并重新渲染
   */
  updateData(newData) {
    this.setData(newData);
    this.render();
  }

  /**
   * 应用过滤并重新渲染
   */
  applyFilter() {
    this.render();
    
    // 设置搜索结果滚动
    import('../utils/highlight.js').then(module => {
      module.setupSearchResultScrolling();
    }).catch(() => {});
  }

  /**
   * 获取虚拟列表实例
   */
  getVirtualList() {
    return this.virtualList;
  }

  /**
   * 销毁列表
   */
  destroy() {
    if (this.virtualList) {
      this.virtualList.destroy();
      this.virtualList = null;
    }
  }
}

