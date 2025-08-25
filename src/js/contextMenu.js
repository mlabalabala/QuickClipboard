// 通用右键菜单模块
import { openUrl } from '@tauri-apps/plugin-opener';
import { showNotification } from './notificationManager.js';
import { getContentType } from './clipboard.js';

// 打开链接
async function openLink(url) {
  try {
    // 如果URL不包含协议，添加https://
    if (!url.match(/^https?:\/\//i)) {
      url = 'https://' + url;
    }

    await openUrl(url);
    showNotification('已在浏览器中打开链接', 'success', 2000);
  } catch (error) {
    console.error('打开链接失败:', error);
    showNotification('打开链接失败', 'error');
  }
}

// 创建菜单项
function createMenuItem(iconClass, text, onClick) {
  const item = document.createElement('div');
  item.className = 'context-menu-item';
  item.style.cssText = `
    display: flex;
    align-items: center;
    padding: 8px 12px;
    cursor: pointer;
    font-size: 13px;
    color: #333;
    transition: background-color 0.15s ease;
    border-bottom: 1px solid #f5f5f5;
  `;

  item.innerHTML = `
    <i class="ti ${iconClass}" style="margin-right: 8px; font-size: 14px;"></i>
    <span>${text}</span>
  `;

  item.addEventListener('mouseenter', () => {
    item.style.backgroundColor = '#f5f5f5';
  });

  item.addEventListener('mouseleave', () => {
    item.style.backgroundColor = 'transparent';
  });

  item.addEventListener('click', onClick);
  return item;
}

// 创建分隔线
function createSeparator() {
  const separator = document.createElement('div');
  separator.className = 'context-menu-separator';
  separator.style.cssText = `
    height: 1px;
    background: #e8e8e8;
    margin: 4px 0;
  `;
  return separator;
}

// 隐藏当前显示的右键菜单
export function hideContextMenu() {
  const existingMenu = document.querySelector('.context-menu');
  if (existingMenu) {
    existingMenu.remove();
  }
}

// 显示通用右键菜单
export function showContextMenu(event, options) {
  // 移除已存在的菜单
  hideContextMenu();

  // 创建菜单容器
  const menu = document.createElement('div');
  menu.className = 'context-menu';
  menu.style.cssText = `
    position: fixed;
    top: ${event.clientY}px;
    left: ${event.clientX}px;
    background: white;
    border: 1px solid #ddd;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 10000;
    min-width: 150px;
    padding: 4px 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  `;

  const menuItems = [];

  // 根据内容类型添加打开链接选项
  if (options.content && getContentType(options.content) === 'link') {
    const openItem = createMenuItem('ti-external-link', '在浏览器中打开', async () => {
      await openLink(options.content);
      menu.remove();
    });
    menuItems.push(openItem);
  }

  // 添加自定义菜单项
  if (options.items && options.items.length > 0) {
    // 如果已经有链接菜单项，添加分隔线
    if (menuItems.length > 0) {
      menuItems.push(createSeparator());
    }

    options.items.forEach(item => {
      if (item.type === 'separator') {
        menuItems.push(createSeparator());
      } else {
        const menuItem = createMenuItem(item.icon, item.text, () => {
          item.onClick();
          menu.remove();
        });

        // 应用自定义样式
        if (item.style) {
          Object.assign(menuItem.style, item.style);
        }

        menuItems.push(menuItem);
      }
    });
  }

  // 添加所有菜单项到容器
  menuItems.forEach(menuItem => {
    menu.appendChild(menuItem);
  });

  document.body.appendChild(menu);

  // 点击其他地方关闭菜单
  const closeMenu = (e) => {
    if (!menu.contains(e.target)) {
      menu.remove();
      document.removeEventListener('click', closeMenu);
    }
  };

  setTimeout(() => {
    document.addEventListener('click', closeMenu);
  }, 0);

  // 调整菜单位置，确保不超出屏幕
  const rect = menu.getBoundingClientRect();
  if (rect.right > window.innerWidth) {
    menu.style.left = (event.clientX - rect.width) + 'px';
  }
  if (rect.bottom > window.innerHeight) {
    menu.style.top = (event.clientY - rect.height) + 'px';
  }
}
