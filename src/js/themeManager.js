/* =================== 主题管理器 =================== */

// 主题类型枚举
export const THEMES = {
  AUTO: 'auto',      // 跟随系统
  LIGHT: 'light',    // 亮色主题
  DARK: 'dark',      // 暗色主题
  TRANSPARENT: 'transparent' // 透明主题
};

// 当前主题状态
let currentTheme = THEMES.AUTO;
let systemPrefersDark = false;

// 主题变更监听器
const themeChangeListeners = new Set();

/**
 * 初始化主题管理器
 */
export async function initThemeManager() {
  // 检测系统主题偏好
  detectSystemTheme();

  // 监听系统主题变化
  if (window.matchMedia) {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', handleSystemThemeChange);
  }

  // 从设置中加载主题
  await loadThemeFromSettings();

  console.log('主题管理器初始化完成');
}

/**
 * 检测系统主题偏好
 */
function detectSystemTheme() {
  if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    systemPrefersDark = true;
  } else {
    systemPrefersDark = false;
  }
}

/**
 * 处理系统主题变化
 */
function handleSystemThemeChange(e) {
  systemPrefersDark = e.matches;

  // 如果当前是自动模式，重新应用主题
  if (currentTheme === THEMES.AUTO) {
    applyTheme(THEMES.AUTO);
  }

  // 通知监听器
  notifyThemeChange();
}

/**
 * 从设置中加载主题
 */
async function loadThemeFromSettings() {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const settings = await invoke('get_settings');

    if (settings && settings.theme) {
      setTheme(settings.theme);
    } else {
      setTheme(THEMES.AUTO);
    }
  } catch (error) {
    console.warn('加载主题设置失败，使用默认主题:', error);
    setTheme(THEMES.AUTO);
  }
}

/**
 * 保存主题到设置
 */
async function saveThemeToSettings(theme) {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('update_theme_setting', { theme });
  } catch (error) {
    console.error('保存主题设置失败:', error);
  }
}

/**
 * 设置主题
 * @param {string} theme - 主题名称
 */
export function setTheme(theme) {
  if (!Object.values(THEMES).includes(theme)) {
    console.warn('无效的主题:', theme);
    return;
  }

  currentTheme = theme;
  applyTheme(theme);
  saveThemeToSettings(theme);
  notifyThemeChange();
}

/**
 * 获取当前主题
 */
export function getCurrentTheme() {
  return currentTheme;
}

/**
 * 获取实际应用的主题（解析auto主题）
 */
export function getEffectiveTheme() {
  if (currentTheme === THEMES.AUTO) {
    return systemPrefersDark ? THEMES.DARK : THEMES.LIGHT;
  }
  return currentTheme;
}

/**
 * 应用主题到DOM
 * @param {string} theme - 主题名称
 */
function applyTheme(theme) {
  const body = document.body;

  // 移除所有主题类
  body.classList.remove('theme-light', 'theme-dark', 'theme-transparent');

  // 应用新主题
  switch (theme) {
    case THEMES.LIGHT:
      body.classList.add('theme-light');
      break;
    case THEMES.DARK:
      body.classList.add('theme-dark');
      break;
    case THEMES.TRANSPARENT:
      body.classList.add('theme-transparent');
      break;
    case THEMES.AUTO:
      // 根据系统偏好应用主题
      if (systemPrefersDark) {
        body.classList.add('theme-dark');
      } else {
        body.classList.add('theme-light');
      }
      break;
  }

  console.log(`主题已应用: ${theme} (实际: ${getEffectiveTheme()})`);
}

/**
 * 添加主题变更监听器
 * @param {Function} listener - 监听器函数
 */
export function addThemeChangeListener(listener) {
  themeChangeListeners.add(listener);
}

/**
 * 移除主题变更监听器
 * @param {Function} listener - 监听器函数
 */
export function removeThemeChangeListener(listener) {
  themeChangeListeners.delete(listener);
}

/**
 * 通知主题变更
 */
function notifyThemeChange() {
  const effectiveTheme = getEffectiveTheme();
  themeChangeListeners.forEach(listener => {
    try {
      listener(currentTheme, effectiveTheme);
    } catch (error) {
      console.error('主题变更监听器执行失败:', error);
    }
  });
}

/**
 * 切换到下一个主题
 */
export function toggleTheme() {
  const themes = Object.values(THEMES);
  const currentIndex = themes.indexOf(currentTheme);
  const nextIndex = (currentIndex + 1) % themes.length;
  setTheme(themes[nextIndex]);
}

/**
 * 获取主题显示名称
 * @param {string} theme - 主题名称
 */
export function getThemeDisplayName(theme) {
  switch (theme) {
    case THEMES.AUTO:
      return '跟随系统';
    case THEMES.LIGHT:
      return '亮色主题';
    case THEMES.DARK:
      return '暗色主题';
    case THEMES.TRANSPARENT:
      return '透明主题';
    default:
      return '未知主题';
  }
}

/**
 * 获取所有可用主题
 */
export function getAvailableThemes() {
  return Object.values(THEMES).map(theme => ({
    value: theme,
    label: getThemeDisplayName(theme)
  }));
}

/**
 * 检查是否为暗色主题
 */
export function isDarkTheme() {
  const effectiveTheme = getEffectiveTheme();
  return effectiveTheme === THEMES.DARK || effectiveTheme === THEMES.TRANSPARENT;
}

/**
 * 检查是否为透明主题
 */
export function isTransparentTheme() {
  return getEffectiveTheme() === THEMES.TRANSPARENT;
}

/**
 * 为其他窗口应用主题
 * @param {Document} targetDocument - 目标文档对象
 */
export function applyThemeToWindow(targetDocument) {
  if (!targetDocument || !targetDocument.body) {
    return;
  }

  const body = targetDocument.body;

  // 移除所有主题类
  body.classList.remove('theme-light', 'theme-dark', 'theme-transparent');

  // 应用当前主题
  const effectiveTheme = getEffectiveTheme();
  switch (effectiveTheme) {
    case THEMES.LIGHT:
      body.classList.add('theme-light');
      break;
    case THEMES.DARK:
      body.classList.add('theme-dark');
      break;
    case THEMES.TRANSPARENT:
      body.classList.add('theme-transparent');
      break;
  }
}

// 自动初始化（如果在浏览器环境中）
if (typeof window !== 'undefined' && typeof document !== 'undefined') {
  // 等待DOM加载完成后初始化
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initThemeManager);
  } else {
    initThemeManager();
  }
}
