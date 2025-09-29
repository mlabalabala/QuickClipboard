/**
 * Fabric.js画笔工具
 */
import { getCanvas, applyOpacity, getToolParams } from './common-utils.js';

export class FabricBrushTool {
    constructor() {
        this.name = 'brush';
        this.fabricCanvas = null;
        
        // 统一参数结构
        this.options = {
            color: '#ff0000',
            width: 3,
            shadowColor: '',
            shadowBlur: 0
        };
    }

    /**
     * 设置Fabric Canvas引用
     */
    setFabricCanvas(fabricCanvas) {
        this.fabricCanvas = fabricCanvas;
    }

    /**
     * 设置画笔参数
     */
    setOptions(options) {
        Object.assign(this.options, options);
        this.applyBrushOptions();
    }

    /**
     * 获取当前参数
     */
    getOptions() {
        return { ...this.options };
    }

    /**
     * 应用参数变化
     */
    applyParameter(paramName, value) {
        switch (paramName) {
            case 'color':
                this.options.color = value;
                break;
            case 'opacity':
                this.options.color = applyOpacity(this.options.color, value);
                break;
            case 'brushSize':
                this.options.width = value;
                break;
            case 'brushType':
                this.setBrushType(value);
                break;
        }
        
        this.applyBrushOptions();
        this.applyToActivePath();
    }


    /**
     * 设置笔刷类型
     */
    setBrushType(type) {
        switch (type) {
            case 'pencil':
                this.options.strokeLineCap = 'square';
                break;
            case 'marker':
                this.options.shadowBlur = 2;
                break;
            default:
                this.options.strokeLineCap = 'round';
        }
    }

    applyBrushOptions() {
        const canvas = getCanvas(this);
        if (!canvas?.freeDrawingBrush) return;
        
        const brush = canvas.freeDrawingBrush;
        brush.color = this.options.color;
        brush.width = this.options.width;
        if (this.options.shadowBlur) brush.shadowBlur = this.options.shadowBlur;
    }

    /**
     * 应用参数到选中的路径对象
     */
    applyToActivePath() {
        const canvas = getCanvas(this);
        const activeObject = canvas?.getActiveObject();
        
        if (activeObject?.type === 'path') {
            activeObject.set({
                stroke: this.options.color,
                strokeWidth: this.options.width
            });
            canvas.renderAll();
        }
    }

    /**
     * 工具激活时的处理
     */
    onActivate(editLayerManager) {
        if (!editLayerManager || !editLayerManager.getFabricCanvas) {
            console.error('画笔工具激活失败：editLayerManager 无效');
            return;
        }
        
        this.fabricCanvas = editLayerManager.getFabricCanvas();
        
        if (!this.fabricCanvas) {
            console.error('画笔工具激活失败：fabricCanvas 为空');
            return;
        }
        
        // 从子工具栏获取当前参数值
        this.syncParametersFromSubToolbar();
        
        // 启用绘画模式
        editLayerManager.enableDrawingMode(this.brushOptions);
        
        // 设置光标
        document.body.style.cursor = 'crosshair';
        
        // 应用画笔选项
        this.applyBrushOptions();
    }

    /**
     * 从子工具栏同步参数值
     */
    syncParametersFromSubToolbar() {
        const params = getToolParams('brush');
        for (const [name, value] of Object.entries(params)) {
            this.applyParameter(name, value);
        }
    }

    /**
     * 工具取消激活时的处理
     */
    onDeactivate(editLayerManager) {
        if (editLayerManager && editLayerManager.disableDrawingMode) {
            // 禁用绘画模式
            editLayerManager.disableDrawingMode();
        }
        
        // 恢复默认光标
        document.body.style.cursor = 'default';
        
        this.fabricCanvas = null;
    }

    /**
     * 设置画笔颜色
     */
    setColor(color) {
        this.setOptions({ color });
    }

    /**
     * 设置画笔宽度
     */
    setWidth(width) {
        this.setOptions({ width });
    }

    getColor() {
        return this.options.color;
    }

    getWidth() {
        return this.options.width;
    }
}
