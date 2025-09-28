/**
 * Fabric.js形状工具
 * 基于Fabric.js的形状工具实现（矩形、圆形、箭头等）
 */

export class FabricShapeTool {
    constructor(shapeType = 'rectangle') {
        this.name = shapeType;
        this.shapeType = shapeType; // 'rectangle', 'circle', 'arrow'
        this.fabricCanvas = null;
        this.editLayerManager = null;
        this.isActive = false;
        this.isDrawing = false;
        this.startPoint = null;
        this.currentShape = null;
        
        // 形状参数（与子工具栏默认值保持一致）
        this.shapeOptions = {
            fill: 'transparent', // 默认无填充，与子工具栏的filled: false一致
            stroke: '#ff0000',
            strokeWidth: 2,
            strokeDashArray: null
        };
        
        // 保存的填充颜色值（即使填充关闭也保存）
        this.fillColorValue = '#ff0000';
        
        // 绑定事件处理器
        this.handleMouseDown = this.handleMouseDown.bind(this);
        this.handleMouseMove = this.handleMouseMove.bind(this);
        this.handleMouseUp = this.handleMouseUp.bind(this);
    }

    /**
     * 设置形状参数
     */
    setOptions(options) {
        if (options.fill !== undefined) this.shapeOptions.fill = options.fill;
        if (options.stroke !== undefined) this.shapeOptions.stroke = options.stroke;
        if (options.strokeWidth !== undefined) this.shapeOptions.strokeWidth = options.strokeWidth;
        if (options.strokeDashArray !== undefined) this.shapeOptions.strokeDashArray = options.strokeDashArray;
    }

    /**
     * 获取当前形状参数
     */
    getOptions() {
        return { ...this.shapeOptions };
    }

    /**
     * 应用参数变化（来自子工具栏）
     */
    applyParameter(paramName, value) {
        switch (paramName) {
            case 'color':
                // 颜色应用到边框
                this.shapeOptions.stroke = value;
                // 如果当前启用了填充，同时更新填充颜色
                if (this.isFillEnabled()) {
                    this.shapeOptions.fill = this.convertColorToFill(value);
                }
                break;
            case 'opacity':
                // 透明度应用到填充和边框
                this.applyOpacity(value);
                break;
            case 'strokeWidth':
                this.shapeOptions.strokeWidth = value;
                break;
            case 'filled':
                // 填充开关
                if (value) {
                    // 启用填充，使用保存的填充颜色值
                    this.shapeOptions.fill = this.fillColorValue || this.convertColorToFill(this.shapeOptions.stroke);
                } else {
                    // 禁用填充
                    this.shapeOptions.fill = 'transparent';
                }
                break;
            case 'fillColor':
                // 填充颜色 - 总是保存颜色值，但只在启用填充时应用
                this.fillColorValue = value; // 保存填充颜色值
                if (this.isFillEnabled()) {
                    this.shapeOptions.fill = value;
                }
                break;
        }
        
        // 如果有活动的形状对象，立即应用更改
        this.applyToActiveShape();
    }

    /**
     * 检查填充是否启用
     */
    isFillEnabled() {
        return this.shapeOptions.fill && this.shapeOptions.fill !== 'transparent';
    }

    /**
     * 将颜色转换为填充颜色（带透明度）
     */
    convertColorToFill(color, alpha = 0.3) {
        if (color.startsWith('#')) {
            // 十六进制颜色转换
            const hex = color.slice(1);
            const r = parseInt(hex.slice(0, 2), 16);
            const g = parseInt(hex.slice(2, 4), 16);
            const b = parseInt(hex.slice(4, 6), 16);
            return `rgba(${r}, ${g}, ${b}, ${alpha})`;
        } else if (color.startsWith('rgb(')) {
            // RGB颜色转换为RGBA
            return color.replace('rgb(', 'rgba(').replace(')', `, ${alpha})`);
        } else if (color.startsWith('rgba(')) {
            // 已经是RGBA，替换透明度
            return color.replace(/,\s*[\d.]+\s*\)$/, `, ${alpha})`);
        }
        
        // 默认返回半透明版本
        return color;
    }

    /**
     * 应用透明度设置
     */
    applyOpacity(opacityPercent) {
        const opacity = opacityPercent / 100;
        
        // 应用到边框颜色
        let strokeColor = this.shapeOptions.stroke;
        if (strokeColor.startsWith('#')) {
            const hex = strokeColor.slice(1);
            const r = parseInt(hex.slice(0, 2), 16);
            const g = parseInt(hex.slice(2, 4), 16);
            const b = parseInt(hex.slice(4, 6), 16);
            strokeColor = `rgba(${r}, ${g}, ${b}, ${opacity})`;
        } else if (strokeColor.startsWith('rgb(')) {
            strokeColor = strokeColor.replace('rgb(', 'rgba(').replace(')', `, ${opacity})`);
        } else if (strokeColor.startsWith('rgba(')) {
            strokeColor = strokeColor.replace(/,\s*[\d.]+\s*\)$/, `, ${opacity})`);
        }
        this.shapeOptions.stroke = strokeColor;
        
        // 应用到填充颜色（如果不是透明的）
        if (this.shapeOptions.fill !== 'transparent') {
            let fillColor = this.shapeOptions.fill;
            const fillOpacity = opacity * 0.3; // 填充透明度稍微低一些
            
            if (fillColor.startsWith('#')) {
                const hex = fillColor.slice(1);
                const r = parseInt(hex.slice(0, 2), 16);
                const g = parseInt(hex.slice(2, 4), 16);
                const b = parseInt(hex.slice(4, 6), 16);
                fillColor = `rgba(${r}, ${g}, ${b}, ${fillOpacity})`;
            } else if (fillColor.startsWith('rgb(')) {
                fillColor = fillColor.replace('rgb(', 'rgba(').replace(')', `, ${fillOpacity})`);
            } else if (fillColor.startsWith('rgba(')) {
                fillColor = fillColor.replace(/,\s*[\d.]+\s*\)$/, `, ${fillOpacity})`);
            }
            this.shapeOptions.fill = fillColor;
        }
    }

    /**
     * 将当前参数应用到活动的形状对象
     */
    applyToActiveShape() {
        if (!this.fabricCanvas) return;
        
        const activeObject = this.fabricCanvas.getActiveObject();
        if (activeObject && (activeObject.type === 'rect' || activeObject.type === 'circle' || activeObject.type === 'path')) {
            activeObject.set({
                fill: this.shapeOptions.fill,
                stroke: this.shapeOptions.stroke,
                strokeWidth: this.shapeOptions.strokeWidth,
                strokeDashArray: this.shapeOptions.strokeDashArray
            });
            this.fabricCanvas.renderAll();
        }
    }

    /**
     * 工具激活时的处理
     */
    onActivate(editLayerManager) {
        if (!editLayerManager || !editLayerManager.getFabricCanvas) return;
        
        this.editLayerManager = editLayerManager;
        this.fabricCanvas = editLayerManager.getFabricCanvas();
        this.isActive = true;
        
        // 确保不在绘画模式，禁用选择功能专注于创建
        this.fabricCanvas.isDrawingMode = false;
        this.fabricCanvas.selection = false;
        this.fabricCanvas.forEachObject((obj) => {
            obj.selectable = false;
        });
        
        // 设置光标
        document.body.style.cursor = 'crosshair';
        
        // 从子工具栏获取当前参数值
        this.syncParametersFromSubToolbar();
        
        // 添加事件监听器
        if (this.fabricCanvas) {
            this.fabricCanvas.on('mouse:down', this.handleMouseDown);
            this.fabricCanvas.on('mouse:move', this.handleMouseMove);
            this.fabricCanvas.on('mouse:up', this.handleMouseUp);
        }
    }

    /**
     * 从子工具栏同步参数值
     */
    syncParametersFromSubToolbar() {
        if (window.screenshotController && window.screenshotController.subToolbarManager) {
            const subToolbar = window.screenshotController.subToolbarManager;
            // 所有形状工具都使用 'shape' 参数配置
            const toolParams = subToolbar.getToolParameters('shape');
            
            // 先同步填充颜色值（即使填充未启用）
            if (toolParams.fillColor) {
                this.fillColorValue = toolParams.fillColor;
            }
            
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
        this.isActive = false;
        this.isDrawing = false;
        
        // 恢复默认光标
        document.body.style.cursor = 'default';
        
        // 移除事件监听器
        if (this.fabricCanvas) {
            this.fabricCanvas.off('mouse:down', this.handleMouseDown);
            this.fabricCanvas.off('mouse:move', this.handleMouseMove);
            this.fabricCanvas.off('mouse:up', this.handleMouseUp);
        }
        
        this.fabricCanvas = null;
        this.editLayerManager = null;
        this.currentShape = null;
        this.startPoint = null;
    }

    /**
     * 切换到选择工具并选中指定对象
     */
    switchToSelectionTool(objectToSelect) {
        // 通过全局事件或工具管理器切换工具
        if (window.screenshotController && window.screenshotController.toolManager) {
            window.screenshotController.toolManager.switchToSelectionTool(objectToSelect);
        }
    }

    /**
     * 处理鼠标按下事件
     */
    handleMouseDown(e) {
        if (!this.isActive || !this.fabricCanvas) return;
        
        const pointer = this.fabricCanvas.getPointer(e.e);
        this.startPoint = { x: pointer.x, y: pointer.y };
        this.isDrawing = true;
        
        // 创建初始形状
        this.currentShape = this.createShape(pointer.x, pointer.y, 1, 1);
        if (this.currentShape) {
            // 新形状不可选择，避免创建过程中出现选择框
            this.currentShape.selectable = false;
            this.fabricCanvas.add(this.currentShape);
            this.fabricCanvas.renderAll();
        }
    }

    /**
     * 处理鼠标移动事件
     */
    handleMouseMove(e) {
        if (!this.isDrawing || !this.currentShape || !this.startPoint) return;
        
        const pointer = this.fabricCanvas.getPointer(e.e);
        this.updateShape(this.currentShape, this.startPoint, pointer);
        this.fabricCanvas.renderAll();
    }

    /**
     * 处理鼠标抬起事件
     */
    handleMouseUp(e) {
        if (!this.isDrawing) return;
        
        this.isDrawing = false;
        
        // 如果形状太小，删除它
        if (this.currentShape) {
            const minSize = 5;
            let shouldRemove = false;
            
            if (this.shapeType === 'rectangle') {
                shouldRemove = this.currentShape.width < minSize || this.currentShape.height < minSize;
            } else if (this.shapeType === 'circle') {
                shouldRemove = this.currentShape.radius < minSize / 2;
            }
            
            if (shouldRemove) {
                this.fabricCanvas.remove(this.currentShape);
            } else {
                // 形状创建成功，使其可选择
                this.currentShape.selectable = true;
                
                // 延迟保存历史状态，避免与Fabric事件冲突
                setTimeout(() => {
                    if (this.editLayerManager && this.editLayerManager.saveState) {
                        this.editLayerManager.saveState(`添加${this.shapeType}`);
                    }
                }, 50);
                
                // 切换到选择工具并选中刚创建的形状
                setTimeout(() => {
                    this.switchToSelectionTool(this.currentShape);
                }, 100);
            }
        }
        
        this.currentShape = null;
        this.startPoint = null;
    }

    /**
     * 创建形状
     */
    createShape(x, y, width, height) {
        const options = {
            left: x,
            top: y,
            fill: this.shapeOptions.fill,
            stroke: this.shapeOptions.stroke,
            strokeWidth: this.shapeOptions.strokeWidth,
            strokeDashArray: this.shapeOptions.strokeDashArray,
            selectable: true,
            evented: true
        };

        switch (this.shapeType) {
            case 'rectangle':
                return new fabric.Rect({
                    ...options,
                    width: width,
                    height: height
                });
                
            case 'circle':
                const radius = Math.min(width, height) / 2;
                return new fabric.Circle({
                    ...options,
                    radius: radius,
                    left: x - radius,
                    top: y - radius
                });
                
            case 'arrow':
                return this.createArrow(x, y, width, height, options);
                
            default:
                return null;
        }
    }

    /**
     * 创建箭头
     */
    createArrow(startX, startY, endX, endY, options) {
        const arrowHeadLength = 15;
        const arrowHeadAngle = Math.PI / 6;
        
        const angle = Math.atan2(endY - startY, endX - startX);
        const length = Math.sqrt((endX - startX) ** 2 + (endY - startY) ** 2);
        
        if (length < 10) return null;
        
        // 创建箭头路径
        const pathString = [
            'M', startX, startY,
            'L', endX, endY,
            'M', endX - arrowHeadLength * Math.cos(angle - arrowHeadAngle), endY - arrowHeadLength * Math.sin(angle - arrowHeadAngle),
            'L', endX, endY,
            'L', endX - arrowHeadLength * Math.cos(angle + arrowHeadAngle), endY - arrowHeadLength * Math.sin(angle + arrowHeadAngle)
        ].join(' ');
        
        return new fabric.Path(pathString, {
            ...options,
            fill: '',
            stroke: this.shapeOptions.stroke,
            strokeWidth: this.shapeOptions.strokeWidth
        });
    }

    /**
     * 更新形状
     */
    updateShape(shape, startPoint, currentPoint) {
        if (!shape || !startPoint) return;
        
        const width = Math.abs(currentPoint.x - startPoint.x);
        const height = Math.abs(currentPoint.y - startPoint.y);
        const left = Math.min(startPoint.x, currentPoint.x);
        const top = Math.min(startPoint.y, currentPoint.y);

        switch (this.shapeType) {
            case 'rectangle':
                shape.set({
                    left: left,
                    top: top,
                    width: width,
                    height: height
                });
                break;
                
            case 'circle':
                const radius = Math.min(width, height) / 2;
                shape.set({
                    left: left,
                    top: top,
                    radius: radius
                });
                break;
                
            case 'arrow':
                // 重新创建箭头
                const newArrow = this.createArrow(startPoint.x, startPoint.y, currentPoint.x, currentPoint.y, this.shapeOptions);
                if (newArrow) {
                    this.fabricCanvas.remove(shape);
                    this.fabricCanvas.add(newArrow);
                    this.fabricCanvas.setActiveObject(newArrow);
                    this.currentShape = newArrow;
                }
                break;
        }
    }

    /**
     * 设置填充颜色
     */
    setFill(fill) {
        this.setOptions({ fill });
    }

    /**
     * 设置边框颜色
     */
    setStroke(stroke) {
        this.setOptions({ stroke });
    }

    /**
     * 设置边框宽度
     */
    setStrokeWidth(strokeWidth) {
        this.setOptions({ strokeWidth });
    }

    /**
     * 获取填充颜色
     */
    getFill() {
        return this.shapeOptions.fill;
    }

    /**
     * 获取边框颜色
     */
    getStroke() {
        return this.shapeOptions.stroke;
    }

    /**
     * 获取边框宽度
     */
    getStrokeWidth() {
        return this.shapeOptions.strokeWidth;
    }
}

// 导出具体的形状工具类
export class FabricRectangleTool extends FabricShapeTool {
    constructor() {
        super('rectangle');
    }
}

export class FabricCircleTool extends FabricShapeTool {
    constructor() {
        super('circle');
    }
}

export class FabricArrowTool extends FabricShapeTool {
    constructor() {
        super('arrow');
    }
}
