import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { togglePin } from './window.js';

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

// 分组侧边栏状态
let isGroupSidebarVisible = false;
let ctrlPressed = false;

// 快捷键帮助面板状态
let footer = null;
let shortcutsHelpContent = null;
let shortcutsHelpClose = null;
let isFirstLaunch = false;

// 初始化导航系统
export async function initNavigation() {
  try {
    // 监听导航按键事件
    await listen('navigation-key-pressed', (event) => {
      const { key } = event.payload;

      // 处理Ctrl组合键
      if (key.startsWith('Ctrl')) {
        handleCtrlCombination(key);
        return;
      }

      // 对于方向键使用节流，其他按键立即执行
      if (key === 'ArrowUp' || key === 'ArrowDown') {
        handleThrottledNavigation(key);
      } else if (key === 'ArrowLeft' || key === 'ArrowRight') {
        // 左右方向键用于切换标签页，立即执行
        handleTabSwitch(key);
      } else {
        // 立即执行非导航按键
        switch (key) {
          case 'Escape':
            hideWindow();
            break;
          case 'Tab':
            focusSearchBox();
            break;
        }
      }
    });

    // 设置点击同步
    setupClickSync();

    // 设置搜索框键盘事件监听
    setupSearchBoxKeyboardEvents();
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

// 处理Ctrl组合键
function handleCtrlCombination(key) {
  switch (key) {
    case 'CtrlEnter':
      // Ctrl+Enter：执行当前选中项目
      executeCurrentItem();
      break;
    case 'CtrlArrowUp':
      // Ctrl+上方向键：切换到上一个分组并临时显示分组列表
      switchToPreviousGroup();
      showGroupSidebarTemporarily();
      break;
    case 'CtrlArrowDown':
      // Ctrl+下方向键：切换到下一个分组并临时显示分组列表
      switchToNextGroup();
      showGroupSidebarTemporarily();
      break;
    case 'CtrlP':
      // Ctrl+P：切换窗口固定状态
      togglePin();
      break;
    case 'CtrlArrowLeft':
    case 'CtrlArrowRight':
      // Ctrl+左右方向键：暂时不处理，避免与标签页切换冲突
      break;
  }
}

// 临时显示分组侧边栏1秒
let groupSidebarTimer = null;

function showGroupSidebarTemporarily() {
  const sidebar = document.getElementById('groups-sidebar');
  if (!sidebar) return;

  // 如果侧边栏已经固定，不需要临时显示
  if (sidebar.classList.contains('pinned')) return;

  // 显示侧边栏
  sidebar.classList.add('show');
  isGroupSidebarVisible = true;

  // 清除之前的定时器
  if (groupSidebarTimer) {
    clearTimeout(groupSidebarTimer);
  }

  // 设置0.5秒后自动隐藏
  groupSidebarTimer = setTimeout(() => {
    if (sidebar && !sidebar.classList.contains('pinned')) {
      sidebar.classList.remove('show');
      isGroupSidebarVisible = false;
    }
    groupSidebarTimer = null;
  }, 500);
}

// 切换到上一个分组
function switchToPreviousGroup() {
  // 确保在常用文本标签页
  const activeTab = document.querySelector('.tab-button.active');
  if (!activeTab || activeTab.dataset.tab !== 'quick-texts') {
    // 切换到常用文本标签页
    const quickTextsTab = document.querySelector('[data-tab="quick-texts"]');
    if (quickTextsTab) {
      quickTextsTab.click();
    }
  }

  // 获取分组列表
  const groupItems = document.querySelectorAll('.group-item');
  if (groupItems.length === 0) return;

  // 找到当前激活的分组
  let currentGroupIndex = -1;
  groupItems.forEach((item, index) => {
    if (item.classList.contains('active')) {
      currentGroupIndex = index;
    }
  });

  // 切换到上一个分组
  let previousGroupIndex;
  if (currentGroupIndex <= 0) {
    previousGroupIndex = groupItems.length - 1; // 循环到最后一个
  } else {
    previousGroupIndex = currentGroupIndex - 1;
  }

  // 点击目标分组
  if (groupItems[previousGroupIndex]) {
    groupItems[previousGroupIndex].click();
  }
}

// 切换到下一个分组
function switchToNextGroup() {
  // 确保在常用文本标签页
  const activeTab = document.querySelector('.tab-button.active');
  if (!activeTab || activeTab.dataset.tab !== 'quick-texts') {
    // 切换到常用文本标签页
    const quickTextsTab = document.querySelector('[data-tab="quick-texts"]');
    if (quickTextsTab) {
      quickTextsTab.click();
    }
  }

  // 获取分组列表
  const groupItems = document.querySelectorAll('.group-item');
  if (groupItems.length === 0) return;

  // 找到当前激活的分组
  let currentGroupIndex = -1;
  groupItems.forEach((item, index) => {
    if (item.classList.contains('active')) {
      currentGroupIndex = index;
    }
  });

  // 切换到下一个分组
  let nextGroupIndex;
  if (currentGroupIndex >= groupItems.length - 1) {
    nextGroupIndex = 0; // 循环到第一个
  } else {
    nextGroupIndex = currentGroupIndex + 1;
  }

  // 点击目标分组
  if (groupItems[nextGroupIndex]) {
    groupItems[nextGroupIndex].click();
  }
}



// 处理标签页切换
function handleTabSwitch(key) {
  const tabs = document.querySelectorAll('.tab-button');
  if (tabs.length === 0) return;

  // 找到当前激活的标签页
  let currentTabIndex = -1;
  tabs.forEach((tab, index) => {
    if (tab.classList.contains('active')) {
      currentTabIndex = index;
    }
  });

  if (currentTabIndex === -1) return;

  let nextTabIndex;
  if (key === 'ArrowLeft') {
    // 向左切换，循环到最后一个
    nextTabIndex = currentTabIndex === 0 ? tabs.length - 1 : currentTabIndex - 1;
  } else if (key === 'ArrowRight') {
    // 向右切换，循环到第一个
    nextTabIndex = currentTabIndex === tabs.length - 1 ? 0 : currentTabIndex + 1;
  }

  // 点击目标标签页来切换
  if (nextTabIndex !== undefined && tabs[nextTabIndex]) {
    tabs[nextTabIndex].click();
    // 重置导航状态，因为切换了标签页
    resetNavigation();
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

// 聚焦搜索框
async function focusSearchBox() {
  const activeTab = document.querySelector('.tab-content.active');
  if (!activeTab) return;

  let searchInput = null;

  if (activeTab.id === 'clipboard-tab') {
    searchInput = document.querySelector('#search-input');
  } else if (activeTab.id === 'quick-texts-tab') {
    searchInput = document.querySelector('#quick-texts-search');
  }

  if (searchInput) {
    // 先确保窗口获得焦点，现在后端会正确处理焦点记录
    await invoke('focus_clipboard_window');

    // 然后聚焦输入框
    searchInput.focus();

    // 选中搜索框中的所有文本，方便用户直接输入新的搜索内容
    searchInput.select();
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

// 设置搜索框键盘事件监听
function setupSearchBoxKeyboardEvents() {
  const searchInputs = [
    document.querySelector('#search-input'),
    document.querySelector('#quick-texts-search')
  ];

  searchInputs.forEach(searchInput => {
    if (searchInput) {
      searchInput.addEventListener('keydown', (event) => {
        // 监听方向键，让搜索框失去焦点以便进入导航模式
        if (event.key === 'ArrowUp' || event.key === 'ArrowDown' || event.key === 'Enter') {
          event.preventDefault(); // 阻止默认的光标移动行为
          searchInput.blur(); // 让搜索框失去焦点

          // 稍微延迟一下，确保焦点已经失去，然后触发导航
          setTimeout(() => {
            if (event.key === 'ArrowUp') {
              navigateUp();
            } else if (event.key === 'ArrowDown') {
              navigateDown();
            }
          }, 10);
        }

        // ESC键也让搜索框失去焦点
        else if (event.key === 'Escape') {
          event.preventDefault();
          searchInput.blur();
        }
      });
    }
  });
}

// 初始化快捷键帮助面板
export function initShortcutsHelpPanel() {
  footer = document.getElementById('footer');
  shortcutsHelpContent = document.getElementById('shortcuts-help-content');
  shortcutsHelpClose = document.getElementById('shortcuts-help-close');

  if (!footer || !shortcutsHelpContent || !shortcutsHelpClose) {
    return;
  }

  // 检查是否是首次启动
  checkFirstLaunch();

  // 点击关闭按钮
  shortcutsHelpClose.addEventListener('click', (e) => {
    console.log('用户点击关闭按钮');
    e.stopPropagation();
    hideShortcutsHelp();
  });

  // 监听footer的鼠标离开事件，延迟移除隐藏状态
  footer.addEventListener('mouseleave', () => {
    setTimeout(() => {
      // 检查鼠标是否还在帮助内容上
      if (!shortcutsHelpContent.matches(':hover')) {
        shortcutsHelpContent.classList.remove('hidden');
      }
    }, 100);
  });

  // 首次启动时自动显示
  if (isFirstLaunch) {
    setTimeout(() => {
      showShortcutsHelpFirstTime();
    }, 1000); // 延迟1秒显示，让用户先看到主界面
  }
}

// 检查是否是首次启动
function checkFirstLaunch() {
  const hasShownHelp = localStorage.getItem('shortcuts-help-shown');
  if (!hasShownHelp) {
    isFirstLaunch = true;
    localStorage.setItem('shortcuts-help-shown', 'true');
  }
}

// 首次显示快捷键帮助
function showShortcutsHelpFirstTime() {
  if (!shortcutsHelpContent) return;

  // 添加首次显示的特殊样式
  shortcutsHelpContent.classList.add('first-show');
  shortcutsHelpContent.style.opacity = '1';
  shortcutsHelpContent.style.visibility = 'visible';
  shortcutsHelpContent.style.transform = 'translateY(0)';

  // 3秒后自动隐藏
  setTimeout(() => {
    hideShortcutsHelp();
  }, 3000);
}

// 隐藏快捷键帮助面板
function hideShortcutsHelp() {
  if (!shortcutsHelpContent) return;

  console.log('隐藏快捷键帮助面板');
  shortcutsHelpContent.classList.remove('first-show');
  shortcutsHelpContent.classList.add('hidden');

  // 清除内联样式
  shortcutsHelpContent.style.opacity = '';
  shortcutsHelpContent.style.visibility = '';
  shortcutsHelpContent.style.transform = '';
}