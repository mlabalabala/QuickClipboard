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
          case 'Enter':
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

// 获取当前标签页的项目列表
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

// 向上导航
function navigateUp() {
  const items = getCurrentTabItems();
  if (items.length === 0) return;

  const oldIndex = currentSelectedIndex;

  if (currentSelectedIndex === -1) {
    currentSelectedIndex = items.length - 1;
  } else if (currentSelectedIndex <= 0) {
    currentSelectedIndex = items.length - 1;
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
  const items = getCurrentTabItems();
  if (items.length === 0) return;

  const oldIndex = currentSelectedIndex;

  if (currentSelectedIndex === -1) {
    currentSelectedIndex = 0;
  } else if (currentSelectedIndex >= items.length - 1) {
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

  const items = getCurrentTabItems();

  requestAnimationFrame(() => {
    let selectedItem = null;

    items.forEach((item, index) => {
      if (index === currentSelectedIndex) {
        if (!item.classList.contains('keyboard-selected')) {
          item.classList.add('keyboard-selected');
          selectedItem = item;
        }
      } else {
        item.classList.remove('keyboard-selected');
      }
    });

    if (selectedItem && !isElementInViewport(selectedItem)) {
      selectedItem.scrollIntoView({
        behavior: 'auto',
        block: 'nearest',
        inline: 'nearest'
      });
    }

    isUpdating = false;
  });
}

// 检查元素是否在视口内
function isElementInViewport(element) {
  const rect = element.getBoundingClientRect();
  const container = element.closest('.clipboard-list, .quick-texts-list');

  if (!container) return false;

  const containerRect = container.getBoundingClientRect();

  return (
    rect.top >= containerRect.top &&
    rect.bottom <= containerRect.bottom &&
    rect.left >= containerRect.left &&
    rect.right <= containerRect.right
  );
}

// 执行当前选中项
async function executeCurrentItem() {
  const items = getCurrentTabItems();
  if (currentSelectedIndex < 0 || currentSelectedIndex >= items.length) return;

  const selectedItem = items[currentSelectedIndex];

  try {
    const clickEvent = new MouseEvent('click', {
      bubbles: true,
      cancelable: true,
      view: window
    });

    selectedItem.dispatchEvent(clickEvent);
  } catch (error) {
    console.error('执行选中项失败:', error);
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
  const items = getCurrentTabItems();
  if (currentSelectedIndex >= items.length) {
    currentSelectedIndex = Math.max(-1, items.length - 1);
  }

  if (navigationMode && items.length > 0 && currentSelectedIndex >= 0) {
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
  const items = getCurrentTabItems();
  const clickedIndex = items.indexOf(clickedElement);

  if (clickedIndex !== -1) {
    resetNavigation();
    currentSelectedIndex = clickedIndex;
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


