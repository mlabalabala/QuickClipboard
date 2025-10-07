/**
 * 背景图管理器
 */
import { convertFileSrc } from '@tauri-apps/api/core';
import { getDominantColor, generateTitleBarColors, applyTitleBarColors, removeTitleBarColors } from './colorAnalyzer.js';

/**
 * 应用背景图到指定容器
 */
export async function applyBackgroundImage(options) {
  const {
    containerSelector,
    theme,
    backgroundImagePath,
    windowName = '窗口'
  } = options;

  try {
    const container = document.querySelector(containerSelector);
    if (!container) {
      console.warn(`未找到容器: ${containerSelector}`);
      return;
    }

    // 只有在背景图主题时才应用背景图
    if (theme === 'background' && backgroundImagePath) {
      // 使用 convertFileSrc 直接转换文件路径
      const assetUrl = convertFileSrc(backgroundImagePath, 'asset');
      container.style.backgroundImage = `url("${assetUrl}")`;

      // 分析背景图主色调并应用到标题栏
      try {
        const dominantColor = await getDominantColor(assetUrl);
        const titleBarColors = generateTitleBarColors(dominantColor);
        applyTitleBarColors(titleBarColors);
        console.log(`${windowName}背景图已应用，主色调分析完成`);
      } catch (colorError) {
        console.warn(`${windowName}分析背景图颜色失败:`, colorError);
        removeTitleBarColors();
      }
    } else {
      // 清除背景图
      container.style.backgroundImage = '';
      removeTitleBarColors();
    }
  } catch (error) {
    console.error(`${windowName}应用背景图失败:`, error);
  }
}

/**
 * 清除背景图
 */
export function clearBackgroundImage(containerSelector) {
  try {
    const container = document.querySelector(containerSelector);
    if (container) {
      container.style.backgroundImage = '';
    }
    removeTitleBarColors();
  } catch (error) {
    console.error('清除背景图失败:', error);
  }
}
