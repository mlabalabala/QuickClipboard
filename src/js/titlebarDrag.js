// 标题栏控件拖拽模块
import Sortable from 'sortablejs';
import { forceOpenPanel, forceClosePanel, isToolsPanelOpen } from './toolsPanel.js';

let titlebarControls = null;
let toolsPanel = null;
let controlsSortable = null;
let toolsPanelSortable = null;
let wasPanelOpenBeforeDrag = false;

// 初始化标题栏拖拽功能
export function initTitlebarDrag() {
  // 延迟初始化，确保DOM完全加载
  setTimeout(() => {
    titlebarControls = document.querySelector('#titlebar .controls');
    toolsPanel = document.querySelector('#tools-panel .tools-panel-content');

    if (!titlebarControls || !toolsPanel) {
      return;
    }

    // 初始化标题栏控件拖拽
    initControlsSortable();
    
    // 初始化工具面板拖拽
    initToolsPanelSortable();
  }, 200);
}

// 初始化标题栏控件的拖拽
function initControlsSortable() {
  if (!titlebarControls) return;

  const options = {
    group: 'shared-tools',
    animation: 200,
    ghostClass: 'sortable-ghost',
    chosenClass: 'sortable-chosen',
    dragClass: 'sortable-drag',
    sort: true,
    
    // 过滤掉工具面板切换按钮，不允许拖拽
    filter: function(evt, item, originalEvent) {
      return item.id === 'tools-panel-toggle';
    },
    
    onStart: function(evt) {
      // 记录拖拽前的面板状态
      wasPanelOpenBeforeDrag = isToolsPanelOpen();
      
      // 自动展开工具面板
      if (!wasPanelOpenBeforeDrag) {
        forceOpenPanel();
      }
      
      document.body.classList.add('dragging');
      evt.item.classList.add('dragging-item');
      
      // 确保两个容器都显示拖拽目标边框
      titlebarControls.classList.add('drag-target');
      toolsPanel.classList.add('drag-target');
    },

    onEnd: function(evt) {
      document.body.classList.remove('dragging');
      evt.item.classList.remove('dragging-item');
      
      // 清除拖拽目标边框
      titlebarControls.classList.remove('drag-target');
      toolsPanel.classList.remove('drag-target');
      
      // 如果拖拽前面板是关闭的，拖拽结束后自动关闭
      if (!wasPanelOpenBeforeDrag) {
        // 延迟关闭，给用户一点时间看到结果
        setTimeout(() => {
          forceClosePanel();
        }, 500);
      }
    },

    onAdd: function(evt) {
      // 转换工具项为控制按钮
      convertToolToControl(evt.item);
    },

    onRemove: function(evt) {
      // 元素从标题栏移除
    }
  };

  controlsSortable = Sortable.create(titlebarControls, options);
}

// 初始化工具面板的拖拽
function initToolsPanelSortable() {
  if (!toolsPanel) return;

  const options = {
    group: 'shared-tools',
    animation: 200,
    ghostClass: 'sortable-ghost',
    chosenClass: 'sortable-chosen',
    dragClass: 'sortable-drag',
    sort: true,
    
    onStart: function(evt) {
      document.body.classList.add('dragging');
      evt.item.classList.add('dragging-item');
      
      // 确保两个容器都显示拖拽目标边框
      titlebarControls.classList.add('drag-target');
      toolsPanel.classList.add('drag-target');
    },

    onEnd: function(evt) {
      document.body.classList.remove('dragging');
      evt.item.classList.remove('dragging-item');
      
      // 清除拖拽目标边框
      titlebarControls.classList.remove('drag-target');
      toolsPanel.classList.remove('drag-target');
    },

    onAdd: function(evt) {
      // 转换控制按钮为工具项
      convertControlToTool(evt.item);
    },

    onRemove: function(evt) {
      // 元素从工具面板移除
    }
  };

  toolsPanelSortable = Sortable.create(toolsPanel, options);
}

// 将工具项转换为控制按钮
function convertToolToControl(element) {
  if (element.classList.contains('tool-item')) {
    const button = element.querySelector('.tool-button');
    if (button) {
      // 转换样式
      button.classList.remove('tool-button');
      button.classList.add('control-button');
      
      // 设置拖拽属性
      button.setAttribute('draggable', 'true');
      element.removeAttribute('draggable');
      
      // 替换元素
      element.parentNode.replaceChild(button, element);
    }
  }
}

// 将控制按钮转换为工具项
function convertControlToTool(element) {
  if (element.classList.contains('control-button')) {
    // 转换样式
    element.classList.remove('control-button');
    element.classList.add('tool-button');
    
    // 创建工具项容器
    const toolItem = document.createElement('div');
    toolItem.className = 'tool-item';
    toolItem.setAttribute('draggable', 'true');
    
    // 移除按钮的拖拽属性
    element.removeAttribute('draggable');
    
    // 包装按钮
    element.parentNode.insertBefore(toolItem, element);
    toolItem.appendChild(element);
  }
}

// 销毁拖拽功能
export function destroyTitlebarDrag() {
  if (controlsSortable) {
    controlsSortable.destroy();
    controlsSortable = null;
  }
  
  if (toolsPanelSortable) {
    toolsPanelSortable.destroy();
    toolsPanelSortable = null;
  }
  
  // 清理拖拽状态
  document.body.classList.remove('dragging');
  const draggingItems = document.querySelectorAll('.dragging-item');
  draggingItems.forEach(item => item.classList.remove('dragging-item'));
  
  // 清理拖拽目标样式
  const dragTargets = document.querySelectorAll('.drag-target');
  dragTargets.forEach(target => target.classList.remove('drag-target'));
}

// 刷新拖拽功能
export function refreshTitlebarDrag() {
  destroyTitlebarDrag();
  initTitlebarDrag();
}

// 测试函数 - 在浏览器控制台中使用（仅返回状态信息）
export function testTitlebarDrag() {
  const titlebarControls = document.querySelector('#titlebar .controls');
  const toolsPanel = document.querySelector('#tools-panel .tools-panel-content');
  
  return {
    titlebarControls,
    toolsPanel,
    controlsSortable,
    toolsPanelSortable,
    wasPanelOpenBeforeDrag
  };
}

// 将测试函数暴露到全局
if (typeof window !== 'undefined') {
  window.testTitlebarDrag = testTitlebarDrag;
  window.refreshTitlebarDrag = refreshTitlebarDrag;
}