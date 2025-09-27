/**
 * 工具管理器
 * 负责管理和切换各种截屏编辑工具
 */

import { BrushTool } from '../tools/brush-tool.js';

export class ToolManager {
    constructor() {
        this.tools = new Map();
        this.currentTool = null;
        this.editLayerManager = null;
        this.isToolActive = false;
        
        // 初始化工具
        this.initTools();
        
        // 绑定事件处理器
        this.handleMouseDown = this.handleMouseDown.bind(this);
        this.handleMouseMove = this.handleMouseMove.bind(this);
        this.handleMouseUp = this.handleMouseUp.bind(this);
    }

    /**
     * 初始化所有工具
     */
    initTools() {
        // 注册画笔工具
        this.registerTool(new BrushTool());
        
        // 未来可以注册更多工具：
        // this.registerTool(new TextTool());
        // this.registerTool(new ArrowTool());
        // this.registerTool(new RectangleTool());
    }

    /**
     * 注册工具
     */
    registerTool(tool) {
        this.tools.set(tool.name, tool);
    }

    /**
     * 设置编辑层管理器引用
     */
    setEditLayerManager(editLayerManager) {
        this.editLayerManager = editLayerManager;
    }

    /**
     * 激活工具
     */
    activateTool(toolName) {
        // 取消当前工具
        if (this.currentTool) {
            this.deactivateTool();
        }

        const tool = this.tools.get(toolName);
        if (!tool) {
            console.error(`工具 "${toolName}" 不存在`);
            return false;
        }

        this.currentTool = tool;
        this.isToolActive = true;

        // 禁用选区和遮罩层的鼠标事件
        this.disableSelectionEvents();

        // 启用编辑层交互
        if (this.editLayerManager) {
            this.editLayerManager.enableInteraction();
            this.addEventListeners();
        }

        // 调用工具的激活回调
        if (tool.onActivate) {
            tool.onActivate();
        }

        return true;
    }

    /**
     * 取消激活当前工具
     */
    deactivateTool() {
        if (!this.currentTool) return;

        // 调用工具的取消激活回调
        if (this.currentTool.onDeactivate) {
            this.currentTool.onDeactivate();
        }

        // 恢复选区和遮罩层的鼠标事件
        this.enableSelectionEvents();

        // 禁用编辑层交互
        if (this.editLayerManager) {
            this.editLayerManager.disableInteraction();
            this.removeEventListeners();
        }

        this.currentTool = null;
        this.isToolActive = false;

    }

    /**
     * 获取当前激活的工具
     */
    getCurrentTool() {
        return this.currentTool;
    }

    /**
     * 检查工具是否激活
     */
    isActive() {
        return this.isToolActive;
    }

    /**
     * 获取所有可用工具列表
     */
    getAvailableTools() {
        return Array.from(this.tools.keys());
    }

    /**
     * 添加事件监听器
     */
    addEventListeners() {
        if (!this.editLayerManager?.canvas) return;

        const canvas = this.editLayerManager.canvas;
        canvas.addEventListener('mousedown', this.handleMouseDown);
        canvas.addEventListener('mousemove', this.handleMouseMove);
        canvas.addEventListener('mouseup', this.handleMouseUp);
        
        // 防止右键菜单
        canvas.addEventListener('contextmenu', (e) => e.preventDefault());
    }

    /**
     * 移除事件监听器
     */
    removeEventListeners() {
        if (!this.editLayerManager?.canvas) return;

        const canvas = this.editLayerManager.canvas;
        canvas.removeEventListener('mousedown', this.handleMouseDown);
        canvas.removeEventListener('mousemove', this.handleMouseMove);
        canvas.removeEventListener('mouseup', this.handleMouseUp);
        canvas.removeEventListener('contextmenu', (e) => e.preventDefault());
    }

    /**
     * 鼠标按下事件处理
     */
    handleMouseDown(e) {
        if (!this.currentTool || e.button !== 0) return; // 只处理左键

        e.preventDefault();
        e.stopPropagation();

        // 转换坐标
        const coords = this.editLayerManager.screenToCanvasCoords(e.clientX, e.clientY);
        const ctx = this.editLayerManager.getContext();

        // 将在绘制完成后保存状态

        // 调用工具的开始绘制方法
        if (this.currentTool.startDrawing) {
            this.currentTool.startDrawing(ctx, coords.x, coords.y, e);
        }
    }

    /**
     * 鼠标移动事件处理
     */
    handleMouseMove(e) {
        if (!this.currentTool) return;

        e.preventDefault();

        // 转换坐标
        const coords = this.editLayerManager.screenToCanvasCoords(e.clientX, e.clientY);
        const ctx = this.editLayerManager.getContext();

        // 调用工具的绘制方法
        if (this.currentTool.draw) {
            this.currentTool.draw(ctx, coords.x, coords.y, e);
        }
    }

    /**
     * 鼠标抬起事件处理
     */
    handleMouseUp(e) {
        if (!this.currentTool || e.button !== 0) return;

        e.preventDefault();

        // 转换坐标
        const coords = this.editLayerManager.screenToCanvasCoords(e.clientX, e.clientY);
        const ctx = this.editLayerManager.getContext();

        // 调用工具的结束绘制方法
        if (this.currentTool.endDrawing) {
            this.currentTool.endDrawing(ctx, coords.x, coords.y, e);
        }

        // 绘制完成后保存状态到历史管理器
        this.editLayerManager.saveState(`使用${this.currentTool.name}工具绘制`);
    }


    /**
     * 清除所有编辑内容
     */
    clear() {
        if (this.editLayerManager) {
            this.editLayerManager.clear();
            this.editLayerManager.saveState(); // 保存清除后的状态
        }
    }

    /**
     * 获取工具选项（如果当前工具支持）
     */
    getToolOptions() {
        if (this.currentTool && this.currentTool.getOptions) {
            return this.currentTool.getOptions();
        }
        return null;
    }

    /**
     * 设置工具选项（如果当前工具支持）
     */
    setToolOptions(options) {
        if (this.currentTool && this.currentTool.setOptions) {
            this.currentTool.setOptions(options);
            return true;
        }
        return false;
    }

    /**
     * 禁用选区和遮罩层的鼠标事件
     */
    disableSelectionEvents() {
        const overlay = document.getElementById('overlay');
        const selectionArea = document.getElementById('selectionArea');
        const maskLayers = document.getElementById('maskLayers');
        const resizeHandles = document.querySelectorAll('.resize-handle');
        
        if (overlay) {
            overlay.style.pointerEvents = 'none';
        }
        if (selectionArea) {
            selectionArea.style.pointerEvents = 'none';
        }
        if (maskLayers) {
            maskLayers.style.pointerEvents = 'none';
        }
        // 禁用调整大小节点
        resizeHandles.forEach(handle => {
            handle.style.pointerEvents = 'none';
        });
    }

    /**
     * 恢复选区和遮罩层的鼠标事件
     */
    enableSelectionEvents() {
        const overlay = document.getElementById('overlay');
        const selectionArea = document.getElementById('selectionArea');
        const maskLayers = document.getElementById('maskLayers');
        const resizeHandles = document.querySelectorAll('.resize-handle');
        
        if (overlay) {
            overlay.style.pointerEvents = 'auto';
        }
        if (selectionArea) {
            selectionArea.style.pointerEvents = 'auto';
        }
        if (maskLayers) {
            maskLayers.style.pointerEvents = 'auto';
        }
        // 恢复调整大小节点
        resizeHandles.forEach(handle => {
            handle.style.pointerEvents = 'auto';
        });
    }
}
