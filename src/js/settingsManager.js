import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentTheme } from './themeManager.js';
import { refreshClipboardHistory } from './clipboard.js';

// 当前设置
let currentSettings = {
  autoStart: false,
  startHidden: false,
  runAsAdmin: false,
  showStartupNotification: true,
  historyLimit: 100,
  theme: 'light',
  opacity: 0.9,
  toggleShortcut: '',
  numberShortcuts: true,
  clipboardMonitor: true,
  ignoreDuplicates: true,
  saveImages: true,
  showImagePreview: false,
  // 音效设置
  soundEnabled: true,
  soundVolume: 50,
  copySoundPath: '',
  pasteSoundPath: '',
  soundPreset: 'default',
  // 动画设置
  clipboardAnimationEnabled: true
  ,
  // 显示行为
  autoScrollToTopOnShow: false
};

// 初始化设置管理器
export async function initializeSettingsManager() {
  // 加载当前设置
  await loadSettings();

  // 应用设置
  applySettings(currentSettings);

  // 监听设置变更事件
  await listen('settings-changed', (event) => {
    const newSettings = event.payload;
    currentSettings = { ...currentSettings, ...newSettings };
    applySettings(newSettings);
  });
}

// 加载设置
async function loadSettings() {
  try {
    const savedSettings = await invoke('get_settings');
    currentSettings = savedSettings;
    // console.log('主窗口设置加载成功:', currentSettings);
  } catch (error) {
    console.error('加载设置失败:', error);
    // 保持默认设置
  }
}

// 应用设置
function applySettings(settings) {
  // 应用主题
  if (settings.theme) {
    applyTheme(settings.theme);
  }

  // 应用透明度
  if (settings.opacity !== undefined) {
    applyOpacity(settings.opacity);
  }

  // 应用其他设置
  if (settings.historyLimit) {
    // 刷新剪贴板历史以应用新的数量限制
    refreshClipboardHistory();
  }

  if (settings.clipboardMonitor !== undefined) {
    // 剪贴板监听设置
    // console.log('剪贴板监听设置:', settings.clipboardMonitor);
  }

  // 应用动画设置
  if (settings.clipboardAnimationEnabled !== undefined) {
    applyAnimationSettings(settings.clipboardAnimationEnabled);
  }

  // 设置显示后滚动行为
  if (settings.autoScrollToTopOnShow !== undefined) {
    console.log('应用自动滚动设置:', settings.autoScrollToTopOnShow);
    setupAutoScrollOnShow(Boolean(settings.autoScrollToTopOnShow));
  }
}

// 应用主题
function applyTheme(theme) {
  const body = document.body;

  // 移除所有主题类
  body.classList.remove('theme-light', 'theme-dark', 'theme-transparent');

  // 应用新主题
  body.classList.add(`theme-${theme}`);

  // console.log('主题已应用:', theme);
}

// 应用透明度
function applyOpacity(opacity) {
  // 透明度主要影响透明主题
  if (currentSettings.theme === 'transparent') {
    document.documentElement.style.setProperty('--window-opacity', opacity);
  }

  // console.log('透明度已应用:', opacity);
}

// 应用动画设置
function applyAnimationSettings(animationEnabled) {
  // 获取或创建动画样式元素
  let animationStyleElement = document.getElementById('animation-control-styles');
  if (!animationStyleElement) {
    animationStyleElement = document.createElement('style');
    animationStyleElement.id = 'animation-control-styles';
    document.head.appendChild(animationStyleElement);
  }

  if (animationEnabled) {
    // 启用动画：使用正常的动画持续时间
    animationStyleElement.textContent = `
      .window-show-animation {
        animation: scrollDown 0.3s cubic-bezier(0.23, 1, 0.32, 1) forwards;
      }
      .window-hide-animation {
        animation: scrollUp 0.2s cubic-bezier(0.755, 0.05, 0.855, 0.06) forwards;
      }
    `;
  } else {
    // 禁用动画：将动画持续时间设置为 0
    animationStyleElement.textContent = `
      .window-show-animation {
        animation: scrollDown 0s forwards;
      }
      .window-hide-animation {
        animation: scrollUp 0s forwards;
      }
    `;
  }

  console.log('动画设置已应用:', animationEnabled);
}

// 获取当前设置
export function getCurrentSettings() {
  return { ...currentSettings };
}

// 更新单个设置项
export function updateSetting(key, value) {
  currentSettings[key] = value;
  applySettings({ [key]: value });
}

// 设置窗口显示后自动滚动到顶部
function setupAutoScrollOnShow(enabled) {
  console.log('设置自动滚动监听:', enabled);
  
  // 如已存在监听，则先解除
  if (window.__autoScrollUnlisten && typeof window.__autoScrollUnlisten === 'function') {
    try { 
      console.log('移除现有自动滚动监听');
      window.__autoScrollUnlisten(); 
    } catch (_) { }
    window.__autoScrollUnlisten = null;
  }

  if (enabled) {
    console.log('创建新的自动滚动监听');
    // 监听来自后端的显示事件
    import('@tauri-apps/api/event')
      .then(({ listen }) => listen('window-show-animation', () => autoScrollHandler()))
      .then((unlisten) => { 
        console.log('自动滚动监听创建成功');
        window.__autoScrollUnlisten = unlisten; 
      })
      .catch((error) => { 
        console.error('设置自动滚动监听失败:', error);
      });
  } else {
    console.log('自动滚动已禁用');
  }
}

function autoScrollHandler() {
  console.log('自动滚动处理器被调用，当前设置:', currentSettings.autoScrollToTopOnShow);
  
  if (!currentSettings.autoScrollToTopOnShow) {
    console.log('自动滚动已禁用，不执行滚动');
    return; 
  }
  
  console.log('执行自动滚动到顶部');
  // 推迟到渲染完成后执行
  setTimeout(() => {
    try {
      const list = document.getElementById('clipboard-list');
      if (list) {
        list.scrollTo({ top: 0, behavior: 'instant' });
        console.log('滚动到顶部完成');
      } else {
        console.log('未找到clipboard-list元素');
      }
    } catch (error) { 
      console.error('滚动到顶部失败:', error);
    }
  }, 0);
}

// 初始化主题（用于应用启动时）
export function initializeTheme() {
  const currentTheme = getCurrentTheme();

  if (currentTheme) {
    // 更新设置对象以保持同步
    currentSettings.theme = currentTheme;
    console.log('主题已同步:', currentTheme);
  } else {
    // 如果主题管理器还没有初始化，使用设置中的主题
    const fallbackTheme = currentSettings.theme || 'light';
    applyTheme(fallbackTheme);
    currentSettings.theme = fallbackTheme;
    console.log('使用备用主题:', fallbackTheme);
  }
}

// 监听系统主题变化
export function setupThemeListener() {
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
    if (currentSettings.theme === 'system') {
      const prefersDark = e.matches;
      applyTheme(prefersDark ? 'dark' : 'light');
    }
  });
}

// 更新快捷键显示
export function updateShortcutDisplay() {
  const toggleShortcutElement = document.getElementById('toggle-shortcut-display');
  if (toggleShortcutElement && currentSettings.toggleShortcut) {
    toggleShortcutElement.textContent = `${currentSettings.toggleShortcut}: 显示/隐藏`;
  }
}
