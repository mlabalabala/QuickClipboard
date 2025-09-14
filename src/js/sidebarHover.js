// 侧边栏悬停延迟控制
let sidebarHoverTimer = null;
let sidebarHideTimer = null;
const HOVER_DELAY = 500; // 0.5秒延迟
const HIDE_DELAY = 0; //秒延迟隐藏

// DOM元素
let sidebarTrigger;
let groupsSidebar;
let isSidebarVisible = false;

// 初始化侧边栏悬停功能
export function initSidebarHover() {
  sidebarTrigger = document.getElementById('sidebar-trigger');
  groupsSidebar = document.getElementById('groups-sidebar');

  if (!sidebarTrigger || !groupsSidebar) {
    console.error('侧边栏触发区域或侧边栏元素未找到');
    return;
  }

  // 为触发区域添加鼠标进入事件
  sidebarTrigger.addEventListener('mouseenter', handleMouseEnter);

  // 为侧边栏添加鼠标进入事件
  groupsSidebar.addEventListener('mouseenter', handleMouseEnter);

  // 为触发区域添加鼠标离开事件
  sidebarTrigger.addEventListener('mouseleave', handleMouseLeave);

  // 为侧边栏添加鼠标离开事件
  groupsSidebar.addEventListener('mouseleave', handleMouseLeave);
}

// 处理鼠标进入事件
function handleMouseEnter() {
  // 如果侧边栏已被固定，不执行任何操作
  if (groupsSidebar.classList.contains('pinned')) {
    return;
  }

  // 清除之前的定时器（如果有）
  if (sidebarHoverTimer) {
    clearTimeout(sidebarHoverTimer);
    sidebarHoverTimer = null;
  }

  // 清除隐藏定时器（如果有）
  if (sidebarHideTimer) {
    clearTimeout(sidebarHideTimer);
    sidebarHideTimer = null;
  }

  // 如果侧边栏已经可见，不需要再次显示
  if (isSidebarVisible) {
    return;
  }

  // 设置新的定时器，延迟显示侧边栏
  sidebarHoverTimer = setTimeout(() => {
    showSidebar();
    isSidebarVisible = true;
  }, HOVER_DELAY);
}

// 处理鼠标离开事件
function handleMouseLeave() {
  // 如果侧边栏已被固定，不执行任何操作
  if (groupsSidebar.classList.contains('pinned')) {
    return;
  }

  // 清除定时器，防止侧边栏显示
  if (sidebarHoverTimer) {
    clearTimeout(sidebarHoverTimer);
    sidebarHoverTimer = null;
  }

  // 如果侧边栏不可见，不需要隐藏
  if (!isSidebarVisible) {
    return;
  }

  // 清除之前的隐藏定时器（如果有）
  if (sidebarHideTimer) {
    clearTimeout(sidebarHideTimer);
    sidebarHideTimer = null;
  }

  // 设置延迟隐藏定时器，给用户时间移动鼠标到侧边栏
  sidebarHideTimer = setTimeout(() => {
    hideSidebar();
    isSidebarVisible = false;
  }, HIDE_DELAY);
}

// 显示侧边栏
function showSidebar() {
  groupsSidebar.style.zIndex = 'var(--z-tooltip)';
  groupsSidebar.style.transform = 'translateX(0)';
}

// 隐藏侧边栏
function hideSidebar() {
  groupsSidebar.style.zIndex = '';
  groupsSidebar.style.transform = '';
  groupsSidebar.style.right = '';
  isSidebarVisible = false;
}

// 在侧边栏固定状态改变时更新悬停行为
export function updateSidebarHoverBehavior() {
  // 如果侧边栏被固定，确保它保持显示状态
  if (groupsSidebar && groupsSidebar.classList.contains('pinned')) {
    showSidebar();
    isSidebarVisible = true;
  } else {
    // 如果侧边栏不再固定，隐藏它
    hideSidebar();
  }
}