/**
 * Fabric.js编辑层管理器
 */

import { Arrow } from '../tools/fabric-simple-arrow-tool.js';

export class FabricEditLayerManager {
    constructor() {
        this.canvas = null;
        this.fabricCanvas = null;
        this.backgroundManager = null;
        
        // 历史管理
        this.historyStack = [];
        this.historyStep = -1;
        this.maxHistorySize = 50;
        this.historyDebounceDelay = 180;
        this.historyDebounceTimer = null;
        this.pendingHistoryDescription = null;
        this.historyEventHandlers = {};
        this.isLoadingFromHistory = false;
        
        // 回调函数
        this.onHistoryChange = null;
        this.onObjectModified = null;
        this.onActiveObjectChange = null;
        
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
        this.setupHistoryTracking();
        this.initializeHistoryState('初始状态');
    }

    /**
     * 初始化Fabric事件
     */
    initFabricEvents() {
        const notifyChange = (e) => this.isLoadingFromHistory ? null : this.onObjectModified?.(e);
        this.fabricCanvas.on('object:modified', notifyChange);
        this.fabricCanvas.on('object:moving', notifyChange);

        // 活动对象变化事件
        const emitActiveObject = () => this.emitActiveObjectChange();
        this.fabricCanvas.on('selection:created', emitActiveObject);
        this.fabricCanvas.on('selection:updated', emitActiveObject);
        this.fabricCanvas.on('selection:cleared', emitActiveObject);
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
     * 设置活动对象变化回调
     */
    setOnActiveObjectChange(callback) {
        this.onActiveObjectChange = callback;
    }

    /**
     * 发出活动对象变化事件
     */
    emitActiveObjectChange() {
        if (!this.fabricCanvas || this.isLoadingFromHistory) return;
        
        const active = this.fabricCanvas.getActiveObject();
        const objects = active ? (active.type === 'activeSelection' ? active._objects || [] : [active]) : [];
        const objectType = this.getObjectType(active);
        
        this.onActiveObjectChange?.({
            activeObject: active,
            objects,
            type: objectType
        });
    }

    /**
     * 获取对象类型
     */
    getObjectType(obj) {
        if (!obj) return null;
        
        // 检查自定义类型
        if (obj.customType) {
            // 特殊处理形状工具的箭头
            if (obj.customType === 'shape-arrow') {
                return 'shape-arrow';
            }
            return obj.customType;
        }
        
        // 检查Fabric内置类型
        switch (obj.type) {
            case 'text':
            case 'i-text':
            case 'textbox':
                return 'text';
            case 'path':
                // 画笔绘制的路径
                return 'brush';
            case 'rect':
                return 'rectangle';
            case 'circle':
                return 'circle';
            case 'arrow':
                return 'arrow';
            case 'activeSelection':
                // 多选时，返回第一个对象的类型
                if (obj._objects && obj._objects.length > 0) {
                    return this.getObjectType(obj._objects[0]);
                }
                return 'selection';
            default:
                return obj.type || 'unknown';
        }
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

        const brush = this.ensureDrawingBrush();

        this.isDrawingMode = true;
        this.fabricCanvas.isDrawingMode = true;

        if (brush) {
            brush.color = brushOptions.color || '#ff0000';
            brush.width = brushOptions.width || 3;

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

    ensureDrawingBrush() {
        if (!this.fabricCanvas) return null;
        if (!this.fabricCanvas.freeDrawingBrush) {
            this.fabricCanvas.freeDrawingBrush = new fabric.PencilBrush(this.fabricCanvas);
            this.fabricCanvas.freeDrawingBrush.color = '#ff0000';
            this.fabricCanvas.freeDrawingBrush.width = 3;
            this.fabricCanvas.freeDrawingBrush.lineCap = 'round';
            this.fabricCanvas.freeDrawingBrush.lineJoin = 'round';
        }
        return this.fabricCanvas.freeDrawingBrush;
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

    setSelectionEnabled(enabled) {
        if (!this.fabricCanvas) return;
        this.fabricCanvas.selection = enabled;
        this.fabricCanvas.forEachObject((obj) => {
            obj.selectable = enabled;
            obj.evented = enabled;
        });
        if (!enabled) {
            this.fabricCanvas.discardActiveObject();
            this.fabricCanvas.renderAll();
        }
    }

    prepareSelectionForTool(toolName) {
        if (!this.fabricCanvas) return;
        if (toolName === 'selection') {
            this.fabricCanvas.selection = true;
            this.fabricCanvas.forEachObject((obj) => {
                obj.selectable = true;
                obj.evented = true;
            });
        } else {
            this.fabricCanvas.discardActiveObject();
            this.fabricCanvas.selection = false;
            this.fabricCanvas.forEachObject((obj) => {
                obj.selectable = false;
                obj.evented = false;
            });
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
     * 初始化历史事件跟踪
     */
    setupHistoryTracking() {
        if (!this.fabricCanvas) return;

        this.cleanupHistoryTracking();

        const register = (eventName, handler) => {
            const boundHandler = handler.bind(this);
            this.historyEventHandlers[eventName] = boundHandler;
            this.fabricCanvas.on(eventName, boundHandler);
        };

        register('object:added', this.onCanvasObjectAdded);
        register('object:removed', this.onCanvasObjectRemoved);
        register('object:modified', this.onCanvasObjectModified);
        register('object:skewing', this.onCanvasObjectModifying);
        register('object:scaling', this.onCanvasObjectModifying);
        register('object:rotating', this.onCanvasObjectModifying);
        register('object:moving', this.onCanvasObjectModifying);
        register('path:created', this.onCanvasPathCreated);
    }

    /**
     * 清理历史事件跟踪
     */
    cleanupHistoryTracking() {
        if (!this.fabricCanvas) {
            this.historyEventHandlers = {};
            return;
        }

        Object.entries(this.historyEventHandlers).forEach(([eventName, handler]) => {
            this.fabricCanvas.off(eventName, handler);
        });
        this.historyEventHandlers = {};

        if (this.historyDebounceTimer) {
            clearTimeout(this.historyDebounceTimer);
            this.historyDebounceTimer = null;
        }
        this.pendingHistoryDescription = null;
    }

    /**
     * 请求保存历史快照
     */
    requestHistorySave(description = '', options = {}) {
        if (!this.fabricCanvas || this.isLoadingFromHistory) return;

        const {
            immediate = false,
            debounceDelay = this.historyDebounceDelay
        } = options;

        if (immediate) {
            this.flushHistorySave(description);
            return;
        }

        if (description) {
            this.pendingHistoryDescription = description;
        }

        if (this.historyDebounceTimer) {
            clearTimeout(this.historyDebounceTimer);
        }

        this.historyDebounceTimer = setTimeout(() => {
            const finalDescription = this.pendingHistoryDescription || description;
            this.flushHistorySave(finalDescription);
        }, debounceDelay);
    }

    /**
     * 立即保存历史快照
     */
    flushHistorySave(description = '') {
        if (!this.fabricCanvas || this.isLoadingFromHistory) return;

        if (this.historyDebounceTimer) {
            clearTimeout(this.historyDebounceTimer);
            this.historyDebounceTimer = null;
        }

        const finalDescription = description || this.pendingHistoryDescription || '';
        this.pendingHistoryDescription = null;
        this.saveState(finalDescription);
    }

    /**
     * 如果存在挂起的历史请求，则立即保存
     */
    flushPendingHistory(description = '自动保存') {
        if (this.historyDebounceTimer) {
            this.flushHistorySave(description);
        }
    }

    onCanvasObjectAdded(event) {
        if (this.isLoadingFromHistory) return;

        const target = event?.target;
        if (!target || target.excludeFromHistory) return;

        const reason = target.historyAddReason || '添加对象';
        this.requestHistorySave(reason, { immediate: true });
        delete target.historyAddReason;
    }

    onCanvasObjectRemoved(event) {
        if (this.isLoadingFromHistory) return;

        const target = event?.target;
        if (!target || target.excludeFromHistory) return;

        const reason = target.historyRemoveReason || '删除对象';
        this.requestHistorySave(reason, { immediate: true });
        delete target.historyRemoveReason;
    }

    onCanvasObjectModified(event) {
        if (this.isLoadingFromHistory) return;

        const target = event?.target;
        if (!target || target.excludeFromHistory) return;

        const reason = target.historyModifyReason || '对象修改';
        this.requestHistorySave(reason, { debounceDelay: this.historyDebounceDelay });
        delete target.historyModifyReason;

        this.onObjectModified?.(event);
    }

    onCanvasObjectModifying(event) {
        if (this.isLoadingFromHistory) return;

        const target = event?.target;
        if (!target || target.excludeFromHistory) return;

        target.historyModifyReason = target.historyModifyReason || '对象修改';
    }

    onCanvasPathCreated(event) {
        if (this.isLoadingFromHistory) return;

        const path = event?.path;
        if (path) {
            path.excludeFromHistory = false;
            path.historyAddReason = path.historyAddReason || '绘制路径';
        }

        this.requestHistorySave('绘制路径', { immediate: true });
    }

    /**
     * 保存当前状态到历史记录
     */
    saveState(description = '') {
        if (!this.fabricCanvas || this.isLoadingFromHistory) return;

        try {
            if (this.historyStep < this.historyStack.length - 1) {
                this.historyStack.splice(this.historyStep + 1);
            }

            const state = JSON.stringify(this.fabricCanvas.toJSON());
            const lastEntry = this.historyStack[this.historyStack.length - 1];
            if (lastEntry && lastEntry.state === state) {
                return;
            }

            this.historyStack.push({ state, description, timestamp: Date.now() });
            this.historyStep = this.historyStack.length - 1;

            if (this.historyStack.length > this.maxHistorySize) {
                this.historyStack.shift();
                this.historyStep--;
            }

            this.triggerHistoryChange();

        } catch (error) {
            console.error('FabricEditLayerManager: Error saving state:', error);
        }
    }

    /**
     * 撤销操作
     */
    async undo() {
        this.flushPendingHistory();
        if (!this.canUndo()) return false;

        try {
            this.historyStep--;
            this.triggerHistoryChange();
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
        this.flushPendingHistory();
        if (!this.canRedo()) return false;

        try {
            this.historyStep++;
            this.triggerHistoryChange();
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
            this.fabricCanvas.clear();
            let callbackExecuted = false;
            const stateData = typeof stateJson === 'string' ? JSON.parse(stateJson) : stateJson;

            const handleLoad = () => {
                if (callbackExecuted) return;
                callbackExecuted = true;
                this.updateCanvasSize();
                this.fabricCanvas.renderAll();
                this.fabricCanvas.requestRenderAll();
                setTimeout(() => {
                    this.isLoadingFromHistory = false;
                    this.triggerHistoryChange();
                    resolve();
                }, 10);
            };

            const handleError = (error) => {
                const message = String(error);
                if (message.includes('No class registered for arrow') || message.includes('fromObject')) {
                    registerArrowClass();
                    try {
                        this.fabricCanvas.loadFromJSON(stateData, handleLoad, reject);
                        return;
                    } catch (retryError) {
                        reject(retryError);
                        return;
                    }
                }
                this.isLoadingFromHistory = false;
                reject(error);
            };

            try {
                this.fabricCanvas.loadFromJSON(stateData, handleLoad, handleError);
            } catch (error) {
                handleError(error);
            }
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
        this.flushPendingHistory();
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
        this.cleanupHistoryTracking();

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
        // 如果还没有初始化，先初始化
        if (!this.fabricCanvas) {
            console.warn('FabricCanvas 还未初始化，正在自动初始化...');
            this.init();
        }
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
            this.updateCanvasSize();
            this.fabricCanvas.renderAll();
            this.triggerHistoryChange();
            if (callback) callback();
        });
    }

    initializeHistoryState(description = '初始状态') {
        if (!this.fabricCanvas) return;
        this.flushPendingHistory();
        this.historyStack = [];
        this.historyStep = -1;
        const state = JSON.stringify(this.fabricCanvas.toJSON());
        this.historyStack.push({ state, description, timestamp: Date.now() });
        this.historyStep = 0;
        this.triggerHistoryChange();
    }
}
