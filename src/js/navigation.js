import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// 导航状态
let currentSelectedIndex = -1;
let navigationMode = false;
let currentTabItems = [];

// 节流相关变量
let lastNavigationTime = 0;
let navigationThrottleDelay = 16;
let pendingNavigationUpdate = null;
let isUpdating = false;
let clickSyncSetup = false;
let preserveNavigationOnUpdate = false;

// 初始化导航系统
export async function initNavigation() {
  try {
    // 监听导航按键事件
    await listen('navigation-key-pressed', (event) => {
      const { key } = event.payload;

      // 对于方向键使用节流，其他按键立即执行
      if (key === 'ArrowUp' || key === 'ArrowDown') {
        handleThrottledNavigation(key);
      } else {
        // 立即执行非导航按键
        switch (key) {
          case 'CtrlEnter':
            executeCurrentItem();
            break;
          case 'Escape':
            hideWindow();
            break;
        }
      }
    });

    // 设置点击同步
    setupClickSync();
  } catch (error) {
    console.error('初始化导航系统失败:', error);
  }
}

// 节流处理导航按键
function handleThrottledNavigation(key) {
  const now = Date.now();

  // 如果距离上次导航时间太短，则延迟执行
  if (now - lastNavigationTime < navigationThrottleDelay) {
    // 取消之前的延迟执行
    if (pendingNavigationUpdate) {
      clearTimeout(pendingNavigationUpdate);
    }

    // 设置新的延迟执行
    pendingNavigationUpdate = setTimeout(() => {
      executeNavigation(key);
      pendingNavigationUpdate = null;
    }, navigationThrottleDelay - (now - lastNavigationTime));
  } else {
    // 立即执行
    executeNavigation(key);
  }
}

// 执行导航操作
function executeNavigation(key) {
  lastNavigationTime = Date.now();

  switch (key) {
    case 'ArrowUp':
      navigateUp();
      break;
    case 'ArrowDown':
      navigateDown();
      break;
  }
}

// 获取当前标签页的项目列表（DOM元素）
function getCurrentTabItems() {
  const activeTab = document.querySelector('.tab-content.active');
  if (!activeTab) return [];

  if (activeTab.id === 'clipboard-tab') {
    return Array.from(activeTab.querySelectorAll('.clipboard-item'));
  } else if (activeTab.id === 'quick-texts-tab') {
    return Array.from(activeTab.querySelectorAll('.quick-text-item'));
  }

  return [];
}

// 获取当前标签页的数据长度
function getCurrentTabDataLength() {
  const activeTab = document.querySelector('.tab-content.active');
  if (!activeTab) return 0;

  if (activeTab.id === 'clipboard-tab') {
    // 获取剪贴板虚拟列表实例
    const clipboardModule = window.clipboardModule;
    if (clipboardModule && clipboardModule.clipboardVirtualList) {
      return clipboardModule.clipboardVirtualList.getDataLength();
    }
  } else if (activeTab.id === 'quick-texts-tab') {
    // 获取常用文本虚拟列表实例
    const quickTextsModule = window.quickTextsModule;
    if (quickTextsModule && quickTextsModule.quickTextsVirtualList) {
      return quickTextsModule.quickTextsVirtualList.getDataLength();
    }
  }

  return 0;
}

// 获取当前标签页的虚拟列表实例
function getCurrentVirtualList() {
  const activeTab = document.querySelector('.tab-content.active');
  if (!activeTab) return null;

  if (activeTab.id === 'clipboard-tab') {
    const clipboardModule = window.clipboardModule;
    return clipboardModule && clipboardModule.clipboardVirtualList ? clipboardModule.clipboardVirtualList : null;
  } else if (activeTab.id === 'quick-texts-tab') {
    const quickTextsModule = window.quickTextsModule;
    return quickTextsModule && quickTextsModule.quickTextsVirtualList ? quickTextsModule.quickTextsVirtualList : null;
  }

  return null;
}

// 向上导航
function navigateUp() {
  const dataLength = getCurrentTabDataLength();
  if (dataLength === 0) return;

  const oldIndex = currentSelectedIndex;

  if (currentSelectedIndex === -1) {
    currentSelectedIndex = dataLength - 1;
  } else if (currentSelectedIndex <= 0) {
    currentSelectedIndex = dataLength - 1;
  } else {
    currentSelectedIndex--;
  }

  if (oldIndex !== currentSelectedIndex) {
    updateSelection();
    navigationMode = true;
  }
}

// 向下导航
function navigateDown() {
  const dataLength = getCurrentTabDataLength();
  if (dataLength === 0) return;

  const oldIndex = currentSelectedIndex;

  if (currentSelectedIndex === -1) {
    currentSelectedIndex = 0;
  } else if (currentSelectedIndex >= dataLength - 1) {
    currentSelectedIndex = 0;
  } else {
    currentSelectedIndex++;
  }

  if (oldIndex !== currentSelectedIndex) {
    updateSelection();
    navigationMode = true;
  }
}

// 更新选择状态
function updateSelection() {
  if (isUpdating) return;
  isUpdating = true;

  const virtualList = getCurrentVirtualList();
  const dataLength = getCurrentTabDataLength();

  // 确保索引在有效范围内
  if (currentSelectedIndex < 0 || currentSelectedIndex >= dataLength) {
    isUpdating = false;
    return;
  }

  requestAnimationFrame(() => {
    // 首先检查目标元素是否已经在DOM中
    const items = getCurrentTabItems();
    let targetElementExists = false;

    items.forEach((item) => {
      const dataIndex = parseInt(item.getAttribute('data-index'));
      if (dataIndex === currentSelectedIndex) {
        targetElementExists = true;
      }
    });

    // 只有当目标元素不存在时才滚动
    let didScroll = false;
    if (!targetElementExists && virtualList) {
      didScroll = virtualList.scrollToIndex(currentSelectedIndex);
    }

    // 定义更新选择状态的函数
    const updateSelectionState = () => {
      const items = getCurrentTabItems();
      let selectedItem = null;

      // 清除所有选择状态并查找目标项
      items.forEach((item) => {
        const dataIndex = parseInt(item.getAttribute('data-index'));
        if (dataIndex === currentSelectedIndex) {
          item.classList.add('keyboard-selected');
          selectedItem = item;
        } else {
          item.classList.remove('keyboard-selected');
        }
      });

      // 如果找到了选中的元素但不在视口内，进行微调滚动
      if (selectedItem && !isElementInViewport(selectedItem)) {
        selectedItem.scrollIntoView({
          behavior: 'instant',
          block: 'nearest',
          inline: 'nearest'
        });
      }

      isUpdating = false;
    };

    // 根据目标元素是否存在决定处理方式
    if (targetElementExists) {
      // 目标元素已存在，立即更新选择状态
      updateSelectionState();
    } else if (didScroll) {
      // 发生了滚动，给虚拟列表时间渲染
      setTimeout(() => updateSelectionState(), 16);
    } else {
      // 没有滚动但目标元素不存在，可能需要等待渲染
      setTimeout(() => updateSelectionState(), 8);
    }
  });
}

// 检查元素是否在视口内
function isElementInViewport(element) {
  const rect = element.getBoundingClientRect();
  const container = element.closest('.clipboard-list, .quick-texts-list');

  if (!container) return false;

  const containerRect = container.getBoundingClientRect();
  const buffer = 20; // 20像素的缓冲区域

  return (
    rect.top >= (containerRect.top - buffer) &&
    rect.bottom <= (containerRect.bottom + buffer)
  );
}

// 执行当前选中项
async function executeCurrentItem() {
  const dataLength = getCurrentTabDataLength();
  if (currentSelectedIndex < 0 || currentSelectedIndex >= dataLength) return;

  const activeTab = document.querySelector('.tab-content.active');
  if (!activeTab) return;

  try {
    // 设置标志以保持导航状态
    preserveNavigationOnUpdate = true;

    if (activeTab.id === 'clipboard-tab') {
      // 对于剪贴板，调用虚拟列表的点击处理函数
      const clipboardModule = window.clipboardModule;
      if (clipboardModule && clipboardModule.clipboardVirtualList && clipboardModule.clipboardVirtualList.onItemClick) {
        // 创建一个模拟的事件对象
        const mockEvent = {
          target: { closest: () => null },
          stopPropagation: () => { },
          preventDefault: () => { }
        };
        clipboardModule.clipboardVirtualList.onItemClick(currentSelectedIndex, mockEvent);
      }
    } else if (activeTab.id === 'quick-texts-tab') {
      // 对于常用文本，调用虚拟列表的点击处理函数
      const quickTextsModule = window.quickTextsModule;
      if (quickTextsModule && quickTextsModule.quickTextsVirtualList && quickTextsModule.quickTextsVirtualList.onItemClick) {
        // 创建一个模拟的事件对象
        const mockEvent = {
          target: { closest: () => null },
          stopPropagation: () => { },
          preventDefault: () => { }
        };
        quickTextsModule.quickTextsVirtualList.onItemClick(currentSelectedIndex, mockEvent);
      }
    }
  } catch (error) {
    console.error('执行选中项失败:', error);
    // 如果出错，重置标志
    preserveNavigationOnUpdate = false;
  }
}

// 隐藏窗口
async function hideWindow() {
  try {
    await invoke('toggle_window_visibility');
    resetNavigation();
  } catch (error) {
    console.error('隐藏窗口失败:', error);
  }
}

// 重置导航状态
export function resetNavigation() {
  currentSelectedIndex = -1;
  navigationMode = false;

  const items = getCurrentTabItems();
  items.forEach(item => {
    item.classList.remove('keyboard-selected');
  });
}

// 当标签页切换时重置导航
export function onTabSwitch() {
  resetNavigation();
}

// 当列表内容更新时重置导航
export function onListUpdate() {
  // 如果设置了保持导航状态，跳过重置逻辑
  if (preserveNavigationOnUpdate) {
    preserveNavigationOnUpdate = false;
    if (navigationMode && currentSelectedIndex >= 0) {
      updateSelection();
    }
    return;
  }

  const dataLength = getCurrentTabDataLength();
  if (currentSelectedIndex >= dataLength) {
    currentSelectedIndex = Math.max(-1, dataLength - 1);
  }

  if (navigationMode && dataLength > 0 && currentSelectedIndex >= 0) {
    updateSelection();
  }
}

// 检查是否处于导航模式
export function isNavigationMode() {
  return navigationMode;
}

// 获取当前选中索引
export function getCurrentSelectedIndex() {
  return currentSelectedIndex;
}

// 同步点击的项目到导航状态
export function syncClickedItem(clickedElement) {
  // 从data-index属性获取真实的数据索引
  const dataIndex = parseInt(clickedElement.getAttribute('data-index'));

  if (!isNaN(dataIndex) && dataIndex >= 0) {
    resetNavigation();
    currentSelectedIndex = dataIndex;
    navigationMode = true;
    updateSelection();
  }
}

// 监听点击事件来同步导航状态
export function setupClickSync() {
  if (clickSyncSetup) return;

  const clipboardList = document.querySelector('.clipboard-list');
  const quickTextsList = document.querySelector('.quick-texts-list');

  if (clipboardList) {
    clipboardList.addEventListener('click', (event) => {
      const clipboardItem = event.target.closest('.clipboard-item');
      if (clipboardItem) {
        syncClickedItem(clipboardItem);
      }
    });
  }

  if (quickTextsList) {
    quickTextsList.addEventListener('click', (event) => {
      const quickTextItem = event.target.closest('.quick-text-item');
      if (quickTextItem) {
        syncClickedItem(quickTextItem);
      }
    });
  }

  clickSyncSetup = true;
}


