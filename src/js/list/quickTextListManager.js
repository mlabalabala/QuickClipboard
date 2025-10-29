/**
 * 常用文本列表管理器
 * 管理常用文本列表的渲染和交互
 */

import { invoke } from '@tauri-apps/api/core';
import { BaseListManager } from './baseListManager.js';
import { generateQuickTextItemHTML } from './renderers/itemRenderer.js';
import { buildQuickTextContextMenu } from './contextMenuBuilder.js';
import { pasteContent, deleteItem, openTextEditor } from './common/listOperations.js';
import { pinImageToScreen, saveImageAs } from './actions/imageActions.js';
import { 
  openFileWithDefaultProgram, 
  openFileLocation, 
  copyFilePaths,
  pinImageFileFromList
} from './actions/fileActions.js';
import { showNotification } from '../notificationManager.js';
import { escapeHtml } from '../utils/formatters.js';
import { getCurrentGroupId, getGroups } from '../groups.js';
import { matchesFilter, matchesSearch } from '../utils/typeFilter.js';

export class QuickTextListManager extends BaseListManager {
  constructor(config) {
    super({
      scrollId: config.scrollId || 'quick-texts-list',
      contentId: config.contentId || 'quick-texts-content',
      renderItemHTML: (item, index) => this.renderItem(item, index),
      onItemClick: (index, event) => this.handleItemClick(index, event),
      onItemContextMenu: (index, event) => this.handleItemContextMenu(index, event),
      onSort: (oldIndex, newIndex) => this.handleSort(oldIndex, newIndex),
      sortableOptions: {
        onStart: () => this.handleDragStart(),
        onEnd: () => this.handleDragEnd()
      }
    });

    this.isOneTimePaste = config.isOneTimePaste || false;
    this.pasteWithFormat = config.pasteWithFormat || false;
    this.currentGroupId = config.currentGroupId || '全部';
    this.isDragging = false;
    
    // 外部回调
    this.onDataChange = config.onDataChange;
  }

  // 渲染单个列表项
  renderItem(item, index) {
    const groupBadgeHTML = this.generateGroupBadge(item);
    
    return generateQuickTextItemHTML(item, index, {
      pasteWithFormat: this.pasteWithFormat,
      groupBadgeHTML
    });
  }

  // 生成分组标签
  generateGroupBadge(item) {
    // 只在"全部"分组中显示分组标签
    if (this.currentGroupId !== '全部') {
      return '';
    }

    const itemGroupName = item.group_name || '全部';
    if (itemGroupName === '全部') {
      return '';
    }

    try {
      const groups = getGroups();
      const group = groups.find(g => g.name === itemGroupName);

      if (group) {
        return `
          <div class="group-badge">
            <i class="${group.icon}"></i>
            <span>${escapeHtml(group.name)}</span>
          </div>
        `;
      }
    } catch (error) {
      console.warn('获取分组信息失败:', error);
    }

    return '';
  }

  // 处理列表项点击
  async handleItemClick(index, event) {
    if (this.isDragging) return;

    const filteredData = this.getFilteredData();
    const item = filteredData[index];
    if (!item) return;

    // 执行粘贴
    await this.pasteItem(item, event.target.closest('.quick-text-item'));
  }

  // 处理列表项右键菜单
  handleItemContextMenu(index, event) {
    event.preventDefault();

    const filteredData = this.getFilteredData();
    const item = filteredData[index];
    if (item) {
      this.showContextMenu(event, item);
    }
  }

  // 处理排序
  async handleSort(oldIndex, newIndex) {
    try {
      const filteredData = this.getFilteredData();

      if (oldIndex >= filteredData.length || newIndex >= filteredData.length) {
        return;
      }

      const movedItem = filteredData[oldIndex];
      const targetItem = filteredData[newIndex];

      if (!movedItem || !movedItem.id) {
        return;
      }

      // 在"全部"分组中，检查是否跨分组拖拽
      if (this.currentGroupId === '全部') {
        const movedItemGroupId = movedItem.group_name || '全部';
        const targetItemGroupId = targetItem ? (targetItem.group_name || '全部') : movedItemGroupId;

        if (movedItemGroupId !== targetItemGroupId) {
          // 跨分组拖拽：将项目移动到目标分组
          await this.moveItemToGroup(movedItem, targetItemGroupId, filteredData, newIndex);
          return;
        }
      }

      // 同分组内的排序
      const movedItemGroupId = movedItem.group_name || '全部';
      let targetIndexInGroup = newIndex;

      if (this.currentGroupId === '全部') {
        targetIndexInGroup = this.calculateTargetPositionInGroup(filteredData, newIndex, movedItemGroupId);
      }

      await invoke('move_quick_text_item', {
        itemId: movedItem.id,
        toIndex: targetIndexInGroup
      });

      await this.refreshFromBackend();

    } catch (error) {
      console.error('更新常用文本顺序失败:', error);
      await this.refreshFromBackend();
    }
  }

  // 跨分组移动项目
  async moveItemToGroup(movedItem, targetGroupId, filteredData, newIndex) {
    try {
      // 计算在目标分组内的正确位置
      const targetPositionInGroup = this.calculateTargetPositionInGroup(filteredData, newIndex, targetGroupId);

      // 先移动到目标分组
      await invoke('move_quick_text_to_group', {
        id: movedItem.id,
        groupName: targetGroupId
      });

      // 在目标分组内排序到特定位置
      await invoke('move_quick_text_item', {
        itemId: movedItem.id,
        toIndex: targetPositionInGroup
      });

      // 显示成功提示
      const groups = getGroups();
      const targetGroupName = groups.find(g => g.id === targetGroupId)?.name || '分组';
      showNotification(`已移动到 ${targetGroupName}`, 'success');

      await this.refreshFromBackend();
    } catch (error) {
      console.error('跨分组移动失败:', error);
      showNotification('移动到分组失败，请重试', 'error');
    }
  }

  // 计算在目标分组内的正确位置
  calculateTargetPositionInGroup(filteredData, newIndex, targetGroupId) {
    const targetGroupItems = [];
    let targetIndexInGroup = 0;

    for (let i = 0; i < filteredData.length; i++) {
      const item = filteredData[i];
      const itemGroupId = item.group_name || 'all';

      if (itemGroupId === targetGroupId) {
        targetGroupItems.push({ item, originalIndex: i });

        if (i <= newIndex) {
          targetIndexInGroup = targetGroupItems.length;
        }
      }
    }

    return Math.max(0, Math.min(targetIndexInGroup - 1, targetGroupItems.length - 1));
  }

  // 处理拖拽开始
  handleDragStart() {
    this.isDragging = true;
    document.querySelector('.tab-content.active')?.classList.add('dragging');
    const sidebar = document.getElementById('groups-sidebar');
    if (sidebar && !sidebar.classList.contains('pinned')) {
      sidebar.classList.add('show');
    }
  }

  // 处理拖拽结束
  handleDragEnd() {
    this.isDragging = false;
    document.querySelector('.tab-content.active')?.classList.remove('dragging');
    const sidebar = document.getElementById('groups-sidebar');
    if (sidebar && !sidebar.classList.contains('pinned')) {
      sidebar.classList.remove('show');
    }
  }

  // 粘贴项目
  async pasteItem(item, element = null) {
    await pasteContent(
      { quick_text_id: item.id },
      {
        element,
        onSuccess: async () => {
          // 一次性粘贴：删除该项
          if (this.isOneTimePaste) {
            setTimeout(() => this.deleteItem(item.id), 100);
          }
        }
      }
    );
  }

  // 删除项目
  async deleteItem(id) {
    const { showConfirmModal } = await import('../ui.js');
    
    showConfirmModal('确认删除', '确定要删除这个常用文本吗？', async () => {
      await deleteItem(id, 'delete_quick_text', async () => {
        const newData = this.data.filter(item => item.id !== id);
        this.setData(newData);
        if (this.onDataChange) {
          this.onDataChange(newData);
        }
        this.render();
        showNotification('已删除常用文本', 'success');
      });
    });
  }

  // 显示上下文菜单
  showContextMenu(event, item) {
    buildQuickTextContextMenu(event, item, {
      pinImage: () => pinImageToScreen(item),
      saveImage: () => saveImageAs(item),
      pinImageFile: () => pinImageFileFromList(item),
      openFile: () => openFileWithDefaultProgram(item),
      openFileLocation: () => openFileLocation(item),
      copyFilePath: () => copyFilePaths(item),
      edit: async () => {
        await openTextEditor({
          type: 'quick-text',
          id: item.id,
          title: item.title,
          content: item.content,
          groupId: item.group_name || item.groupId || '',
          timestamp: item.timestamp
        });
      },
      delete: () => this.deleteItem(item.id)
    });
  }

  // 从后端刷新数据
  async refreshFromBackend() {
    let retries = 3;

    while (retries > 0) {
      try {
        let texts;
        const currentGroupId = getCurrentGroupId();

        if (currentGroupId === '全部') {
          texts = await invoke('get_quick_texts');
        } else {
          try {
            texts = await invoke('get_quick_texts_by_group', { groupName: currentGroupId });
          } catch (groupError) {
            console.warn('按分组获取常用文本失败，回退到获取全部:', groupError);
            texts = await invoke('get_quick_texts');
          }
        }

        this.setData(texts);
        if (this.onDataChange) {
          this.onDataChange(texts);
        }
        this.render();
        return;
      } catch (error) {
        console.error('获取常用文本失败:', error);
        retries--;
        if (retries > 0) {
          await new Promise(resolve => setTimeout(resolve, 200));
        }
      }
    }

    // 如果完全失败，设置空数组
    this.setData([]);
    if (this.onDataChange) {
      this.onDataChange([]);
    }
    this.render();
  }

  // 获取过滤后的数据（重写以支持分组排序）
  getFilteredData() {
    let filteredTexts = this.data.filter(item => {
      const contentType = item.content_type || 'text';

      // 类型筛选
      if (!matchesFilter(contentType, this.filterType, item)) {
        return false;
      }

      // 搜索筛选
      return matchesSearch(item, this.searchTerm, contentType);
    });

    // 如果是"全部"分组，按分组顺序重新排列数据
    if (this.currentGroupId === '全部') {
      filteredTexts = this.sortTextsByGroupOrder(filteredTexts);
    }

    return filteredTexts;
  }

  // 按分组顺序排列文本数据
  sortTextsByGroupOrder(texts) {
    try {
      const groupsOrder = getGroups();

      // 按group_name分组
      const textsByGroup = {};
      texts.forEach(text => {
        const groupId = text.group_name || 'all';
        if (!textsByGroup[groupId]) {
          textsByGroup[groupId] = [];
        }
        textsByGroup[groupId].push(text);
      });

      // 按分组顺序合并
      const sortedTexts = [];
      groupsOrder.forEach(group => {
        if (textsByGroup[group.id]) {
          sortedTexts.push(...textsByGroup[group.id]);
        }
      });

      // 添加任何不在分组列表中的文本
      Object.keys(textsByGroup).forEach(groupId => {
        if (!groupsOrder.find(g => g.id === groupId)) {
          sortedTexts.push(...textsByGroup[groupId]);
        }
      });

      return sortedTexts;
    } catch (error) {
      console.warn('按分组顺序排列失败，使用原始顺序:', error);
      return texts;
    }
  }

  // 设置当前分组
  setCurrentGroup(groupId) {
    this.currentGroupId = groupId;
  }
}

