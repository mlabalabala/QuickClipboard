import {
  currentTab,
  setCurrentTab,
  alertModal,
  alertTitle,
  alertMessage,
  confirmModal,
  confirmTitle,
  confirmMessage,
  confirmCallback,
  setConfirmCallback
} from './config.js';

// 显示通知消息
export function showNotification(message, type = 'info', duration = 3000) {
  // 移除已存在的通知，避免堆叠
  const existingNotifications = document.querySelectorAll('.notification');
  existingNotifications.forEach(n => {
    if (n.parentNode) {
      n.parentNode.removeChild(n);
    }
  });

  // 创建通知元素
  const notification = document.createElement('div');
  notification.className = `notification ${type}`;

  // 创建图标
  const icon = document.createElement('i');
  if (type === 'success') {
    icon.className = 'ti ti-check';
  } else if (type === 'error') {
    icon.className = 'ti ti-alert-circle';
  } else if (type === 'warning') {
    icon.className = 'ti ti-alert-triangle';
  } else {
    icon.className = 'ti ti-info-circle';
  }

  // 创建消息文本
  const messageSpan = document.createElement('span');
  messageSpan.textContent = message;

  // 组装通知内容
  notification.appendChild(icon);
  notification.appendChild(messageSpan);

  // 添加样式
  notification.style.cssText = `
    position: fixed;
    bottom: 20px;
    right: 20px;
    padding: 12px 16px;
    border-radius: 8px;
    color: white;
    font-size: 14px;
    font-weight: 500;
    z-index: 999999999;
    opacity: 0;
    transform: translateX(100%);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    display: flex;
    align-items: center;
    gap: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    backdrop-filter: blur(10px);
    max-width: 300px;
    word-wrap: break-word;
  `;

  // 根据类型设置背景色和边框
  if (type === 'success') {
    notification.style.background = 'linear-gradient(135deg, #28a745, #20c997)';
    notification.style.border = '1px solid rgba(40, 167, 69, 0.3)';
  } else if (type === 'error') {
    notification.style.background = 'linear-gradient(135deg, #dc3545, #e74c3c)';
    notification.style.border = '1px solid rgba(220, 53, 69, 0.3)';
  } else if (type === 'warning') {
    notification.style.background = 'linear-gradient(135deg, #ffc107, #ffb300)';
    notification.style.border = '1px solid rgba(255, 193, 7, 0.3)';
    notification.style.color = '#212529';
  } else {
    notification.style.background = 'linear-gradient(135deg, #4a89dc, #007bff)';
    notification.style.border = '1px solid rgba(74, 137, 220, 0.3)';
  }

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

// 设置标签页切换
export function setupTabSwitching() {
  const tabButtons = document.querySelectorAll('.tab-button');
  const tabContents = document.querySelectorAll('.tab-content');

  // 创建并缓存滑动指示器
  let tabIndicatorResizeTimer = null;
  function ensureTabSwitchIndicator() {
    const group = document.querySelector('.tab-switch-group');
    if (!group) return null;
    let indicator = group.querySelector('.tab-switch-indicator');
    if (!indicator) {
      indicator = document.createElement('div');
      indicator.className = 'tab-switch-indicator';
      group.appendChild(indicator);
    }
    return indicator;
  }

  function moveTabSwitchIndicatorToActive() {
    const activeButton = document.querySelector('.tab-button.active');
    const group = document.querySelector('.tab-switch-group');
    const indicator = ensureTabSwitchIndicator();
    if (!activeButton || !group || !indicator) return;
    const left = activeButton.offsetLeft;
    const width = activeButton.offsetWidth;
    indicator.style.left = left + 'px';
    indicator.style.width = width + 'px';
    indicator.style.opacity = '1';
  }

  tabButtons.forEach(button => {
    button.addEventListener('click', () => {
      const tabName = button.dataset.tab;

      // 更新按钮状态
      tabButtons.forEach(btn => btn.classList.remove('active'));
      button.classList.add('active');

      // 更新内容显示
      tabContents.forEach(content => content.classList.remove('active'));
      document.getElementById(`${tabName}-tab`).classList.add('active');

      setCurrentTab(tabName);

      // 发送标签页切换事件给虚拟列表
      window.dispatchEvent(new CustomEvent('tab-switched', {
        detail: { tabName: tabName }
      }));

      import('./navigation.js').then(module => {
        module.onTabSwitch();
      });

      notifyPreviewWindowTabChange(tabName);

      // 移动指示器到当前激活按钮
      moveTabSwitchIndicatorToActive();
    });
  });

  // 初始位置
  requestAnimationFrame(moveTabSwitchIndicatorToActive);

  // 窗口尺寸变化时重算位置（防抖）
  window.addEventListener('resize', () => {
    clearTimeout(tabIndicatorResizeTimer);
    tabIndicatorResizeTimer = setTimeout(moveTabSwitchIndicatorToActive, 120);
  });

  // 监听标题栏位置变化事件，更新指示器
  window.addEventListener('update-tab-indicator', moveTabSwitchIndicatorToActive);
}

// 通知预览窗口标签切换
async function notifyPreviewWindowTabChange(tabName) {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const groupId = await getCurrentGroupId(); // 获取当前选中的分组ID

    await invoke('notify_preview_tab_change', {
      tab: tabName,
      groupName: groupId
    });
  } catch (error) {
    // 预览窗口可能未打开，忽略错误
  }
}

// 获取当前选中的分组ID
async function getCurrentGroupId() {
  // 如果在常用文本标签页，获取当前选中的分组
  if (currentTab === 'quick-texts') {
    const { getCurrentGroupId } = await import('./groups.js');
    return getCurrentGroupId();
  }
  return 'clipboard'; // 剪贴板历史
}

// 显示确认对话框
export function showConfirmModal(title, message, callback) {
  confirmTitle.textContent = title;
  confirmMessage.textContent = message;
  setConfirmCallback(callback);
  confirmModal.classList.add('active');
}

// 隐藏确认对话框
export function hideConfirmModal() {
  confirmModal.classList.remove('active');
  setConfirmCallback(null);
}

// 显示提示框
export function showAlertModal(title, message) {
  alertTitle.textContent = title;
  alertMessage.textContent = message;
  alertModal.classList.add('active');
}

// 隐藏提示框
export function hideAlertModal() {
  alertModal.classList.remove('active');
}

// 设置确认对话框事件监听器
export function setupConfirmModal() {
  document.getElementById('confirm-cancel-btn').addEventListener('click', hideConfirmModal);
  const confirmOkBtn = document.getElementById('confirm-ok-btn');
  confirmOkBtn.addEventListener('click', () => {
    if (confirmCallback) {
      confirmCallback();
    }
    hideConfirmModal();
  });

  // 点击遮罩关闭确认对话框
  confirmModal.addEventListener('click', (e) => {
    if (e.target === confirmModal) {
      hideConfirmModal();
    }
  });
}

// 设置提示框事件监听器
export function setupAlertModal() {
  document.getElementById('alert-ok-btn').addEventListener('click', hideAlertModal);

  // 点击遮罩关闭提示框
  alertModal.addEventListener('click', (e) => {
    if (e.target === alertModal) {
      hideAlertModal();
    }
  });
}
