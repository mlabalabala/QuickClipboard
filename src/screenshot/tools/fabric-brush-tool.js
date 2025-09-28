/**
 * Fabric.js画笔工具
 * 基于Fabric.js的画笔工具实现
 */

export class FabricBrushTool {
    constructor() {
        this.name = 'brush';
        this.fabricCanvas = null;
        
        // 画笔参数
        this.brushOptions = {
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
        if (options.color !== undefined) this.brushOptions.color = options.color;
        if (options.width !== undefined) this.brushOptions.width = options.width;
        if (options.shadowColor !== undefined) this.brushOptions.shadowColor = options.shadowColor;
        if (options.shadowBlur !== undefined) this.brushOptions.shadowBlur = options.shadowBlur;
        
        // 立即应用到Fabric Canvas
        this.applyBrushOptions();
    }

    /**
     * 获取当前画笔参数
     */
    getOptions() {
        return { ...this.brushOptions };
    }

    /**
     * 应用画笔选项到Fabric Canvas
     */
    applyBrushOptions() {
        if (!this.fabricCanvas) return;
        
        const brush = this.fabricCanvas.freeDrawingBrush;
        if (!brush) return;
        
        brush.color = this.brushOptions.color;
        brush.width = this.brushOptions.width;
        
        if (this.brushOptions.shadowColor) {
            brush.shadowColor = this.brushOptions.shadowColor;
        }
        if (this.brushOptions.shadowBlur) {
            brush.shadowBlur = this.brushOptions.shadowBlur;
        }
    }

    /**
     * 工具激活时的处理
     */
    onActivate(editLayerManager) {
        if (!editLayerManager || !editLayerManager.getFabricCanvas) return;
        
        this.fabricCanvas = editLayerManager.getFabricCanvas();
        
        // 启用绘画模式
        editLayerManager.enableDrawingMode(this.brushOptions);
        
        // 设置光标
        document.body.style.cursor = 'crosshair';
        
        // 应用画笔选项
        this.applyBrushOptions();
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

    /**
     * 获取画笔颜色
     */
    getColor() {
        return this.brushOptions.color;
    }

    /**
     * 获取画笔宽度
     */
    getWidth() {
        return this.brushOptions.width;
    }

    /**
     * 清除当前绘制状态（兼容旧接口）
     */
    clear() {
        // Fabric.js自动管理绘制状态，无需手动清理
    }

    /**
     * 开始绘制（兼容旧接口 - 实际由Fabric.js处理）
     */
    startDrawing(ctx, x, y) {
        // 由Fabric.js的绘画模式自动处理
    }

    /**
     * 继续绘制（兼容旧接口 - 实际由Fabric.js处理）
     */
    draw(ctx, x, y) {
        // 由Fabric.js的绘画模式自动处理
    }

    /**
     * 结束绘制（兼容旧接口 - 实际由Fabric.js处理）
     */
    endDrawing() {
        // 由Fabric.js的绘画模式自动处理
    }
}
