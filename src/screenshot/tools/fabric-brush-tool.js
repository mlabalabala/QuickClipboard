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
     * 应用参数变化（来自子工具栏）
     */
    applyParameter(paramName, value) {
        switch (paramName) {
            case 'color':
                this.brushOptions.color = value;
                break;
            case 'opacity':
                // 透明度需要转换为0-1的值，并作为颜色的alpha通道
                this.applyOpacity(value);
                break;
            case 'brushSize':
                this.brushOptions.width = value;
                break;
            case 'brushType':
                // 根据笔刷类型调整设置
                this.setBrushType(value);
                break;
        }
        
        // 应用设置到画布
        this.applyBrushOptions();
    }

    /**
     * 应用透明度设置
     */
    applyOpacity(opacityPercent) {
        const opacity = opacityPercent / 100;
        
        // 如果颜色是十六进制格式，转换为rgba格式
        let color = this.brushOptions.color;
        if (color.startsWith('#')) {
            const hex = color.slice(1);
            const r = parseInt(hex.slice(0, 2), 16);
            const g = parseInt(hex.slice(2, 4), 16);
            const b = parseInt(hex.slice(4, 6), 16);
            color = `rgba(${r}, ${g}, ${b}, ${opacity})`;
        } else if (color.startsWith('rgb(')) {
            // rgb格式转换为rgba
            color = color.replace('rgb(', 'rgba(').replace(')', `, ${opacity})`);
        } else if (color.startsWith('rgba(')) {
            // 替换现有的alpha值
            color = color.replace(/,\s*[\d.]+\s*\)$/, `, ${opacity})`);
        }
        
        this.brushOptions.color = color;
    }

    /**
     * 设置笔刷类型
     */
    setBrushType(type) {
        switch (type) {
            case 'pencil':
                // 铅笔：清晰、硬边
                this.brushOptions.shadowBlur = 0;
                this.brushOptions.strokeLineCap = 'square';
                break;
            case 'brush':
                // 画笔：柔和、圆边
                this.brushOptions.shadowBlur = 0;
                this.brushOptions.strokeLineCap = 'round';
                break;
            case 'marker':
                // 马克笔：半透明、柔和边缘
                this.brushOptions.shadowBlur = 2;
                this.brushOptions.strokeLineCap = 'round';
                break;
        }
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
        if (window.screenshotController && window.screenshotController.subToolbarManager) {
            const subToolbar = window.screenshotController.subToolbarManager;
            const toolParams = subToolbar.getToolParameters('brush');
            
            // 应用所有参数
            for (const [paramName, value] of Object.entries(toolParams)) {
                this.applyParameter(paramName, value);
            }
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
     * 清除当前绘制状态
     */
    clear() {
        // Fabric.js自动管理绘制状态，无需手动清理
    }

    /**
     * 开始绘制
     */
    startDrawing(ctx, x, y) {
        // 由Fabric.js的绘画模式自动处理
    }

    /**
     * 继续绘制
     */
    draw(ctx, x, y) {
        // 由Fabric.js的绘画模式自动处理
    }

    /**
     * 结束绘制
     */
    endDrawing() {
        // 由Fabric.js的绘画模式自动处理
    }
}
