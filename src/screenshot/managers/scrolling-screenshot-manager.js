/**
 * 长截屏管理器
 * 负责长截屏功能的控制和预览显示
 */

import { ScreenshotAPI } from '../api/screenshot-api.js';
import { boundsConstraint } from '../utils/bounds-constraint.js';

export class ScrollingScreenshotManager {
    constructor() {
        this.isActive = false;
        this.isPaused = false;
        this.previewImageUrl = null;
        this.selection = null;
        this.lastPanelHeight = null;
        
        // DOM元素
        this.panel = null;
        this.previewImage = null;
        
        // 回调函数
        this.onCancel = null;
        this.onComplete = null;
        
        this.initUI();
    }

    /**
     * 初始化UI元素
     */
    initUI() {
        // 创建长截屏面板
        this.panel = document.createElement('div');
        this.panel.id = 'scrollingScreenshotPanel';
        this.panel.className = 'scrolling-screenshot-panel';
        this.panel.style.display = 'none';
        
        this.panel.innerHTML = `
            <div class="scrolling-preview-wrapper">
                <img class="scrolling-preview-image" alt="预览图" />
            </div>
            <div class="scrolling-controls-area">
                <div class="scrolling-controls">
                    <button id="scrollingStartBtn" class="scrolling-control-btn" data-tooltip="开始">
                        <i class="ti ti-player-play"></i>
                    </button>
                    <button id="scrollingPauseBtn" class="scrolling-control-btn" data-tooltip="暂停" style="display: none;">
                        <i class="ti ti-player-pause"></i>
                    </button>
                    <button id="scrollingResumeBtn" class="scrolling-control-btn" data-tooltip="继续" style="display: none;">
                        <i class="ti ti-player-play"></i>
                    </button>
                    <button id="scrollingStopBtn" class="scrolling-control-btn" data-tooltip="完成">
                        <i class="ti ti-check"></i>
                    </button>
                    <button id="scrollingCancelBtn" class="scrolling-control-btn scrolling-cancel-btn" data-tooltip="取消">
                        <i class="ti ti-x"></i>
                    </button>
                </div>
                <div class="scrolling-info">
                    <span class="scrolling-info-text" id="scrollingStatus">准备</span>
                    <div class="scrolling-info-data">
                        <div class="scrolling-info-item">
                            <i class="ti ti-ruler-measure"></i>
                            <span id="scrollingHeight">0px</span>
                        </div>
                        <div class="scrolling-info-item">
                            <i class="ti ti-photo"></i>
                            <span id="scrollingFrames">0帧</span>
                        </div>
                    </div>
                </div>
            </div>
        `;
        
        document.body.appendChild(this.panel);
        
        this.previewImage = this.panel.querySelector('.scrolling-preview-image');
        this.previewWrapper = this.panel.querySelector('.scrolling-preview-wrapper');

        this.bindEvents();
    }

    /**
     * 绑定控制按钮事件
     */
    bindEvents() {
        const startBtn = document.getElementById('scrollingStartBtn');
        const pauseBtn = document.getElementById('scrollingPauseBtn');
        const resumeBtn = document.getElementById('scrollingResumeBtn');
        const stopBtn = document.getElementById('scrollingStopBtn');
        const cancelBtn = document.getElementById('scrollingCancelBtn');
        
        if (startBtn) {
            startBtn.addEventListener('click', () => this.start());
        }
        if (pauseBtn) {
            pauseBtn.addEventListener('click', () => this.pause());
        }
        if (resumeBtn) {
            resumeBtn.addEventListener('click', () => this.resume());
        }
        if (stopBtn) {
            stopBtn.addEventListener('click', () => this.stop());
        }
        if (cancelBtn) {
            cancelBtn.addEventListener('click', () => this.cancel());
        }
    }

    /**
     * 激活长截屏模式
     */
    async activate(selection) {
        if (this.isActive) return;
        
        this.isActive = true;
        this.isPaused = false;
        this.selection = selection;
        
        // 清理预览状态
        this.clearPreview();
        
        // 隐藏选区 UI 元素
        this.hideSelectionElements();
        
        // 显示并定位控制面板
        this.showPanel(selection);
        
        // 隐藏截屏背景
        this.hideScreenshotBackground();
        
        // 设置初始状态
        this.updateStatus('准备开始');
        
        const stopBtn = document.getElementById('scrollingStopBtn');
        if (stopBtn) {
            stopBtn.disabled = true;
            stopBtn.style.opacity = '0.5';
            stopBtn.style.cursor = 'not-allowed';
        }
        
        // 计算控制面板区域
        const panelRect = this.panel.getBoundingClientRect();
        const panel = {
            left: Math.round(panelRect.left),
            top: Math.round(panelRect.top),
            width: Math.round(panelRect.width),
            height: Math.round(panelRect.height)
        };
        
        // 初始化长截屏服务
        try {
            await ScreenshotAPI.initScrollingScreenshot(selection, panel);
        } catch (error) {
            console.error('初始化长截屏失败:', error);
            this.updateStatus('初始化失败');
        }
    }

    /**
     * 显示并定位面板
     */
    showPanel(selection) {
        this.panel.style.display = 'grid';
        
        const { left, top, width } = selection;
        const panelWidth = 216;
        const screenWidth = window.innerWidth;
        
        // 优先显示在选区右侧，顶部对齐选区顶部
        let panelLeft = left + width + 12;
        let panelTop = top;

        if (panelLeft + panelWidth > screenWidth) {
            panelLeft = left - panelWidth - 12;
            if (panelLeft < 0) {
                panelLeft = 12;
            }
        }
        
        this.panel.style.left = panelLeft + 'px';
        this.panel.style.top = panelTop + 'px';
        this.panel.style.bottom = '';

        this.panelInitialTop = panelTop;
    }

    /**
     * 开始长截屏
     */
    async start() {
        if (!this.isActive) return;
        
        try {
            await ScreenshotAPI.startScrollingScreenshot();
            
            document.getElementById('scrollingStartBtn').style.display = 'none';
            document.getElementById('scrollingPauseBtn').style.display = 'inline-block';
            
            const stopBtn = document.getElementById('scrollingStopBtn');
            if (stopBtn) {
                stopBtn.disabled = false;
                stopBtn.style.opacity = '1';
                stopBtn.style.cursor = 'pointer';
            }
            
            this.updateStatus('滚动中...');
            this.listenForPreviewUpdates();
        } catch (error) {
            console.error('开始长截屏失败:', error);
            this.updateStatus('开始失败: ' + error.message);
        }
    }

    /**
     * 暂停长截屏
     */
    async pause() {
        if (!this.isActive || this.isPaused) return;
        
        try {
            await ScreenshotAPI.pauseScrollingScreenshot();
            this.isPaused = true;
            
            // 切换按钮显示状态
            document.getElementById('scrollingPauseBtn').style.display = 'none';
            document.getElementById('scrollingResumeBtn').style.display = 'inline-block';
            this.updateStatus('已暂停');
        } catch (error) {
            console.error('暂停长截屏失败:', error);
        }
    }

    /**
     * 继续长截屏
     */
    async resume() {
        if (!this.isActive || !this.isPaused) return;
        
        try {
            await ScreenshotAPI.resumeScrollingScreenshot();
            this.isPaused = false;
            
            document.getElementById('scrollingResumeBtn').style.display = 'none';
            document.getElementById('scrollingPauseBtn').style.display = 'inline-block';
            this.updateStatus('滚动中...');
        } catch (error) {
            console.error('继续长截屏失败:', error);
        }
    }

    /**
     * 结束长截屏
     */
    async stop() {
        if (!this.isActive) return;
        
        try {
            await ScreenshotAPI.stopScrollingScreenshot();
            
            // 调用完成回调
            if (this.onComplete) {
                this.onComplete();
            }
        } catch (error) {
            console.error('结束长截屏失败:', error);
            this.updateStatus('保存失败: ' + error.message);
        }
    }

    /**
     * 取消长截屏
     */
    async cancel() {
        if (!this.isActive) return;
        
        try {
            // 显示选区 UI 元素
            this.showSelectionElements();
            
            await ScreenshotAPI.cancelScrollingScreenshot();
            
            // 调用取消回调
            if (this.onCancel) {
                this.onCancel();
            }
            
            this.deactivate();
        } catch (error) {
            console.error('取消长截屏失败:', error);
            this.deactivate();
        }
    }

    /**
     * 清理预览状态
     */
    clearPreview() {
        // 清空图片资源
        if (this.previewImage) {
            this.previewImage.onload = null;
            this.previewImage.src = '';
        }
        
        // 释放 Blob URL 内存
        if (this.previewImageUrl && this.previewImageUrl.startsWith('blob:')) {
            URL.revokeObjectURL(this.previewImageUrl);
        }
        this.previewImageUrl = null;
        this.lastPanelHeight = null;
    }

    /**
     * 停用长截屏模式
     */
    deactivate() {
        this.isActive = false;
        this.isPaused = false;

        this.clearPreview();
        
        this.showSelectionElements();

        this.panel.style.display = 'none';

        this.showScreenshotBackground();

        document.getElementById('scrollingStartBtn').style.display = 'inline-block';
        document.getElementById('scrollingPauseBtn').style.display = 'none';
        document.getElementById('scrollingResumeBtn').style.display = 'none';

        const stopBtn = document.getElementById('scrollingStopBtn');
        if (stopBtn) {
            stopBtn.disabled = true;
            stopBtn.style.opacity = '0.5';
            stopBtn.style.cursor = 'not-allowed';
        }
        
        this.updateStatus('准备开始');
        this.updateInfo(0, 0);
    }

    /**
     * 监听预览更新事件
     */
    listenForPreviewUpdates() {
        // 监听预览更新事件
        window.__TAURI__.event.listen('scrolling-screenshot-preview', (event) => {
            const payload = event.payload;
            this.updatePreview(payload);
        });
        
        // 监听截屏完成事件
        window.__TAURI__.event.listen('scrolling-screenshot-complete', (event) => {
            const payload = event.payload;
            this.handleComplete(payload);
        });
        
        // 监听截屏错误事件
        window.__TAURI__.event.listen('scrolling-screenshot-error', (event) => {
            const error = event.payload;
            this.updateStatus('错误: ' + error);
        });
    }

    /**
     * 更新预览图片
     */
    updatePreview(payload) {
        if (!payload || !this.previewImage) return;
        
        const { image_url, height, frames } = payload;

        // 更新显示信息
        this.updateInfo(height, frames);

        // 延迟释放前一个 Blob URL 避免闪烁
        if (this.previewImageUrl && this.previewImageUrl !== image_url && this.previewImageUrl.startsWith('blob:')) {
            setTimeout(() => URL.revokeObjectURL(this.previewImageUrl), 200);
        }

        // 设置图片源
        this.previewImageUrl = image_url;
        this.previewImage.src = image_url;
        
        // 图片加载完成后的处理
        this.previewImage.onload = () => {
            // 自动滚动到预览底部
            if (this.previewWrapper) {
                this.previewWrapper.scrollTop = this.previewWrapper.scrollHeight;
            }
            
            // 检测面板高度变化并通知后端
            const currentHeight = this.panel.getBoundingClientRect().height;
            if (!this.lastPanelHeight || Math.abs(currentHeight - this.lastPanelHeight) > 10) {
                this.lastPanelHeight = currentHeight;
                this.updatePanelRect();
            }
        };
    }

    /**
     * 处理长截屏完成
     */
    handleComplete(payload) {
        const { image_url, height, frames } = payload;
        
        // 更新显示信息
        this.updateInfo(height, frames);
        this.updateStatus('长截屏完成！');

        this.updatePreview(payload);
    }

    /**
     * 更新状态文本
     */
    updateStatus(text) {
        const statusText = document.getElementById('scrollingStatus');
        if (statusText) {
            statusText.textContent = text;
        }
    }

    /**
     * 更新信息显示
     */
    updateInfo(height, frames) {
        const heightElem = document.getElementById('scrollingHeight');
        const framesElem = document.getElementById('scrollingFrames');
        
        if (heightElem) {
            heightElem.textContent = `${height}px`;
        }
        if (framesElem) {
            framesElem.textContent = `${frames}帧`;
        }
    }

    /**
     * 隐藏截屏背景
     */
    hideScreenshotBackground() {
        const backgroundCanvas = document.querySelector('#screenshot-background');
        if (backgroundCanvas) {
            backgroundCanvas.style.opacity = '0';
        }
    }

    /**
     * 恢复截屏背景
     */
    showScreenshotBackground() {
        const backgroundCanvas = document.querySelector('#screenshot-background');
        if (backgroundCanvas) {
            backgroundCanvas.style.opacity = '1';
        }
    }


    /**
     * 设置取消回调
     */
    setOnCancel(callback) {
        this.onCancel = callback;
    }

    setOnComplete(callback) {
        this.onComplete = callback;
    }

    /**
     * 更新面板区域
     */
    async updatePanelRect() {
        if (!this.panel || !this.isActive) return;
        
        try {
            const panelRect = this.panel.getBoundingClientRect();
            const panel = {
                left: Math.round(panelRect.left),
                top: Math.round(panelRect.top),
                width: Math.round(panelRect.width),
                height: Math.round(panelRect.height)
            };
            
            await ScreenshotAPI.updateScrollingPanelRect(panel);
        } catch (error) {
            console.error('更新面板区域失败:', error);
        }
    }

    /**
     * 动态调整面板位置
     */
    adjustPanelExpansion() {
        if (!this.panel || this.panelInitialTop === undefined) return;
        
        const panelRect = this.panel.getBoundingClientRect();
        const panelWidth = panelRect.width;
        const currentPanelHeight = panelRect.height;
        const panelLeft = parseInt(this.panel.style.left) || 0;
        
        const constrained = boundsConstraint.constrain(
            panelLeft, 
            this.panelInitialTop, 
            panelWidth, 
            currentPanelHeight
        );
        
        // 应用约束后的位置
        this.panel.style.top = constrained.y + 'px';
    }

    /**
     * 隐藏选区内的UI元素
     */
    hideSelectionElements() {

        const resizeHandles = document.querySelectorAll('.resize-handle');
        resizeHandles.forEach(handle => {
            handle.style.display = 'none';
        });
        
        const radiusHandles = document.querySelectorAll('.radius-handle');
        radiusHandles.forEach(handle => {
            handle.style.display = 'none';
        });

        const selectionInfo = document.getElementById('selectionInfo');
        if (selectionInfo) {
            selectionInfo.style.display = 'none';
        }

        const maskLayer = document.getElementById('maskLayer');
        if (maskLayer) {
            maskLayer.style.display = 'none';
        }

        const helpPanel = document.getElementById('helpPanel');
        if (helpPanel) {
            helpPanel.style.display = 'none';
        }

        this.elementsHidden = true;
    }

    /**
     * 显示选区内的UI元素
     */
    showSelectionElements() {

        const resizeHandles = document.querySelectorAll('.resize-handle');
        resizeHandles.forEach(handle => {
            handle.style.display = 'block';
        });

        const radiusHandles = document.querySelectorAll('.radius-handle');
        radiusHandles.forEach(handle => {
            handle.style.display = 'block';
        });

        const selectionInfo = document.getElementById('selectionInfo');
        if (selectionInfo) {
            selectionInfo.style.display = 'inline-flex';
        }

        const maskLayer = document.getElementById('maskLayer');
        if (maskLayer) {
            maskLayer.style.display = 'block';
        }

        const helpPanel = document.getElementById('helpPanel');
        if (helpPanel) {
            helpPanel.style.display = 'flex';
        }
        
        this.elementsHidden = false;
    }

    /**
     * 清理资源
     */
    clear() {
        if (this.isActive) {
            this.cancel();
        }
    }
}

