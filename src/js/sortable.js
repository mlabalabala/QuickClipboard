import Sortable from 'sortablejs';
import {
  clipboardList,
  quickTextsList,
  setIsDragging,
  setClipboardSortable,
  setQuickTextsSortable
} from './config.js';
import { updateClipboardOrder } from './clipboard.js';
import { updateQuickTextsOrder } from './quickTexts.js';

// 初始化剪贴板拖拽排序
export function initClipboardSortable() {
  console.log('initClipboardSortable called');
  if (!clipboardList) {
    console.error('clipboardList not found');
    return;
  }

  console.log('clipboardList found:', clipboardList);

  // 对于虚拟滚动列表，需要找到实际包含clipboard-item的容器
  // 虚拟滚动的结构是：clipboardList > scrollContainer > viewport
  const scrollContainer = clipboardList.querySelector('div');
  const viewport = scrollContainer ? scrollContainer.querySelector('div') : null;

  console.log('scrollContainer:', scrollContainer);
  console.log('viewport:', viewport);

  if (viewport) {
    const clipboardSortable = Sortable.create(viewport, {
      animation: 150,
      ghostClass: 'sortable-ghost',
      chosenClass: 'sortable-chosen',
      dragClass: 'sortable-drag',
      // 只允许拖拽clipboard-item元素
      draggable: '.clipboard-item',
      onStart: function () {
        // 拖拽开始时禁用点击事件
        setIsDragging(true);
      },
      onEnd: async function (evt) {
        setIsDragging(false);

        let oldIndex = evt.oldIndex;
        let newIndex = evt.newIndex;

        // 获取当前可见的剪贴板项目
        const clipboardItems = Array.from(viewport.querySelectorAll('.clipboard-item'));

        // 获取拖拽的元素信息并修正索引
        const draggedElement = evt.item;
        const draggedText = draggedElement.querySelector('.clipboard-text')?.textContent || '';

        // 根据拖拽元素的文本内容找到正确的索引
        const correctOldIndex = Array.from(clipboardItems).findIndex(item => {
          const itemText = item.querySelector('.clipboard-text')?.textContent || '';
          return itemText === draggedText;
        });

        if (correctOldIndex !== -1) {
          oldIndex = correctOldIndex;
        }

        // 修正newIndex越界问题
        if (newIndex >= clipboardItems.length) {
          newIndex = clipboardItems.length - 1;
        }

        // 如果位置没有变化，跳过重新排序
        if (oldIndex === newIndex) {
          return;
        }

        // 调用重新排序函数
        const { updateClipboardOrder } = await import('./clipboard.js');
        await updateClipboardOrder(oldIndex, newIndex);
      }
    });
    setClipboardSortable(clipboardSortable);
  }
}

// 初始化拖拽排序
export function initSortable() {
  // 初始化常用文本列表拖拽排序
  if (quickTextsList) {
    const quickTextsSortable = Sortable.create(quickTextsList, {
      animation: 150,
      ghostClass: 'sortable-ghost',
      chosenClass: 'sortable-chosen',
      dragClass: 'sortable-drag',
      onStart: function (evt) {
        setIsDragging(true);
      },
      onEnd: function (evt) {
        setTimeout(() => {
          setIsDragging(false);
        }, 100);

        // 如果位置发生变化，更新常用文本顺序
        if (evt.oldIndex !== evt.newIndex) {
          updateQuickTextsOrder(evt.oldIndex, evt.newIndex);
        }
      }
    });
    setQuickTextsSortable(quickTextsSortable);
  }
}
