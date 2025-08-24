// 标题栏控件拖拽模块
import Sortable from 'sortablejs';
import { forceOpenPanel, forceClosePanel, isToolsPanelOpen } from './toolsPanel.js';

let titlebarControls = null;
let toolsPanel = null;
let controlsSortable = null;
let toolsPanelSortable = null;
let wasPanelOpenBeforeDrag = false;

// 本地存储键名
const STORAGE_KEY = 'titlebar-controls-layout';

// 初始化标题栏拖拽功能
export function initTitlebarDrag() {
  // 延迟初始化，确保DOM完全加载
  setTimeout(() => {
    titlebarControls = document.querySelector('#titlebar .controls');
    toolsPanel = document.querySelector('#tools-panel .tools-panel-content');

    if (!titlebarControls || !toolsPanel) {
      return;
    }

    // 恢复保存的布局
    restoreLayout();

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
      
      // 当前容器（标题栏）保持蓝色，其他容器（工具面板）显示绿色
      titlebarControls.classList.add('drag-source'); // 蓝色边框
      toolsPanel.classList.add('drag-target'); // 绿色边框
    },

    onEnd: function(evt) {
      document.body.classList.remove('dragging');
      evt.item.classList.remove('dragging-item');
      
      // 清除所有拖拽目标样式
      clearAllDragTargets();
      
      // 保存当前布局
      saveLayout();
      
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
      
      // 当前容器（工具面板）保持蓝色，其他容器（标题栏）显示绿色
      toolsPanel.classList.add('drag-source'); // 蓝色边框
      titlebarControls.classList.add('drag-target'); // 绿色边框
    },

    onEnd: function(evt) {
      document.body.classList.remove('dragging');
      evt.item.classList.remove('dragging-item');
      
      // 清除所有拖拽目标样式
      clearAllDragTargets();
      
      // 保存当前布局
      saveLayout();
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

// 清除所有拖拽目标样式
function clearAllDragTargets() {
  const dragTargets = document.querySelectorAll('.drag-target');
  dragTargets.forEach(target => target.classList.remove('drag-target'));
  
  const dragSources = document.querySelectorAll('.drag-source');
  dragSources.forEach(source => source.classList.remove('drag-source'));
}

// 刷新拖拽功能
export function refreshTitlebarDrag() {
  destroyTitlebarDrag();
  initTitlebarDrag();
}

// 保存当前布局到本地存储
function saveLayout() {
  try {
    const layout = {
      titlebarControls: [],
      toolsPanelItems: []
    };

    // 保存标题栏控件的顺序和ID
    if (titlebarControls) {
      const buttons = titlebarControls.querySelectorAll('.control-button');
      buttons.forEach(button => {
        layout.titlebarControls.push({
          id: button.id,
          title: button.title,
          innerHTML: button.innerHTML
        });
      });
    }

    // 保存工具面板项的顺序和ID
    if (toolsPanel) {
      const toolItems = toolsPanel.querySelectorAll('.tool-item');
      toolItems.forEach(item => {
        const button = item.querySelector('.tool-button');
        if (button) {
          layout.toolsPanelItems.push({
            id: button.id,
            title: button.title,
            innerHTML: button.innerHTML,
            className: button.className
          });
        }
      });
    }

    localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
  } catch (error) {
    // 静默处理存储错误
  }
}

// 从本地存储恢复布局
function restoreLayout() {
  try {
    const savedLayout = localStorage.getItem(STORAGE_KEY);
    if (!savedLayout) return;

    const layout = JSON.parse(savedLayout);
    
    // 恢复标题栏控件
    if (layout.titlebarControls && titlebarControls) {
      restoreTitlebarControls(layout.titlebarControls);
    }

    // 恢复工具面板项
    if (layout.toolsPanelItems && toolsPanel) {
      restoreToolsPanelItems(layout.toolsPanelItems);
    }
  } catch (error) {
    // 静默处理恢复错误
  }
}

// 恢复标题栏控件
function restoreTitlebarControls(controlsData) {
  const existingButtons = Array.from(titlebarControls.querySelectorAll('.control-button'));
  
  // 创建一个映射，方便查找现有按钮
  const buttonMap = new Map();
  existingButtons.forEach(button => {
    buttonMap.set(button.id, button);
  });

  // 清空容器
  titlebarControls.innerHTML = '';

  // 按保存的顺序重新添加按钮
  controlsData.forEach(controlData => {
    const existingButton = buttonMap.get(controlData.id);
    if (existingButton) {
      titlebarControls.appendChild(existingButton);
    } else {
      // 如果按钮不存在，创建新的
      const button = document.createElement('button');
      button.id = controlData.id;
      button.className = 'control-button';
      button.title = controlData.title;
      button.innerHTML = controlData.innerHTML;
      button.setAttribute('draggable', 'true');
      titlebarControls.appendChild(button);
    }
  });
}

// 恢复工具面板项
function restoreToolsPanelItems(itemsData) {
  const existingItems = Array.from(toolsPanel.querySelectorAll('.tool-item'));
  
  // 创建一个映射，方便查找现有项
  const itemMap = new Map();
  existingItems.forEach(item => {
    const button = item.querySelector('.tool-button');
    if (button) {
      itemMap.set(button.id, item);
    }
  });

  // 清空容器
  toolsPanel.innerHTML = '';

  // 按保存的顺序重新添加项
  itemsData.forEach(itemData => {
    const existingItem = itemMap.get(itemData.id);
    if (existingItem) {
      toolsPanel.appendChild(existingItem);
    } else {
      // 如果项不存在，创建新的
      const toolItem = document.createElement('div');
      toolItem.className = 'tool-item';
      toolItem.setAttribute('draggable', 'true');
      
      const button = document.createElement('button');
      button.id = itemData.id;
      button.className = itemData.className;
      button.title = itemData.title;
      button.innerHTML = itemData.innerHTML;
      
      toolItem.appendChild(button);
      toolsPanel.appendChild(toolItem);
    }
  });
}

// 清除保存的布局
export function clearSavedLayout() {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch (error) {
    // 静默处理错误
  }
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
    wasPanelOpenBeforeDrag,
    savedLayout: localStorage.getItem(STORAGE_KEY)
  };
}

// 将测试函数暴露到全局
if (typeof window !== 'undefined') {
  window.testTitlebarDrag = testTitlebarDrag;
  window.refreshTitlebarDrag = refreshTitlebarDrag;
  window.clearSavedLayout = clearSavedLayout;
}