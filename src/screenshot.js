import { invoke } from '@tauri-apps/api/core';
import { Window } from '@tauri-apps/api/window';

// 简单的通知系统
function showScreenshotNotification(message, type = 'error', duration = 3000) {
  // 移除已存在的通知
  const existingNotifications = document.querySelectorAll('.screenshot-notification');
  existingNotifications.forEach(n => n.remove());

  // 创建通知元素
  const notification = document.createElement('div');
  notification.className = 'screenshot-notification';

  // 设置样式
  notification.style.cssText = `
    position: fixed;
    top: 20px;
    right: 20px;
    padding: 12px 16px;
    border-radius: 8px;
    color: white;
    font-size: 14px;
    font-weight: 500;
    z-index: 10000;
    opacity: 0;
    transform: translateX(100%);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    max-width: 300px;
    word-wrap: break-word;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    backdrop-filter: blur(10px);
    ${type === 'error' ? 'background-color: rgba(239, 68, 68, 0.9);' : 'background-color: rgba(59, 130, 246, 0.9);'}
  `;

  notification.textContent = message;

  // 添加到页面
  document.body.appendChild(notification);

  // 显示动画
  setTimeout(() => {
    notification.style.opacity = '1';
    notification.style.transform = 'translateX(0)';
  }, 10);

  // 自动隐藏
  setTimeout(() => {
    notification.style.opacity = '0';
    notification.style.transform = 'translateX(100%)';
    setTimeout(() => {
      if (notification.parentNode) {
        notification.parentNode.removeChild(notification);
      }
    }, 300);
  }, duration);

  // 点击关闭
  notification.addEventListener('click', () => {
    notification.style.opacity = '0';
    notification.style.transform = 'translateX(100%)';
    setTimeout(() => {
      if (notification.parentNode) {
        notification.parentNode.removeChild(notification);
      }
    }, 300);
  });
}

// =================== 启动横幅 ===================
function printScreenshotBanner() {
  console.log('');
  console.log('███╗   ███╗ ██████╗ ███████╗██╗  ██╗███████╗███╗   ██╗ ██████╗ ');
  console.log('████╗ ████║██╔═══██╗██╔════╝██║  ██║██╔════╝████╗  ██║██╔════╝ ');
  console.log('██╔████╔██║██║   ██║███████╗███████║█████╗  ██╔██╗ ██║██║  ███╗');
  console.log('██║╚██╔╝██║██║   ██║╚════██║██╔══██║██╔══╝  ██║╚██╗██║██║   ██║');
  console.log('██║ ╚═╝ ██║╚██████╔╝███████║██║  ██║███████╗██║ ╚████║╚██████╔╝');
  console.log('╚═╝     ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═══╝ ╚═════╝ ');
  console.log('');
  console.log('Screenshot Window - 截屏窗口');
  console.log('Author: MoSheng | QuickClipboard v1.0.0');
  console.log('Screenshot window initializing...');
  console.log('');
}
document.addEventListener('contextmenu', function (e) {
  e.preventDefault();
});
// 截屏窗口实例
const screenshotWindow = new Window('screenshot');

// DOM 元素
let overlay, selectionArea, toolbar, sizeInfo, hintInfo, loadingOverlay;
let confirmButton, cancelButton, fullscreenButton;
let maskTop, maskBottom, maskLeft, maskRight;

// 选择状态
let isSelecting = false;
let isResizing = false;
let resizeDirection = '';
let startX = 0;
let startY = 0;
let currentX = 0;
let currentY = 0;
let selectionBounds = { left: 0, top: 0, width: 0, height: 0 };

// 初始化
document.addEventListener('DOMContentLoaded', async () => {
  // 输出启动横幅
  printScreenshotBanner();

  // 初始化主题管理器
  const { initThemeManager } = await import('./js/themeManager.js');
  initThemeManager();

  initializeElements();
  setupEventListeners();
  setupKeyboardShortcuts();

  // 全局禁用右键菜单
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault();
  });
});

// 初始化DOM元素
function initializeElements() {
  overlay = document.getElementById('screenshot-overlay');
  selectionArea = document.getElementById('selection-area');
  toolbar = document.getElementById('toolbar');
  sizeInfo = document.getElementById('size-info');
  hintInfo = document.getElementById('hint-info');
  loadingOverlay = document.getElementById('loading-overlay');

  // 获取四个遮罩层
  maskTop = document.getElementById('mask-top');
  maskBottom = document.getElementById('mask-bottom');
  maskLeft = document.getElementById('mask-left');
  maskRight = document.getElementById('mask-right');

  confirmButton = document.getElementById('confirm-button');
  cancelButton = document.getElementById('cancel-button');
  fullscreenButton = document.getElementById('fullscreen-button');

  // 初始化遮罩层，显示完整覆盖
  initializeMasks();

  // 初始化调整节点事件
  initializeResizeHandles();
}

// 初始化遮罩层，显示完整的半透明覆盖
function initializeMasks() {
  const windowWidth = window.innerWidth;
  const windowHeight = window.innerHeight;

  // 初始状态：只显示上遮罩覆盖整个屏幕
  maskTop.style.height = windowHeight + 'px';
  maskBottom.style.height = '0';
  maskLeft.style.width = '0';
  maskRight.style.width = '0';
}

// 初始化调整节点事件
function initializeResizeHandles() {
  const resizeHandles = document.querySelectorAll('.resize-handle');

  resizeHandles.forEach(handle => {
    handle.addEventListener('mousedown', startResize);
  });
}

// 开始调整大小
function startResize(e) {
  e.preventDefault();
  e.stopPropagation();

  isResizing = true;
  resizeDirection = e.target.dataset.direction;

  // 记录当前选区边界
  const rect = selectionArea.getBoundingClientRect();
  selectionBounds = {
    left: rect.left,
    top: rect.top,
    width: rect.width,
    height: rect.height
  };

  startX = e.clientX;
  startY = e.clientY;

  // 添加调整状态类
  selectionArea.classList.add('resizing');

  // 添加全局事件监听
  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', stopResize);
}

// 处理调整大小
function handleResize(e) {
  if (!isResizing) return;

  const deltaX = e.clientX - startX;
  const deltaY = e.clientY - startY;

  let newLeft = selectionBounds.left;
  let newTop = selectionBounds.top;
  let newWidth = selectionBounds.width;
  let newHeight = selectionBounds.height;

  // 根据调整方向计算新的边界
  switch (resizeDirection) {
    case 'nw': // 左上角
      newLeft += deltaX;
      newTop += deltaY;
      newWidth -= deltaX;
      newHeight -= deltaY;
      break;
    case 'ne': // 右上角
      newTop += deltaY;
      newWidth += deltaX;
      newHeight -= deltaY;
      break;
    case 'sw': // 左下角
      newLeft += deltaX;
      newWidth -= deltaX;
      newHeight += deltaY;
      break;
    case 'se': // 右下角
      newWidth += deltaX;
      newHeight += deltaY;
      break;
    case 'n': // 上边
      newTop += deltaY;
      newHeight -= deltaY;
      break;
    case 's': // 下边
      newHeight += deltaY;
      break;
    case 'w': // 左边
      newLeft += deltaX;
      newWidth -= deltaX;
      break;
    case 'e': // 右边
      newWidth += deltaX;
      break;
  }

  // 确保最小尺寸
  if (newWidth < 10) {
    if (resizeDirection.includes('w')) {
      newLeft = selectionBounds.left + selectionBounds.width - 10;
    }
    newWidth = 10;
  }
  if (newHeight < 10) {
    if (resizeDirection.includes('n')) {
      newTop = selectionBounds.top + selectionBounds.height - 10;
    }
    newHeight = 10;
  }

  // 更新选区位置和大小
  selectionArea.style.left = newLeft + 'px';
  selectionArea.style.top = newTop + 'px';
  selectionArea.style.width = newWidth + 'px';
  selectionArea.style.height = newHeight + 'px';

  // 更新遮罩层
  updateOverlayMasks(newLeft, newTop, newWidth, newHeight);

  // 更新尺寸信息
  updateSizeInfoForResize(newWidth, newHeight, newLeft, newTop);

  // 更新工具栏位置
  updateToolbarPositionForResize(newLeft, newTop, newWidth, newHeight);
}

// 停止调整大小
function stopResize() {
  if (!isResizing) return;

  isResizing = false;
  resizeDirection = '';

  // 移除调整状态类
  selectionArea.classList.remove('resizing');

  // 更新坐标变量以保持一致性
  const rect = selectionArea.getBoundingClientRect();
  startX = rect.left;
  startY = rect.top;
  currentX = rect.left + rect.width;
  currentY = rect.top + rect.height;

  // 移除全局事件监听
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
}

// 为调整大小更新尺寸信息
function updateSizeInfoForResize(width, height, left, top) {
  document.getElementById('size-text').textContent = `${Math.round(width)} × ${Math.round(height)}`;

  // 更新尺寸信息位置
  let sizeX = left;
  let sizeY = top - 30;

  // 如果上方空间不够，显示在选区内部左上角
  if (sizeY < 10) {
    sizeY = top + 10;
  }

  // 确保不超出屏幕右边界
  const sizeInfoWidth = 80; // 估算尺寸信息的宽度
  if (sizeX + sizeInfoWidth > window.innerWidth) {
    sizeX = window.innerWidth - sizeInfoWidth - 10;
  }

  sizeInfo.style.left = Math.max(10, sizeX) + 'px';
  sizeInfo.style.top = Math.max(10, sizeY) + 'px';
  sizeInfo.style.display = 'block';
}

// 为调整大小更新工具栏位置
function updateToolbarPositionForResize(left, top, width, height) {
  const toolbarWidth = 200;
  const toolbarHeight = 40;
  const margin = 10;

  let toolbarX, toolbarY;

  // 优先尝试在右侧显示
  if (left + width + margin + toolbarWidth <= window.innerWidth) {
    // 右侧有足够空间
    toolbarX = left + width + margin;
    toolbarY = top + height / 2 - toolbarHeight / 2;
  } else if (left - margin - toolbarWidth >= 0) {
    // 左侧有足够空间
    toolbarX = left - margin - toolbarWidth;
    toolbarY = top + height / 2 - toolbarHeight / 2;
  } else {
    // 左右都没有足够空间，显示在选区内部右下角
    toolbarX = left + width - toolbarWidth - margin;
    toolbarY = top + height - toolbarHeight - margin;
  }

  // 确保工具栏不超出屏幕边界
  toolbarX = Math.max(margin, Math.min(toolbarX, window.innerWidth - toolbarWidth - margin));
  toolbarY = Math.max(margin, Math.min(toolbarY, window.innerHeight - toolbarHeight - margin));

  toolbar.style.left = toolbarX + 'px';
  toolbar.style.top = toolbarY + 'px';
  toolbar.style.display = 'block';
}

// 设置事件监听器
function setupEventListeners() {
  // 鼠标事件
  overlay.addEventListener('mousedown', handleMouseDown);
  overlay.addEventListener('mousemove', handleMouseMove);
  overlay.addEventListener('mouseup', handleMouseUp);

  // 右键菜单事件 - 禁用默认菜单并取消截屏
  overlay.addEventListener('contextmenu', handleContextMenu);

  // 工具栏按钮事件 - 添加事件阻止冒泡
  confirmButton.addEventListener('click', (e) => {
    e.stopPropagation();
    e.preventDefault();
    confirmScreenshot();
  });
  cancelButton.addEventListener('click', (e) => {
    e.stopPropagation();
    e.preventDefault();
    cancelScreenshot();
  });
  fullscreenButton.addEventListener('click', (e) => {
    e.stopPropagation();
    e.preventDefault();
    fullscreenScreenshot();
  });

  // 防止工具栏区域的鼠标事件冒泡到overlay
  toolbar.addEventListener('mousedown', (e) => {
    e.stopPropagation();
  });
  toolbar.addEventListener('mousemove', (e) => {
    e.stopPropagation();
  });
  toolbar.addEventListener('mouseup', (e) => {
    e.stopPropagation();
  });
}

// 设置键盘快捷键
function setupKeyboardShortcuts() {
  document.addEventListener('keydown', (e) => {
    switch (e.key) {
      case 'Escape':
        e.preventDefault();
        cancelScreenshot();
        break;
      case 'Enter':
        e.preventDefault();
        if (e.ctrlKey && isSelectionValid()) {
          confirmScreenshot();
        }
        break;
      case 'F11':
        e.preventDefault();
        fullscreenScreenshot();
        break;
    }
  });
}

// 鼠标按下事件
function handleMouseDown(e) {
  if (e.button !== 0) return; // 只处理左键

  // 如果点击的是调整节点，不开始新的选择
  if (e.target.classList.contains('resize-handle')) {
    return;
  }

  // 如果正在调整大小，不开始新的选择
  if (isResizing) {
    return;
  }

  isSelecting = true;
  startX = e.clientX;
  startY = e.clientY;
  currentX = e.clientX;
  currentY = e.clientY;

  // 隐藏提示信息
  hintInfo.style.display = 'none';

  // 显示选择区域
  selectionArea.style.display = 'block';
  updateSelectionArea();

  e.preventDefault();
}

// 鼠标移动事件
function handleMouseMove(e) {
  if (!isSelecting) return;

  currentX = e.clientX;
  currentY = e.clientY;

  updateSelectionArea();
  updateSizeInfo();
  updateToolbarPosition();
}

// 鼠标释放事件
function handleMouseUp() {
  if (!isSelecting) return;

  isSelecting = false;

  if (isSelectionValid()) {
    // 显示工具栏
    toolbar.style.display = 'block';
    updateToolbarPosition();
  } else {
    // 选择区域太小，重置
    resetSelection();
  }
}

// 右键菜单事件处理
function handleContextMenu(e) {
  e.preventDefault(); // 禁用默认右键菜单
  e.stopPropagation();

  // 右键取消截屏
  cancelScreenshot();
}

// 更新选择区域
function updateSelectionArea() {
  const left = Math.min(startX, currentX);
  const top = Math.min(startY, currentY);
  const width = Math.abs(currentX - startX);
  const height = Math.abs(currentY - startY);

  selectionArea.style.left = left + 'px';
  selectionArea.style.top = top + 'px';
  selectionArea.style.width = width + 'px';
  selectionArea.style.height = height + 'px';

  // 更新四个遮罩层，创建"挖空"效果
  updateOverlayMasks(left, top, width, height);
}

// 更新四个遮罩层，创建"挖空"效果
function updateOverlayMasks(left, top, width, height) {
  const right = left + width;
  const bottom = top + height;
  const windowWidth = window.innerWidth;
  const windowHeight = window.innerHeight;

  // 上遮罩：从顶部到选区顶部
  maskTop.style.height = top + 'px';

  // 下遮罩：从选区底部到底部
  maskBottom.style.height = (windowHeight - bottom) + 'px';

  // 左遮罩：选区高度范围内，从左边到选区左边
  maskLeft.style.top = top + 'px';
  maskLeft.style.height = height + 'px';
  maskLeft.style.width = left + 'px';

  // 右遮罩：选区高度范围内，从选区右边到右边
  maskRight.style.top = top + 'px';
  maskRight.style.height = height + 'px';
  maskRight.style.width = (windowWidth - right) + 'px';
}

// 更新尺寸信息
function updateSizeInfo() {
  const left = Math.min(startX, currentX);
  const top = Math.min(startY, currentY);
  const width = Math.abs(currentX - startX);
  const height = Math.abs(currentY - startY);

  document.getElementById('size-text').textContent = `${width} × ${height}`;

  if (width > 0 && height > 0) {
    sizeInfo.style.display = 'block';

    // 尺寸信息显示在选区左上角的上方
    let sizeX = left;
    let sizeY = top - 30;

    // 如果上方空间不够，显示在选区内部左上角
    if (sizeY < 10) {
      sizeY = top + 10;
    }

    // 确保不超出屏幕右边界
    const sizeInfoWidth = 80; // 估算尺寸信息的宽度
    if (sizeX + sizeInfoWidth > window.innerWidth) {
      sizeX = window.innerWidth - sizeInfoWidth - 10;
    }

    sizeInfo.style.left = Math.max(10, sizeX) + 'px';
    sizeInfo.style.top = Math.max(10, sizeY) + 'px';
  } else {
    sizeInfo.style.display = 'none';
  }
}

// 更新工具栏位置
function updateToolbarPosition() {
  const left = Math.min(startX, currentX);
  const top = Math.min(startY, currentY);
  const width = Math.abs(currentX - startX);
  const height = Math.abs(currentY - startY);

  const toolbarWidth = 200;
  const toolbarHeight = 40;
  const margin = 10;

  let toolbarX, toolbarY;

  // 优先尝试在右侧显示
  if (left + width + margin + toolbarWidth <= window.innerWidth) {
    // 右侧有足够空间
    toolbarX = left + width + margin;
    toolbarY = top + height / 2 - toolbarHeight / 2;
  } else if (left - margin - toolbarWidth >= 0) {
    // 左侧有足够空间
    toolbarX = left - margin - toolbarWidth;
    toolbarY = top + height / 2 - toolbarHeight / 2;
  } else {
    // 左右都没有足够空间，显示在选区内部右下角
    toolbarX = left + width - toolbarWidth - margin;
    toolbarY = top + height - toolbarHeight - margin;
  }

  // 确保工具栏不超出屏幕边界
  toolbarX = Math.max(margin, Math.min(toolbarX, window.innerWidth - toolbarWidth - margin));
  toolbarY = Math.max(margin, Math.min(toolbarY, window.innerHeight - toolbarHeight - margin));

  toolbar.style.left = toolbarX + 'px';
  toolbar.style.top = toolbarY + 'px';
}

// 检查选择是否有效
function isSelectionValid() {
  // 如果选区已显示，直接从DOM获取实际尺寸
  if (selectionArea.style.display === 'block') {
    const rect = selectionArea.getBoundingClientRect();
    return rect.width >= 10 && rect.height >= 10;
  }

  // 否则使用原始坐标计算
  const width = Math.abs(currentX - startX);
  const height = Math.abs(currentY - startY);
  return width >= 10 && height >= 10;
}

// 重置选择
function resetSelection() {
  selectionArea.style.display = 'none';
  toolbar.style.display = 'none';
  sizeInfo.style.display = 'none';
  hintInfo.style.display = 'block';
  isSelecting = false;

  // 隐藏所有遮罩层
  maskTop.style.height = '0';
  maskBottom.style.height = '0';
  maskLeft.style.width = '0';
  maskRight.style.width = '0';
}

// 确认截屏
async function confirmScreenshot() {
  if (!isSelectionValid()) return;

  showLoading(true);

  try {
    // 获取当前选区的实际坐标（支持调整节点后的坐标）
    let left, top, width, height;

    if (selectionArea.style.display === 'block') {
      // 从DOM获取实际的选区坐标
      const rect = selectionArea.getBoundingClientRect();
      left = rect.left;
      top = rect.top;
      width = rect.width;
      height = rect.height;
    } else {
      // 使用原始坐标计算
      left = Math.min(startX, currentX);
      top = Math.min(startY, currentY);
      width = Math.abs(currentX - startX);
      height = Math.abs(currentY - startY);
    }

    // 获取设备像素比和缩放信息
    const devicePixelRatio = window.devicePixelRatio || 1;
    const screenWidth = window.screen.width;
    const screenHeight = window.screen.height;
    const availWidth = window.screen.availWidth;
    const availHeight = window.screen.availHeight;

    // 计算可能的DPI缩放坐标
    const scaledX = Math.round(left * devicePixelRatio);
    const scaledY = Math.round(top * devicePixelRatio);
    const scaledWidth = Math.round(width * devicePixelRatio);
    const scaledHeight = Math.round(height * devicePixelRatio);

    // 发送调试信息到后端日志
    await invoke('log_debug', {
      message: `前端坐标计算: 选择区域 left=${left}, top=${top}, width=${width}, height=${height}`
    });
    await invoke('log_debug', {
      message: `设备信息: devicePixelRatio=${devicePixelRatio}, screen=${screenWidth}x${screenHeight}, avail=${availWidth}x${availHeight}`
    });
    await invoke('log_debug', {
      message: `原始坐标: x=${Math.round(left)}, y=${Math.round(top)}, width=${Math.round(width)}, height=${Math.round(height)}`
    });
    await invoke('log_debug', {
      message: `缩放坐标: x=${scaledX}, y=${scaledY}, width=${scaledWidth}, height=${scaledHeight}`
    });

    // 使用DPI缩放后的坐标，因为后端需要物理像素坐标
    const finalX = scaledX;
    const finalY = scaledY;
    const finalWidth = scaledWidth;
    const finalHeight = scaledHeight;

    // 隐藏选框和工具栏，避免被截入
    selectionArea.style.display = 'none';
    toolbar.style.display = 'none';
    sizeInfo.style.display = 'none';
    hintInfo.style.display = 'none';

    // 等待一小段时间确保UI更新
    await new Promise(resolve => setTimeout(resolve, 50));

    // 调用后端截屏命令（后端会自动关闭窗口）
    await invoke('take_screenshot', {
      x: finalX,
      y: finalY,
      width: finalWidth,
      height: finalHeight
    });

  } catch (error) {
    console.error('截屏失败:', error);
    showLoading(false);

    // 显示错误提示并恢复UI
    showScreenshotNotification(`截屏失败: ${error}`, 'error', 4000);

    // 恢复选框和工具栏显示
    selectionArea.style.display = 'block';
    toolbar.style.display = 'block';
    sizeInfo.style.display = 'block';
    hintInfo.style.display = 'block';
  }
}

// 取消截屏
async function cancelScreenshot() {
  await closeScreenshotWindow();
}

// 全屏截屏
async function fullscreenScreenshot() {
  showLoading(true);

  try {
    // 调用后端全屏截屏命令（后端会自动关闭窗口）
    await invoke('take_fullscreen_screenshot');

  } catch (error) {
    console.error('全屏截屏失败:', error);
    showLoading(false);
  }
}

// 显示/隐藏加载状态
function showLoading(show) {
  loadingOverlay.style.display = show ? 'flex' : 'none';
}

// 关闭截屏窗口
async function closeScreenshotWindow() {
  try {
    await invoke('close_screenshot_window');
  } catch (error) {
    console.error('关闭截屏窗口失败:', error);
  }
}
