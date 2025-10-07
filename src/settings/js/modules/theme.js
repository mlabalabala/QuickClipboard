/**
 * 主题设置模块
 */
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { getDominantColor, generateTitleBarColors, applyTitleBarColors, removeTitleBarColors } from '../../../js/colorAnalyzer.js';
import { setTheme } from '../../../js/themeManager.js';
import { showNotification } from '../../../js/notificationManager.js';

export class ThemeManager {
    constructor(settings, saveCallback) {
        this.settings = settings;
        this.saveSettings = saveCallback;
    }

    /**
     * 绑定主题设置事件
     */
    bindEvents() {
        this.bindThemeSelection();
        this.bindOpacitySlider();
        this.bindBackgroundImage();
    }

    /**
     * 绑定主题选择
     */
    bindThemeSelection() {
        document.querySelectorAll('.theme-option').forEach(option => {
            option.addEventListener('click', () => {
                const theme = option.dataset.theme;
                this.setActiveTheme(theme);
                this.settings.theme = theme;
                
                // 切换背景图设置显隐
                const bgSetting = document.getElementById('background-image-setting');
                if (bgSetting) {
                    bgSetting.style.display = theme === 'background' ? '' : 'none';
                }
                
                this.applyBackgroundToSettingsContainer();
                this.saveSettings();
            });
        });
    }

    /**
     * 绑定透明度滑块
     */
    bindOpacitySlider() {
        const opacitySlider = document.getElementById('opacity-slider');
        if (opacitySlider) {
            opacitySlider.addEventListener('input', (e) => {
                const opacity = parseFloat(e.target.value);
                this.settings.opacity = opacity;
                this.updateOpacityDisplay(opacity);
                this.saveSettings();
            });
        }
    }

    /**
     * 绑定背景图设置
     */
    bindBackgroundImage() {
        const browseBgBtn = document.getElementById('browse-background-image');
        if (browseBgBtn) {
            browseBgBtn.addEventListener('click', async () => {
                try {
                    const result = await invoke('browse_image_file');
                    if (result) {
                        this.settings.backgroundImagePath = result;
                        const bgPathInput = document.getElementById('background-image-path');
                        if (bgPathInput) bgPathInput.value = result;
                        await this.applyBackgroundToSettingsContainer();
                        this.saveSettings();
                    }
                } catch (error) {
                    console.error('浏览背景图片失败:', error);
                    showNotification('浏览图片失败', 'error');
                }
            });
        }
    }

    /**
     * 设置活动主题
     */
    setActiveTheme(theme) {
        document.querySelectorAll('.theme-option').forEach(option => {
            option.classList.remove('active');
        });

        const themeOption = document.querySelector(`[data-theme="${theme}"]`);
        if (themeOption) {
            themeOption.classList.add('active');
        }

        setTheme(theme);
    }

    /**
     * 应用背景图到设置容器
     */
    async applyBackgroundToSettingsContainer() {
        try {
            const container = document.querySelector('.settings-container');
            const path = this.settings.backgroundImagePath || '';
            
            if (container) {
                if (path && this.settings.theme === 'background') {
                    let url = '';
                    try {
                        const dataUrl = await invoke('read_image_file', { filePath: path });
                        url = dataUrl;
                    } catch (e) {
                        url = convertFileSrc ? convertFileSrc(path) : path;
                    }
                    
                    container.style.backgroundImage = `url("${url.replaceAll('"', '\\"')}")`;

                    try {
                        const dominantColor = await getDominantColor(url);
                        const titleBarColors = generateTitleBarColors(dominantColor);
                        applyTitleBarColors(titleBarColors);
                    } catch (colorError) {
                        console.warn('设置页面分析背景图颜色失败:', colorError);
                        removeTitleBarColors();
                    }
                } else {
                    container.style.backgroundImage = '';
                    removeTitleBarColors();
                }
            }
        } catch (e) {
            console.warn('应用背景图片失败:', e);
        }
    }

    /**
     * 更新透明度显示
     */
    updateOpacityDisplay(opacity) {
        const percentage = Math.round(opacity * 100);
        const display = document.querySelector('.slider-value');
        if (display) {
            display.textContent = `${percentage}%`;
        }
    }
}
