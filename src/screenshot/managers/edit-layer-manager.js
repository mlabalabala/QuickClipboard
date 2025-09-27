/**
 * 编辑层管理器
 * 负责管理截屏上的编辑内容（画笔、文字、标注等）
 */

export class EditLayerManager {
    constructor() {
        this.canvas = null;
        this.ctx = null;
        this.backgroundManager = null;
        
        // 编辑历史记录
        this.history = [];
        this.historyIndex = -1;
        this.maxHistory = 20;
    }

    /**
     * 初始化编辑层Canvas
     */
    init() {
        // 如果已经存在，先清理
        if (this.canvas) {
            this.destroy();
        }

        // 创建编辑层Canvas
        this.canvas = document.createElement('canvas');
        this.canvas.id = 'edit-layer';
        this.canvas.style.cssText = `
            position: fixed;
            top: 0;
            left: 0;
            width: 100vw;
            height: 100vh;
            z-index: 1;
            pointer-events: none;
        `;

        this.ctx = this.canvas.getContext('2d');
        document.body.appendChild(this.canvas);
        
        this.updateCanvasSize();
    }

    /**
     * 设置背景管理器引用
     */
    setBackgroundManager(backgroundManager) {
        this.backgroundManager = backgroundManager;
        if (backgroundManager?.canvas) {
            this.updateCanvasSize();
        }
    }

    /**
     * 更新Canvas尺寸（与背景Canvas保持一致）
     */
    updateCanvasSize() {
        if (!this.canvas || !this.backgroundManager?.canvas) return;

        const bgCanvas = this.backgroundManager.canvas;
        this.canvas.width = bgCanvas.width;
        this.canvas.height = bgCanvas.height;
        
        // 保持CSS显示尺寸与背景一致
        this.canvas.style.width = '100vw';
        this.canvas.style.height = '100vh';
    }

    /**
     * 启用编辑层的鼠标事件
     */
    enableInteraction() {
        if (this.canvas) {
            this.canvas.style.pointerEvents = 'auto';
        }
    }

    /**
     * 禁用编辑层的鼠标事件
     */
    disableInteraction() {
        if (this.canvas) {
            this.canvas.style.pointerEvents = 'none';
        }
    }

    /**
     * 清除编辑层内容
     */
    clear() {
        if (this.ctx && this.canvas) {
            this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        }
    }

    /**
     * 保存当前状态到历史记录
     */
    saveState() {
        if (!this.canvas) return;

        // 删除当前位置之后的历史记录
        this.history = this.history.slice(0, this.historyIndex + 1);

        // 保存当前Canvas状态
        const imageData = this.ctx.getImageData(0, 0, this.canvas.width, this.canvas.height);
        this.history.push(imageData);

        // 限制历史记录数量
        if (this.history.length > this.maxHistory) {
            this.history.shift();
        } else {
            this.historyIndex++;
        }
    }

    /**
     * 撤销上一步操作
     */
    undo() {
        if (this.historyIndex > 0) {
            this.historyIndex--;
            const imageData = this.history[this.historyIndex];
            this.ctx.putImageData(imageData, 0, 0);
            return true;
        }
        return false;
    }

    /**
     * 重做下一步操作
     */
    redo() {
        if (this.historyIndex < this.history.length - 1) {
            this.historyIndex++;
            const imageData = this.history[this.historyIndex];
            this.ctx.putImageData(imageData, 0, 0);
            return true;
        }
        return false;
    }

    /**
     * 检查是否可以撤销
     */
    canUndo() {
        return this.historyIndex > 0;
    }

    /**
     * 检查是否可以重做
     */
    canRedo() {
        return this.historyIndex < this.history.length - 1;
    }

    /**
     * 将屏幕坐标转换为Canvas坐标
     */
    screenToCanvasCoords(screenX, screenY) {
        if (!this.canvas) return { x: screenX, y: screenY };

        const rect = this.canvas.getBoundingClientRect();
        const scaleX = this.canvas.width / rect.width;
        const scaleY = this.canvas.height / rect.height;

        return {
            x: (screenX - rect.left) * scaleX,
            y: (screenY - rect.top) * scaleY
        };
    }

    /**
     * 合并编辑层和背景层（用于导出）
     */
    mergeWithBackground() {
        if (!this.canvas || !this.backgroundManager?.canvas) {
            throw new Error('Canvas未准备就绪');
        }

        // 创建合成Canvas
        const mergedCanvas = document.createElement('canvas');
        const bgCanvas = this.backgroundManager.canvas;
        
        mergedCanvas.width = bgCanvas.width;
        mergedCanvas.height = bgCanvas.height;
        const mergedCtx = mergedCanvas.getContext('2d');

        // 先绘制背景
        mergedCtx.drawImage(bgCanvas, 0, 0);
        
        // 再绘制编辑层
        mergedCtx.drawImage(this.canvas, 0, 0);

        return mergedCanvas;
    }

    /**
     * 获取编辑层是否有内容
     */
    hasContent() {
        if (!this.canvas || !this.ctx) return false;

        // 检查Canvas是否有非透明像素
        const imageData = this.ctx.getImageData(0, 0, this.canvas.width, this.canvas.height);
        const data = imageData.data;
        
        for (let i = 3; i < data.length; i += 4) {
            if (data[i] > 0) { // alpha > 0
                return true;
            }
        }
        return false;
    }

    /**
     * 销毁编辑层
     */
    destroy() {
        if (this.canvas && this.canvas.parentNode) {
            this.canvas.parentNode.removeChild(this.canvas);
        }
        this.canvas = null;
        this.ctx = null;
        this.history = [];
        this.historyIndex = -1;
    }

    /**
     * 获取编辑层Canvas（供工具使用）
     */
    getCanvas() {
        return this.canvas;
    }

    /**
     * 获取编辑层Context（供工具使用）
     */
    getContext() {
        return this.ctx;
    }
}
