const { invoke } = window.__TAURI__.core;

class ScreenshotManager {
    constructor() {
        this.isSelecting = false;
        this.isMoving = false;
        this.startX = 0;
        this.startY = 0;
        this.currentX = 0;
        this.currentY = 0;
        this.selectionRect = null;
        this.moveOffsetX = 0;
        this.moveOffsetY = 0;
        this.monitors = [];
        
        this.overlay = document.getElementById('overlay');
        this.selectionArea = document.getElementById('selectionArea');
        this.toolbar = document.getElementById('toolbar');
        this.infoText = document.getElementById('infoText');
        this.selectionInfo = document.getElementById('selectionInfo');
        this.maskTop = document.getElementById('maskTop');
        this.maskBottom = document.getElementById('maskBottom');
        this.maskLeft = document.getElementById('maskLeft');
        this.maskRight = document.getElementById('maskRight');
        
        this.initEvents();
        this.loadMonitorInfo();
    }

    initEvents() {
        this.overlay.addEventListener('mousedown', (e) => this.startSelection(e));
        this.selectionArea.addEventListener('mousedown', (e) => this.startSelection(e));
        document.addEventListener('mousemove', (e) => this.updateSelection(e));
        document.addEventListener('mouseup', (e) => this.endSelection(e));
        document.addEventListener('keydown', (e) => this.handleKeydown(e));
        document.addEventListener('contextmenu', (e) => this.handleRightClick(e));
        document.getElementById('confirmBtn').addEventListener('click', () => this.confirmScreenshot());
        document.getElementById('cancelBtn').addEventListener('click', () => this.cancelScreenshot());
    }

    startSelection(e) {
        if (e.button !== 0) return;
        e.preventDefault();
        e.stopPropagation();
        
        const mouseX = e.clientX;
        const mouseY = e.clientY;
        
        // 检查是否点击在现有选区内
        if (this.selectionRect && this.isPointInSelection(mouseX, mouseY)) {
            // 在选区内：开始移动选区
            this.isMoving = true;
            this.isSelecting = false;
            this.moveOffsetX = mouseX - this.selectionRect.left;
            this.moveOffsetY = mouseY - this.selectionRect.top;
            this.hideToolbar();
        } else {
            // 在选区外：开始新的选区
        this.isSelecting = true;
            this.isMoving = false;
            this.startX = mouseX;
            this.startY = mouseY;
            this.currentX = mouseX;
            this.currentY = mouseY;
            
            this.selectionArea.style.display = 'block';
            this.infoText.style.opacity = '0';
        document.body.classList.add('has-selection');
            this.hideToolbar();
        
            this.updateDisplay();
        }
    }

    updateSelection(e) {
        if (!this.isSelecting && !this.isMoving) return;
        
        if (this.isMoving) {
            // 移动模式：移动整个选区
            this.moveSelection(e);
        } else {
            // 选择模式：调整选区大小
        this.currentX = e.clientX;
        this.currentY = e.clientY;
            this.updateDisplay();
        }
        
        // 确保工具栏在操作过程中保持隐藏
        this.hideToolbar();
    }

    endSelection(e) {
        if ((!this.isSelecting && !this.isMoving) || e.button !== 0) return;
        
        if (this.isMoving) {
            // 结束移动
            this.isMoving = false;
            this.showToolbar();
        } else {
            // 结束选择
        this.isSelecting = false;
        
        const width = Math.abs(this.currentX - this.startX);
        const height = Math.abs(this.currentY - this.startY);
        
        if (width > 10 && height > 10) {
                this.showToolbar();
        } else {
            this.reset();
            }
        }
    }

    updateDisplay() {
        const left = Math.min(this.startX, this.currentX);
        const top = Math.min(this.startY, this.currentY);
        const width = Math.abs(this.currentX - this.startX);
        const height = Math.abs(this.currentY - this.startY);
        
        // 更新选择区域
        this.selectionArea.style.left = left + 'px';
        this.selectionArea.style.top = top + 'px';
        this.selectionArea.style.width = width + 'px';
        this.selectionArea.style.height = height + 'px';
        
        // 更新信息显示
        this.selectionInfo.textContent = `${width} × ${height}`;
        this.selectionInfo.style.left = '8px';
        this.selectionInfo.style.top = (top < 40 ? height + 8 : -30) + 'px';
        
        // 更新遮罩层
        this.updateMask(left, top, width, height);
        
        this.selectionRect = { left, top, width, height };
    }

    updateMask(left, top, width, height) {
        const right = left + width;
        const bottom = top + height;
        const screenWidth = window.innerWidth;
        const screenHeight = window.innerHeight;
        
        this.maskTop.style.cssText = `left: 0; top: 0; width: ${screenWidth}px; height: ${Math.max(0, top)}px; position: absolute; background: rgba(0, 0, 0, 0.5);`;
        this.maskBottom.style.cssText = `left: 0; top: ${Math.min(bottom, screenHeight)}px; width: ${screenWidth}px; height: ${Math.max(0, screenHeight - bottom)}px; position: absolute; background: rgba(0, 0, 0, 0.5);`;
        this.maskLeft.style.cssText = `left: 0; top: ${Math.max(0, top)}px; width: ${Math.max(0, left)}px; height: ${Math.max(0, Math.min(height, screenHeight - top))}px; position: absolute; background: rgba(0, 0, 0, 0.5);`;
        this.maskRight.style.cssText = `left: ${Math.min(right, screenWidth)}px; top: ${Math.max(0, top)}px; width: ${Math.max(0, screenWidth - right)}px; height: ${Math.max(0, Math.min(height, screenHeight - top))}px; position: absolute; background: rgba(0, 0, 0, 0.5);`;
    }

    handleKeydown(e) {
        if (e.key === 'Escape') {
                e.preventDefault();
                this.cancelScreenshot();
        } else if (e.key === 'Enter' && this.selectionRect) {
                    e.preventDefault();
                    this.confirmScreenshot();
                }
    }

    showToolbar() {
        if (!this.selectionRect) return;
        
        const { left, top, width, height } = this.selectionRect;
        
        // 计算工具栏尺寸 (2个32px按钮 + 2个4px内边距 + 1个4px间距)
        const toolbarWidth = 32 * 2 + 4 * 2 + 4; // 76px
        const toolbarHeight = 32 + 4 * 2; // 40px
        
        // 计算工具栏位置：选区右下角，右对齐
        let toolbarLeft = left + width - toolbarWidth;
        let toolbarTop = top + height + 8; // 选区下方8px
        
        // 边界检测
        const screenWidth = window.innerWidth;
        const screenHeight = window.innerHeight;
        
        // 防止超出右边界
        if (toolbarLeft + toolbarWidth > screenWidth - 8) {
            toolbarLeft = screenWidth - toolbarWidth - 8;
        }
        
        // 防止超出左边界
        if (toolbarLeft < 8) {
            toolbarLeft = left; // 与选区左边对齐
        }
        
        // 如果下方空间不足，显示在选区上方
        if (toolbarTop + toolbarHeight > screenHeight - 8) {
            toolbarTop = top - toolbarHeight - 8;
        }
        
        // 设置工具栏位置并显示
        this.toolbar.style.left = toolbarLeft + 'px';
        this.toolbar.style.top = toolbarTop + 'px';
        this.toolbar.classList.add('visible');
    }

    hideToolbar() {
                this.toolbar.classList.remove('visible');
            }
            
    handleRightClick(e) {
        e.preventDefault();
        
        if (this.selectionRect) {
            // 有选区时：取消选区，回到初始状态
            this.clearSelection();
        } else {
            // 没有选区时：关闭截屏窗口
            this.cancelScreenshot();
        }
    }

    async loadMonitorInfo() {
        // 只在初始化时加载一次
        if (this.monitors.length > 0) return;
        
        try {
            // 使用后端统一处理的CSS像素数据
            this.monitors = await invoke('get_css_monitors');
        } catch (error) {
            console.error('获取显示器信息失败:', error);
            // 使用默认单显示器配置
            this.monitors = [{
                x: 0,
                y: 0,
                width: window.innerWidth,
                height: window.innerHeight,
                is_primary: true
            }];
        }
    }


    clearSelection() {
        this.isSelecting = false;
        this.isMoving = false;
        this.selectionRect = null;
        this.selectionArea.style.display = 'none';
        this.hideToolbar();
            this.infoText.style.opacity = '1';
        this.infoText.textContent = '拖拽选择截屏区域，选区内可拖拽移动，右键取消/关闭，按 ESC 键关闭';
        document.body.classList.remove('has-selection');
        
        // 重置遮罩层为全屏状态
        this.maskTop.style.cssText = 'top: 0; left: 0; width: 100%; height: 100%;';
        this.maskBottom.style.cssText = 'display: none;';
        this.maskLeft.style.cssText = 'display: none;';
        this.maskRight.style.cssText = 'display: none;';
    }

    isPointInSelection(x, y) {
        if (!this.selectionRect) return false;
        
        const { left, top, width, height } = this.selectionRect;
        return x >= left && x <= left + width && 
               y >= top && y <= top + height;
    }

    async moveSelection(e) {
        if (!this.selectionRect) return;
        
        const mouseX = e.clientX;
        const mouseY = e.clientY;
        
        // 计算新的选区位置
        let newLeft = mouseX - this.moveOffsetX;
        let newTop = mouseY - this.moveOffsetY;
        
        // 使用后端边界约束（复用后端的显示器边界逻辑）
        const { width, height } = this.selectionRect;
        try {
            const [constrainedX, constrainedY] = await invoke('constrain_selection_bounds', {
                x: newLeft,
                y: newTop,
                width,
                height
            });
            newLeft = constrainedX;
            newTop = constrainedY;
        } catch (error) {
            console.error('边界约束失败:', error);
            // 降级到简单边界检查
            newLeft = Math.max(0, Math.min(newLeft, window.innerWidth - width));
            newTop = Math.max(0, Math.min(newTop, window.innerHeight - height));
        }
        
        // 更新选区位置
        this.selectionRect = {
            left: newLeft,
            top: newTop,
            width: width,
            height: height
        };
        
        // 直接更新，避免过度优化导致的性能问题
        this.selectionArea.style.left = newLeft + 'px';
        this.selectionArea.style.top = newTop + 'px';
        this.selectionArea.style.width = width + 'px';
        this.selectionArea.style.height = height + 'px';
        
        // 更新选区信息显示
        this.selectionInfo.textContent = `${width} × ${height}`;
        this.selectionInfo.style.left = '8px';
        this.selectionInfo.style.top = (newTop < 40 ? height + 8 : -30) + 'px';
        
        // 更新遮罩层
        this.updateMask(newLeft, newTop, width, height);
    }


    async confirmScreenshot() {
        if (!this.selectionRect) return;
        
        try {
            this.toolbar.classList.remove('visible');
            await new Promise(resolve => setTimeout(resolve, 100));
            // TODO: 实现实际截屏逻辑
            await invoke('hide_screenshot_window');
        } catch (error) {
            console.error('截屏失败:', error);
        }
    }

    async cancelScreenshot() {
        try {
            await invoke('hide_screenshot_window');
        } catch (error) {
            console.error('隐藏窗口失败:', error);
        }
    }

    reset() {
        this.isSelecting = false;
        this.isMoving = false;
        this.selectionRect = null;
        this.selectionArea.style.display = 'none';
        this.toolbar.classList.remove('visible');
        this.infoText.style.opacity = '1';
        this.infoText.textContent = '拖拽选择截屏区域，选区内可拖拽移动，右键取消/关闭，按 ESC 键关闭';
        document.body.classList.remove('has-selection');
        
        // 重置遮罩层
        [this.maskTop, this.maskBottom, this.maskLeft, this.maskRight].forEach(mask => {
            mask.style.cssText = '';
        });
    }
}

// 初始化
let screenshotManager = null;

document.addEventListener('DOMContentLoaded', () => {
    screenshotManager = new ScreenshotManager();
});

window.addEventListener('focus', () => {
    screenshotManager?.reset();
});

window.addEventListener('blur', () => {
    screenshotManager?.reset();
});