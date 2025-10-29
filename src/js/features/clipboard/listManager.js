/**
 * 剪贴板列表管理器
 * 管理剪贴板历史列表的渲染和交互
 */

import { invoke } from '@tauri-apps/api/core';
import { BaseListManager } from './baseListManager.js';
import { generateClipboardItemHTML } from './renderers/itemRenderer.js';
import { buildClipboardContextMenu } from './contextMenuBuilder.js';
import { pasteContent, deleteItem, openTextEditor, addToFavorites } from './common/listOperations.js';
import { pinImageToScreen, saveImageAs } from './actions/imageActions.js';
import { 
  openFileWithDefaultProgram, 
  openFileLocation, 
  copyFilePaths,
  pinImageFileFromList
} from './actions/fileActions.js';
import { showNotification } from '../notificationManager.js';
import { showConfirmModal } from '../ui.js';

export class ClipboardListManager extends BaseListManager {
  constructor(config) {
    super({
      scrollId: config.scrollId || 'clipboard-list',
      contentId: config.contentId || 'clipboard-content',
      renderItemHTML: (item, index) => this.renderItem(item, index),
      onItemClick: (index, event) => this.handleItemClick(index, event),
      onItemContextMenu: (index, event) => this.handleItemContextMenu(index, event),
      onSort: (oldIndex, newIndex) => this.handleSort(oldIndex, newIndex),
      sortableOptions: {
        onStart: () => this.handleDragStart(),
        onEnd: () => this.handleDragEnd()
      }
    });

    this.activeItemIndex = config.activeItemIndex || -1;
    this.isOneTimePaste = config.isOneTimePaste || false;
    this.pasteWithFormat = config.pasteWithFormat || false;
    this.currentFilter = config.currentFilter || 'all';
    this.isDragging = false;
    
    // 外部回调
    this.onActiveItemChange = config.onActiveItemChange;
    this.onDataChange = config.onDataChange;
  }

  /**
   * 渲染单个列表项
   */
  renderItem(item, index) {
    const isSearching = this.searchTerm.trim() !== '';
    const isFiltering = this.currentFilter !== 'all';
    
    return generateClipboardItemHTML(item, index, {
      activeItemIndex: this.activeItemIndex,
      isSearching,
      isFiltering,
      pasteWithFormat: this.pasteWithFormat
    });
  }

  /**
   * 处理列表项点击
   */
  async handleItemClick(index, event) {
    if (this.isDragging) return;

    const filteredData = this.getFilteredData();
    const item = filteredData[index];
    if (!item) return;

    // 找到在原始数组中的索引
    const originalIndex = this.data.findIndex(originalItem => originalItem === item);
    if (originalIndex === -1) return;

    // 执行粘贴
    await this.pasteItem(item, originalIndex, event.target.closest('.clipboard-item'));
  }

  /**
   * 处理列表项右键菜单
   */
  handleItemContextMenu(index, event) {
    event.preventDefault();

    const filteredData = this.getFilteredData();
    const item = filteredData[index];
    if (!item) return;

    const originalIndex = this.data.findIndex(originalItem => originalItem === item);
    if (originalIndex === -1) return;

    this.showContextMenu(event, item, originalIndex);
  }

  /**
   * 处理排序
   */
  async handleSort(oldIndex, newIndex) {
    try {
      const filteredData = this.getFilteredData();

      if (oldIndex >= filteredData.length || newIndex >= filteredData.length) {
        return;
      }

      const movedItem = filteredData[oldIndex];
      const targetItem = filteredData[newIndex];

      const originalOldIndex = this.data.findIndex(item =>
        item.content === movedItem.content && item.created_at === movedItem.created_at
      );
      const originalNewIndex = this.data.findIndex(item =>
        item.content === targetItem.content && item.created_at === targetItem.created_at
      );

      if (originalOldIndex === -1 || originalNewIndex === -1) {
        return;
      }

      await invoke('move_clipboard_item', {
        fromIndex: originalOldIndex,
        toIndex: originalNewIndex
      });

      const newData = [...this.data];
      const [removed] = newData.splice(originalOldIndex, 1);
      newData.splice(originalNewIndex, 0, removed);
      
      this.setData(newData);
      if (this.onDataChange) {
        this.onDataChange(newData);
      }
      
      this.render();

    } catch (error) {
      console.error('更新剪贴板顺序失败:', error);
      await this.refreshFromBackend();
    }
  }

  /**
   * 处理拖拽开始
   */
  handleDragStart() {
    this.isDragging = true;
    document.querySelector('.tab-content.active')?.classList.add('dragging');
    const sidebar = document.getElementById('groups-sidebar');
    if (sidebar && !sidebar.classList.contains('pinned')) {
      sidebar.classList.add('show');
    }
  }

  /**
   * 处理拖拽结束
   */
  handleDragEnd() {
    this.isDragging = false;
    document.querySelector('.tab-content.active')?.classList.remove('dragging');
    const sidebar = document.getElementById('groups-sidebar');
    if (sidebar && !sidebar.classList.contains('pinned')) {
      sidebar.classList.remove('show');
    }
  }

  /**
   * 粘贴项目
   */
  async pasteItem(item, index, element = null) {
    await pasteContent(
      { clipboard_id: item.id },
      {
        element,
        onSuccess: async () => {
          this.setActiveItem(index);
          
          // 一次性粘贴：删除该项
          if (this.isOneTimePaste) {
            setTimeout(() => this.deleteItem(item.id), 100);
          }
        }
      }
    );
  }

  /**
   * 删除项目
   */
  async deleteItem(id) {
    await deleteItem(id, 'delete_clipboard_item', async () => {
      const newData = this.data.filter(item => item.id !== id);
      this.setData(newData);
      if (this.onDataChange) {
        this.onDataChange(newData);
      }
      this.render();
    });
  }

  /**
   * 清空剪贴板
   */
  async clearAll() {
    showConfirmModal(
      '确认清空',
      '确定要清空所有剪贴板历史记录吗？此操作不可撤销。',
      async () => {
        try {
          await invoke('clear_clipboard_history');
          showNotification('已清空剪贴板历史', 'success');
          await this.refreshFromBackend();
        } catch (error) {
          console.error('清空剪贴板历史失败:', error);
          showNotification('清空失败', 'error');
        }
      }
    );
  }

  /**
   * 显示上下文菜单
   */
  showContextMenu(event, item, index) {
    buildClipboardContextMenu(event, item, {
      pinImage: () => pinImageToScreen(item),
      saveImage: () => saveImageAs(item),
      pinImageFile: () => pinImageFileFromList(item),
      openFile: () => openFileWithDefaultProgram(item),
      openFileLocation: () => openFileLocation(item),
      copyFilePath: () => copyFilePaths(item),
      edit: async () => {
        await openTextEditor({
          id: item.id,
          content: item.content,
          title: `剪贴板项目 #${index + 1}`,
          timestamp: item.created_at
        });
      },
      addToFavorites: async () => {
        await addToFavorites(item.id);
      },
      deleteItem: () => this.deleteItem(item.id),
      clearAll: () => this.clearAll()
    });
  }

  /**
   * 设置活动项
   */
  setActiveItem(index) {
    this.activeItemIndex = index;
    if (this.onActiveItemChange) {
      this.onActiveItemChange(index);
    }
    this.render();
  }

  /**
   * 从后端刷新数据
   */
  async refreshFromBackend() {
    let retries = 3;

    while (retries > 0) {
      try {
        const history = await invoke('get_clipboard_history');
        this.setData(history);
        if (this.onDataChange) {
          this.onDataChange(history);
        }
        this.render();
        return;
      } catch (error) {
        console.error('刷新剪贴板历史失败:', error);
        retries--;
        if (retries > 0) {
          await new Promise(resolve => setTimeout(resolve, 200));
        }
      }
    }
  }

  /**
   * 增量添加项目
   */
  addItemIncremental(item, isNew) {
    if (isNew) {
      const newData = [item, ...this.data];
      this.setData(newData);
      if (this.onDataChange) {
        this.onDataChange(newData);
      }
    } else {
      const newData = this.data.filter(
        existingItem => existingItem.content !== item.content
      );
      newData.unshift(item);
      this.setData(newData);
      if (this.onDataChange) {
        this.onDataChange(newData);
      }
    }

    this.render();
  }
}

