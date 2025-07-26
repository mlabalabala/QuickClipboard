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

// 初始化拖拽排序
export function initSortable() {
  // 初始化剪贴板列表拖拽排序
  if (clipboardList) {
    const clipboardSortable = Sortable.create(clipboardList, {
      animation: 150,
      ghostClass: 'sortable-ghost',
      chosenClass: 'sortable-chosen',
      dragClass: 'sortable-drag',
      onStart: function (evt) {
        // 拖拽开始时禁用点击事件
        setIsDragging(true);
      },
      onEnd: function (evt) {
        // 拖拽结束后重新启用点击事件
        setTimeout(() => {
          setIsDragging(false);
        }, 100);

        // 如果位置发生变化，更新剪贴板历史顺序
        if (evt.oldIndex !== evt.newIndex) {
          updateClipboardOrder(evt.oldIndex, evt.newIndex);
        }
      }
    });
    setClipboardSortable(clipboardSortable);
  }

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
