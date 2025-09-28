/**
 * Fabric.js编辑层管理器
 * 使用Fabric.js重构的编辑层管理器，提供更强大的对象管理和编辑功能
 */

export class FabricEditLayerManager {
    constructor() {
        this.canvas = null;
        this.fabricCanvas = null;
        this.backgroundManager = null;
        
        // 历史管理
        this.historyStack = [];
        this.historyStep = -1;
        this.maxHistorySize = 50;
        
        // 回调函数
        this.onHistoryChange = null;
        this.onObjectModified = null;
        
        // 绘画模式状态
        this.isDrawingMode = false;
        
        // 坐标缩放比例（用于导出时的坐标转换）
        this.scaleX = 1;
        this.scaleY = 1;
    }

    /**
     * 初始化Fabric Canvas
     */
    init() {
        // 如果已经存在，先清理
        if (this.fabricCanvas) {
            this.destroy();
        }

        // 创建Canvas元素
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

        document.body.appendChild(this.canvas);
        
        // 初始化Fabric Canvas
        this.fabricCanvas = new fabric.Canvas(this.canvas, {
            selection: true,
            preserveObjectStacking: true,
            enableRetinaScaling: true,
            imageSmoothingEnabled: false,
            skipTargetFind: false,
            interactive: true,
            hoverCursor: 'move',
            moveCursor: 'move',
            defaultCursor: 'default'
        });

        this.updateCanvasSize();
        this.initFabricEvents();
        
        // 初始化画笔
        this.initBrush();
        
        // 延迟保存初始状态，确保Canvas完全准备好
        setTimeout(() => {
            this.saveState('初始状态');
        }, 100);
    }

    /**
     * 初始化画笔
     */
    initBrush() {
        if (!this.fabricCanvas) return;
        
        // 创建画笔实例
        this.fabricCanvas.freeDrawingBrush = new fabric.PencilBrush(this.fabricCanvas);
        
        // 设置默认画笔选项
        const brush = this.fabricCanvas.freeDrawingBrush;
        brush.color = '#ff0000';
        brush.width = 3;
        brush.lineCap = 'round';
        brush.lineJoin = 'round';
    }

    /**
     * 初始化Fabric事件
     */
    initFabricEvents() {
        // 对象修改事件（移动、缩放等）
        this.fabricCanvas.on('object:modified', () => {
            if (!this.isLoadingFromHistory) {
                this.saveState('对象修改');
                this.onObjectModified?.();
            }
        });

        // 路径创建完成事件（绘画模式） - 只保存这一个就够了
        this.fabricCanvas.on('path:created', () => {
            if (!this.isLoadingFromHistory) {
                this.saveState('绘制路径');
            }
        });

        // 对象移除事件
        this.fabricCanvas.on('object:removed', (e) => {
            if (!this.isLoadingFromHistory) {
                this.saveState('删除对象');
            }
        });
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
     * 设置历史状态改变回调
     */
    setOnHistoryChange(callback) {
        this.onHistoryChange = callback;
    }

    /**
     * 设置对象修改回调
     */
    setOnObjectModified(callback) {
        this.onObjectModified = callback;
    }

    /**
     * 更新Canvas尺寸（与背景Canvas保持一致）
     */
    updateCanvasSize() {
        if (!this.fabricCanvas || !this.backgroundManager?.canvas) return;

        const bgCanvas = this.backgroundManager.canvas;
        
        // 获取视口尺寸
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;
        
        // 设置Fabric Canvas的逻辑尺寸为视口尺寸，这样坐标系统就一致了
        this.fabricCanvas.setWidth(viewportWidth);
        this.fabricCanvas.setHeight(viewportHeight);
        
        // 设置CSS显示尺寸与逻辑尺寸一致
        this.canvas.style.width = viewportWidth + 'px';
        this.canvas.style.height = viewportHeight + 'px';
        
        // 计算缩放比例，用于导出时的坐标转换
        this.scaleX = bgCanvas.width / viewportWidth;
        this.scaleY = bgCanvas.height / viewportHeight;
        
        
        // 重新渲染
        this.fabricCanvas.renderAll();
    }

    /**
     * 启用编辑层的鼠标事件
     */
    enableInteraction() {
        if (this.canvas) {
            this.canvas.style.pointerEvents = 'auto';
        }
        if (this.fabricCanvas) {
            this.fabricCanvas.selection = true;
            this.fabricCanvas.interactive = true;
            this.fabricCanvas.skipTargetFind = false;
            
            // 关键修复：确保upperCanvas能接收鼠标事件
            if (this.fabricCanvas.upperCanvasEl) {
                this.fabricCanvas.upperCanvasEl.style.pointerEvents = 'auto';
            }
        }
    }

    /**
     * 禁用编辑层的鼠标事件
     */
    disableInteraction() {
        if (this.canvas) {
            this.canvas.style.pointerEvents = 'none';
        }
        if (this.fabricCanvas) {
            this.fabricCanvas.selection = false;
            this.fabricCanvas.interactive = false;
            this.fabricCanvas.skipTargetFind = true;
            this.fabricCanvas.discardActiveObject();
            
            // 同时禁用upperCanvas的鼠标事件
            if (this.fabricCanvas.upperCanvasEl) {
                this.fabricCanvas.upperCanvasEl.style.pointerEvents = 'none';
            }
            
            this.fabricCanvas.renderAll();
        }
    }

    /**
     * 启用绘画模式
     */
    enableDrawingMode(brushOptions = {}) {
        if (!this.fabricCanvas) return;

        this.isDrawingMode = true;
        this.fabricCanvas.isDrawingMode = true;
        
        // 设置画笔选项
        const brush = this.fabricCanvas.freeDrawingBrush;
        if (brush) {
            brush.color = brushOptions.color || '#ff0000';
            brush.width = brushOptions.width || 3;
            
            // 设置其他画笔属性
            if (brushOptions.shadowColor !== undefined) {
                brush.shadowColor = brushOptions.shadowColor;
            }
            if (brushOptions.shadowBlur !== undefined) {
                brush.shadowBlur = brushOptions.shadowBlur;
            }
        }
        
        // 禁用对象选择
        this.fabricCanvas.selection = false;
        this.fabricCanvas.forEachObject((obj) => {
            obj.selectable = false;
        });
        
        this.fabricCanvas.renderAll();
    }

    /**
     * 禁用绘画模式
     */
    disableDrawingMode() {
        if (!this.fabricCanvas) return;

        this.isDrawingMode = false;
        this.fabricCanvas.isDrawingMode = false;
        
        // 恢复对象选择
        this.fabricCanvas.selection = true;
        this.fabricCanvas.forEachObject((obj) => {
            obj.selectable = true;
        });
        
        this.fabricCanvas.renderAll();
        
        // 不在这里保存状态，避免工具切换时的干扰
    }

    /**
     * 设置画笔选项
     */
    setBrushOptions(options) {
        if (!this.fabricCanvas) return;
        
        const brush = this.fabricCanvas.freeDrawingBrush;
        if (!brush) return;
        
        if (options.color !== undefined) brush.color = options.color;
        if (options.width !== undefined) brush.width = options.width;
        if (options.shadowColor !== undefined) brush.shadowColor = options.shadowColor;
        if (options.shadowBlur !== undefined) brush.shadowBlur = options.shadowBlur;
    }

    /**
     * 添加文本对象
     */
    addText(text, options = {}) {
        if (!this.fabricCanvas) return null;

        const textObj = new fabric.Text(text, {
            left: options.left || 100,
            top: options.top || 100,
            fontFamily: options.fontFamily || 'Arial',
            fontSize: options.fontSize || 20,
            fill: options.color || '#000000',
            editable: true
        });

        this.fabricCanvas.add(textObj);
        this.fabricCanvas.setActiveObject(textObj);
        this.fabricCanvas.renderAll();
        
        return textObj;
    }

    /**
     * 添加矩形
     */
    addRectangle(options = {}) {
        if (!this.fabricCanvas) return null;

        const rect = new fabric.Rect({
            left: options.left || 100,
            top: options.top || 100,
            width: options.width || 100,
            height: options.height || 100,
            fill: options.fill || 'rgba(255, 0, 0, 0.3)',
            stroke: options.stroke || '#ff0000',
            strokeWidth: options.strokeWidth || 2
        });

        this.fabricCanvas.add(rect);
        this.fabricCanvas.setActiveObject(rect);
        this.fabricCanvas.renderAll();
        
        return rect;
    }

    /**
     * 添加圆形
     */
    addCircle(options = {}) {
        if (!this.fabricCanvas) return null;

        const circle = new fabric.Circle({
            left: options.left || 100,
            top: options.top || 100,
            radius: options.radius || 50,
            fill: options.fill || 'rgba(0, 255, 0, 0.3)',
            stroke: options.stroke || '#00ff00',
            strokeWidth: options.strokeWidth || 2
        });

        this.fabricCanvas.add(circle);
        this.fabricCanvas.setActiveObject(circle);
        this.fabricCanvas.renderAll();
        
        return circle;
    }

    /**
     * 添加箭头
     */
    addArrow(options = {}) {
        if (!this.fabricCanvas) return null;

        // 创建箭头路径
        const arrowPath = 'M 0 0 L 100 0 M 90 -10 L 100 0 L 90 10';
        
        const arrow = new fabric.Path(arrowPath, {
            left: options.left || 100,
            top: options.top || 100,
            stroke: options.color || '#0000ff',
            strokeWidth: options.width || 3,
            fill: '',
            scaleX: options.scaleX || 1,
            scaleY: options.scaleY || 1
        });

        this.fabricCanvas.add(arrow);
        this.fabricCanvas.setActiveObject(arrow);
        this.fabricCanvas.renderAll();
        
        return arrow;
    }

    /**
     * 删除选中的对象
     */
    deleteSelected() {
        if (!this.fabricCanvas) return;

        const activeObjects = this.fabricCanvas.getActiveObjects();
        if (activeObjects.length > 0) {
            this.fabricCanvas.remove(...activeObjects);
            this.fabricCanvas.discardActiveObject();
            this.fabricCanvas.renderAll();
        }
    }

    /**
     * 清除所有内容
     */
    clear() {
        if (this.fabricCanvas) {
            this.fabricCanvas.clear();
            this.fabricCanvas.renderAll();
        }
    }

    /**
     * 保存当前状态到历史记录
     */
    saveState(description = '') {
        if (!this.fabricCanvas || this.isLoadingFromHistory) return;

        try {
            // 删除当前位置之后的历史记录
            if (this.historyStep < this.historyStack.length - 1) {
                this.historyStack.splice(this.historyStep + 1);
            }

            // 获取当前状态
            const state = JSON.stringify(this.fabricCanvas.toJSON());
            
            // 避免保存重复状态
            if (this.historyStack.length > 0 && 
                this.historyStack[this.historyStack.length - 1].state === state) {
                return;
            }
            
            // 添加到历史记录
            this.historyStack.push({ state, description, timestamp: Date.now() });
            this.historyStep = this.historyStack.length - 1;

            // 限制历史记录大小
            if (this.historyStack.length > this.maxHistorySize) {
                this.historyStack.shift();
                this.historyStep--;
            }

            // 触发历史状态改变回调
            this.triggerHistoryChange();

        } catch (error) {
            console.error('FabricEditLayerManager: Error saving state:', error);
        }
    }

    /**
     * 撤销操作
     */
    async undo() {
        if (!this.canUndo()) return false;

        try {
            this.historyStep--;
            const historyItem = this.historyStack[this.historyStep];
            await this.loadHistoryState(historyItem.state);
            return true;
        } catch (error) {
            console.error('FabricEditLayerManager: Error during undo:', error);
            this.historyStep++;
            return false;
        }
    }

    /**
     * 重做操作
     */
    async redo() {
        if (!this.canRedo()) return false;

        try {
            this.historyStep++;
            const historyItem = this.historyStack[this.historyStep];
            await this.loadHistoryState(historyItem.state);
            return true;
        } catch (error) {
            console.error('FabricEditLayerManager: Error during redo:', error);
            this.historyStep--;
            return false;
        }
    }

    /**
     * 加载历史状态的通用方法
     */
    async loadHistoryState(stateJson) {
        return new Promise((resolve, reject) => {
            this.isLoadingFromHistory = true;
            
            // 先清空Canvas，避免loadFromJSON的合并行为
            this.fabricCanvas.clear();
            
            // 防止重复回调
            let callbackExecuted = false;
            
            // 解析JSON状态
            const stateData = typeof stateJson === 'string' ? JSON.parse(stateJson) : stateJson;
            
            // 加载状态
            this.fabricCanvas.loadFromJSON(stateData, () => {
                if (callbackExecuted) return;
                callbackExecuted = true;
                
                // 恢复Canvas尺寸
                this.updateCanvasSize();
                
                // 重新渲染
                this.fabricCanvas.renderAll();
                this.fabricCanvas.requestRenderAll();
                
                // 延迟重置标志，确保渲染完成
                setTimeout(() => {
                    this.isLoadingFromHistory = false;
                    this.triggerHistoryChange();
                    resolve();
                }, 10);
            }, (error) => {
                this.isLoadingFromHistory = false;
                reject(error);
            });
        });
    }

    /**
     * 检查是否可以撤销
     */
    canUndo() {
        return this.historyStep > 0;
    }

    /**
     * 检查是否可以重做
     */
    canRedo() {
        return this.historyStep < this.historyStack.length - 1;
    }

    /**
     * 清除历史记录
     */
    clearHistory() {
        this.historyStack = [];
        this.historyStep = -1;
        this.triggerHistoryChange();
    }

    /**
     * 触发历史状态改变回调
     */
    triggerHistoryChange() {
        if (this.onHistoryChange) {
            this.onHistoryChange({
                canUndo: this.canUndo(),
                canRedo: this.canRedo(),
                currentIndex: this.historyStep,
                totalCount: this.historyStack.length
            });
        }
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
        if (!this.fabricCanvas || !this.backgroundManager?.canvas) {
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
        
        // 再绘制编辑层，需要按比例缩放到背景Canvas尺寸
        const editLayerDataURL = this.fabricCanvas.toDataURL({
            format: 'png',
            quality: 1,
            multiplier: this.scaleX || 1 // 使用缩放比例
        });
        
        return new Promise((resolve) => {
            const img = new Image();
            img.onload = () => {
                // 如果有缩放比例，需要调整绘制尺寸
                if (this.scaleX && this.scaleY) {
                    mergedCtx.drawImage(img, 0, 0, bgCanvas.width, bgCanvas.height);
                } else {
                    mergedCtx.drawImage(img, 0, 0);
                }
                resolve(mergedCanvas);
            };
            img.src = editLayerDataURL;
        });
    }

    /**
     * 获取编辑层是否有内容
     */
    hasContent() {
        return this.fabricCanvas && this.fabricCanvas.getObjects().length > 0;
    }

    /**
     * 销毁编辑层
     */
    destroy() {
        if (this.fabricCanvas) {
            this.fabricCanvas.dispose();
            this.fabricCanvas = null;
        }
        
        if (this.canvas && this.canvas.parentNode) {
            this.canvas.parentNode.removeChild(this.canvas);
        }
        
        this.canvas = null;
        this.historyStack = [];
        this.historyStep = -1;
    }

    /**
     * 获取Fabric Canvas实例（供工具使用）
     */
    getFabricCanvas() {
        return this.fabricCanvas;
    }

    /**
     * 获取原生Canvas（兼容性）
     */
    getCanvas() {
        return this.canvas;
    }

    /**
     * 获取原生Context（兼容性）
     */
    getContext() {
        return this.canvas?.getContext('2d');
    }

    /**
     * 导出为JSON
     */
    toJSON() {
        if (!this.fabricCanvas) return null;
        return this.fabricCanvas.toJSON();
    }

    /**
     * 从JSON加载
     */
    loadFromJSON(json, callback) {
        if (!this.fabricCanvas) return;
        this.fabricCanvas.loadFromJSON(json, () => {
            this.fabricCanvas.renderAll();
            if (callback) callback();
        });
    }
}
