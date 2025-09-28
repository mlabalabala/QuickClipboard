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
        
        // 形状参数
        this.shapeOptions = {
            fill: 'rgba(255, 0, 0, 0.3)',
            stroke: '#ff0000',
            strokeWidth: 2,
            strokeDashArray: null
        };
        
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
     * 工具激活时的处理
     */
    onActivate(editLayerManager) {
        if (!editLayerManager || !editLayerManager.getFabricCanvas) return;
        
        this.editLayerManager = editLayerManager;
        this.fabricCanvas = editLayerManager.getFabricCanvas();
        this.isActive = true;
        
        // 确保不在绘画模式
        editLayerManager.disableDrawingMode();
        
        // 设置光标
        document.body.style.cursor = 'crosshair';
        
        // 添加事件监听器
        if (this.fabricCanvas) {
            this.fabricCanvas.on('mouse:down', this.handleMouseDown);
            this.fabricCanvas.on('mouse:move', this.handleMouseMove);
            this.fabricCanvas.on('mouse:up', this.handleMouseUp);
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
     * 处理鼠标按下事件
     */
    handleMouseDown(e) {
        if (!this.isActive || !this.fabricCanvas) return;
        
        // 如果点击的是已存在的对象，不创建新形状
        if (e.target && e.target !== this.fabricCanvas) return;
        
        const pointer = this.fabricCanvas.getPointer(e.e);
        this.startPoint = { x: pointer.x, y: pointer.y };
        this.isDrawing = true;
        
        // 创建初始形状
        this.currentShape = this.createShape(pointer.x, pointer.y, 1, 1);
        if (this.currentShape) {
            this.fabricCanvas.add(this.currentShape);
            this.fabricCanvas.setActiveObject(this.currentShape);
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
                this.fabricCanvas.renderAll();
            } else {
                // 形状创建完成，延迟保存历史状态，避免与Fabric事件冲突
                setTimeout(() => {
                    if (this.editLayerManager && this.editLayerManager.saveState) {
                        this.editLayerManager.saveState(`添加${this.shapeType}`);
                    }
                }, 50);
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
