import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentTheme } from './themeManager.js';

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
  // 音效设置
  soundEnabled: true,
  soundVolume: 50,
  copySoundPath: '',
  pasteSoundPath: '',
  soundPreset: 'default'
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
    // 历史记录数量在后端已经处理
    // console.log('历史记录数量已更新:', settings.historyLimit);
  }

  if (settings.clipboardMonitor !== undefined) {
    // 剪贴板监听设置
    // console.log('剪贴板监听设置:', settings.clipboardMonitor);
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

// 获取当前设置
export function getCurrentSettings() {
  return { ...currentSettings };
}

// 更新单个设置项
export function updateSetting(key, value) {
  currentSettings[key] = value;
  applySettings({ [key]: value });
}

// 初始化主题（用于应用启动时）
export function initializeTheme() {
  // 使用主题管理器获取当前主题，不再从localStorage读取
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
