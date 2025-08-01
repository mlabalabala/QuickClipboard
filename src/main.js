// 主入口文件 - 协调各个模块

// =================== 启动横幅 ===================
function printStartupBanner() {
  console.log('');
  console.log('███╗   ███╗ ██████╗ ███████╗██╗  ██╗███████╗███╗   ██╗ ██████╗ ');
  console.log('████╗ ████║██╔═══██╗██╔════╝██║  ██║██╔════╝████╗  ██║██╔════╝ ');
  console.log('██╔████╔██║██║   ██║███████╗███████║█████╗  ██╔██╗ ██║██║  ███╗');
  console.log('██║╚██╔╝██║██║   ██║╚════██║██╔══██║██╔══╝  ██║╚██╗██║██║   ██║');
  console.log('██║ ╚═╝ ██║╚██████╔╝███████║██║  ██║███████╗██║ ╚████║╚██████╔╝');
  console.log('╚═╝     ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═══╝ ╚═════╝ ');
  console.log('');
  console.log('QuickClipboard v1.0.0 - 快速剪贴板管理工具');
  console.log('Author: MoSheng | Frontend: JavaScript + Vite');
  console.log('Main window initializing...');
  console.log('');
}

import { initThemeManager } from './js/themeManager.js';
import './js/fileIconUtils.js';
import { initNavigation } from './js/navigation.js';

import {
  initDOMReferences,
  setCurrentFilter,
  setCurrentQuickTextsFilter,
  setIsOneTimePaste,
  setQuickTextsCustomFilter,
  setContentCustomFilter,
  searchInput,
  contentFilter,
  contentFilterContainer,
  quickTextsSearch,
  quickTextsFilter,
  quickTextsFilterContainer,
  oneTimePasteSwitch
} from './js/config.js';

import { CustomSelect } from './js/customSelect.js';

// 自定义组件实例
let quickTextsCustomFilter;
let contentCustomFilter;

import {
  initAiTranslation
} from './js/aiTranslation.js';

import {
  refreshClipboardHistory,
  filterClipboardItems
} from './js/clipboard.js';

import {
  refreshQuickTexts,
  filterQuickTexts,
  setupQuickTexts
} from './js/quickTexts.js';



import {
  setupTabSwitching,
  setupConfirmModal,
  setupAlertModal
} from './js/ui.js';

import {
  setupClipboardEventListener,
  setupTrayEventListeners,
  setupKeyboardShortcuts,
  setupContextMenuDisable,
  setupCustomWindowDrag
} from './js/events.js';

import { initSortable } from './js/sortable.js';
import { initInputFocusManagement } from './js/focus.js';
import { setupWindowControls } from './js/window.js';
import { initGroups } from './js/groups.js';
import { initScreenshot } from './js/screenshot.js';
import {
  initializeSettingsManager,
  initializeTheme,
  setupThemeListener,
  updateShortcutDisplay
} from './js/settingsManager.js';
document.addEventListener('contextmenu', function (e) {
  e.preventDefault();
});
// 等待后端初始化完成
async function waitForBackendInitialization() {
  let attempts = 0;
  const maxAttempts = 50; // 最多等待5秒

  while (attempts < maxAttempts) {
    try {
      const isInitialized = await invoke('is_backend_initialized');
      if (isInitialized) {
        return;
      }
    } catch (error) {
      // 静默处理错误
    }

    // 等待100ms后重试
    await new Promise(resolve => setTimeout(resolve, 100));
    attempts++;
  }
}

// 初始化应用
async function initApp() {

  // 设置自定义窗口拖拽
  setupCustomWindowDrag();

  // 等待后端初始化完成，然后获取数据
  await waitForBackendInitialization();

  // 输出启动横幅
  printStartupBanner();

  // 初始化DOM元素引用
  initDOMReferences();

  // 初始化设置管理器
  await initializeSettingsManager();

  // 更新快捷键显示
  updateShortcutDisplay();

  // 初始化主题管理器（必须等待完成）
  await initThemeManager();

  // 初始化主题（同步主题管理器的状态）
  initializeTheme();

  // 设置主题监听器
  setupThemeListener();

  // 初始化分组功能（必须在常用文本之前）
  await initGroups();

  // 获取剪贴板历史
  await refreshClipboardHistory();
  // 获取常用文本
  await refreshQuickTexts();

  // 设置搜索功能
  searchInput.addEventListener('input', filterClipboardItems);
  quickTextsSearch.addEventListener('input', filterQuickTexts);

  // 共享的筛选器选项配置
  const filterOptions = [
    { value: 'all', text: '全部' },
    { value: 'text', text: '文本' },
    { value: 'image', text: '图片' },
    { value: 'files', text: '文件' },
    { value: 'link', text: '链接' }
  ];

  // 初始化自定义剪贴板筛选器
  contentCustomFilter = new CustomSelect(contentFilterContainer, {
    options: filterOptions,
    value: 'all',
    onChange: (value) => {
      setCurrentFilter(value);
      filterClipboardItems();
    }
  });

  // 初始化自定义常用文本筛选器
  quickTextsCustomFilter = new CustomSelect(quickTextsFilterContainer, {
    options: filterOptions,
    value: 'all',
    onChange: (value) => {
      setCurrentQuickTextsFilter(value);
      filterQuickTexts();
    }
  });

  // 将自定义组件实例设置到config中
  setContentCustomFilter(contentCustomFilter);
  setQuickTextsCustomFilter(quickTextsCustomFilter);

  // 设置一次性粘贴开关
  if (oneTimePasteSwitch) {
    oneTimePasteSwitch.addEventListener('change', (e) => {
      setIsOneTimePaste(e.target.checked);
    });
  }

  // 初始化AI翻译功能
  await initAiTranslation();

  // 设置标签页切换
  setupTabSwitching();

  // 设置常用文本功能
  setupQuickTexts();

  // 设置UI模态框
  setupConfirmModal();
  setupAlertModal();



  // 设置窗口控制按钮
  setupWindowControls();

  // 监听剪贴板变化事件
  setupClipboardEventListener();

  // 监听托盘事件
  setupTrayEventListeners();

  // 设置键盘快捷键
  // setupKeyboardShortcuts();

  // 初始化拖拽排序
  initSortable();

  // 初始化输入框焦点管理
  initInputFocusManagement();

  // 初始化导航系统
  await initNavigation();

  // 初始化截屏功能
  initScreenshot();

  // 设置右键菜单禁用
  setupContextMenuDisable();

  // 监听常用文本刷新事件
  window.addEventListener('refreshQuickTexts', refreshQuickTexts);

  // 监听分组变化事件
  window.addEventListener('groupChanged', refreshQuickTexts);

  // 设置窗口可见性监听器
  setupWindowVisibilityListener();

  // 设置窗口动画监听器
  setupWindowAnimationListeners();
}

// 设置窗口可见性监听器
function setupWindowVisibilityListener() {
  // 监听页面可见性变化
  document.addEventListener('visibilitychange', () => {
    updateShortcutDisplay();
    if (!document.hidden) {
      // 页面变为可见时，更新快捷键显示
      updateShortcutDisplay();
    }
  });

  // 监听窗口焦点事件
  window.addEventListener('focus', () => {
    // 窗口获得焦点时，更新快捷键显示
    updateShortcutDisplay();
  });
}

// 设置窗口动画监听器
async function setupWindowAnimationListeners() {
  try {
    console.log('开始设置窗口动画监听器...');
    const { listen } = await import('@tauri-apps/api/event');

    // 监听窗口显示动画事件
    await listen('window-show-animation', () => {
      console.log('收到窗口显示动画事件');
      playWindowShowAnimation();
    });

    // 监听窗口隐藏动画事件
    await listen('window-hide-animation', () => {
      console.log('收到窗口隐藏动画事件');
      playWindowHideAnimation();
    });

    console.log('窗口动画监听器设置完成');
  } catch (error) {
    console.error('设置窗口动画监听器失败:', error);
  }
}

// 播放窗口显示动画
function playWindowShowAnimation() {
  const container = document.querySelector('body');
  if (!container) return;

  // 重置动画状态
  container.classList.remove('window-hide-animation', 'window-show-animation');

  // 强制重绘
  container.offsetHeight;

  // 添加显示动画类
  container.classList.add('window-show-animation');
}

// 播放窗口隐藏动画
function playWindowHideAnimation() {
  const container = document.querySelector('body');
  if (!container) return;

  // 重置动画状态
  container.classList.remove('window-hide-animation', 'window-show-animation');

  // 强制重绘
  container.offsetHeight;

  // 添加隐藏动画类
  container.classList.add('window-hide-animation');
}

// 页面加载完成后初始化
window.addEventListener('DOMContentLoaded', () => {
  // 初始化应用
  initApp();
});

