/**
 * Fabric.js 统一形状工具
 * 包含矩形、圆形、箭头形状等几何形状
 */

import * as fabric from 'fabric';

export class FabricUnifiedShapeTool {
    constructor() {
        this.name = 'shape';
        this.editLayerManager = null;
        this.fabricCanvas = null;
        this.isActive = false;
        this.isDrawing = false;
        this.startPoint = null;
        this.currentShape = null;
        this.currentShapeType = 'rectangle'; // 当前选择的形状类型
        
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
     * 设置当前形状类型
     */
    setShapeType(shapeType) {
        this.currentShapeType = shapeType;
    }

    /**
     * 获取当前形状类型
     */
    getShapeType() {
        return this.currentShapeType;
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
     * 获取形状参数
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
            case 'shapeType':
                // 形状类型切换
                this.setShapeType(value);
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
        
        // 如果启用了填充，也应用到填充颜色
        if (this.isFillEnabled()) {
            let fillColor = this.shapeOptions.fill;
            if (fillColor.startsWith('#')) {
                const hex = fillColor.slice(1);
                const r = parseInt(hex.slice(0, 2), 16);
                const g = parseInt(hex.slice(2, 4), 16);
                const b = parseInt(hex.slice(4, 6), 16);
                fillColor = `rgba(${r}, ${g}, ${b}, ${opacity})`;
            } else if (fillColor.startsWith('rgb(')) {
                fillColor = fillColor.replace('rgb(', 'rgba(').replace(')', `, ${opacity})`);
            } else if (fillColor.startsWith('rgba(')) {
                fillColor = fillColor.replace(/,\s*[\d.]+\s*\)$/, `, ${opacity})`);
            }
            this.shapeOptions.fill = fillColor;
        }
    }

    /**
     * 应用设置到活动的形状对象
     */
    applyToActiveShape() {
        if (this.fabricCanvas) {
            const activeObject = this.fabricCanvas.getActiveObject();
            if (activeObject && (activeObject.type === 'rect' || activeObject.type === 'circle' || 
                                 activeObject.type === 'ellipse' || activeObject.type === 'path' || 
                                 activeObject.type === 'group')) {
                activeObject.set(this.shapeOptions);
                this.fabricCanvas.renderAll();
            }
        }
    }

    /**
     * 工具激活时的处理
     */
    onActivate(editLayerManager) {
        if (!editLayerManager || !editLayerManager.getFabricCanvas) {
            console.error('形状工具激活失败：editLayerManager 无效');
            return;
        }
        
        this.editLayerManager = editLayerManager;
        this.fabricCanvas = editLayerManager.getFabricCanvas();
        
        if (!this.fabricCanvas) {
            console.error('形状工具激活失败：fabricCanvas 为空');
            return;
        }
        
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
        this.startPoint = null;
        this.currentShape = null;
        
        // 恢复光标
        document.body.style.cursor = 'default';
        
        // 移除事件监听器
        if (this.fabricCanvas) {
            this.fabricCanvas.off('mouse:down', this.handleMouseDown);
            this.fabricCanvas.off('mouse:move', this.handleMouseMove);
            this.fabricCanvas.off('mouse:up', this.handleMouseUp);
        }
    }

    /**
     * 处理鼠标按下事件
     */
    handleMouseDown(options) {
        if (!this.isActive) return;
        
        const pointer = this.fabricCanvas.getPointer(options.e);
        this.startPoint = { x: pointer.x, y: pointer.y };
        this.isDrawing = true;
        
        // 创建预览形状
        this.currentShape = this.createShape(
            this.startPoint.x, 
            this.startPoint.y, 
            1, 
            1, 
            this.shapeOptions
        );
        
        if (this.currentShape) {
            this.currentShape.excludeFromHistory = true;
            this.fabricCanvas.add(this.currentShape);
            this.fabricCanvas.renderAll();
        }
    }

    /**
     * 处理鼠标移动事件
     */
    handleMouseMove(options) {
        if (!this.isActive || !this.isDrawing || !this.startPoint || !this.currentShape) return;
        
        const pointer = this.fabricCanvas.getPointer(options.e);
        
        // 更新形状
        this.updateShape(this.currentShape, this.startPoint, pointer);
        this.fabricCanvas.renderAll();
    }

    /**
     * 处理鼠标松开事件
     */
    handleMouseUp(options) {
        if (!this.isActive || !this.isDrawing || !this.startPoint) return;
        
        const pointer = this.fabricCanvas.getPointer(options.e);
        const distance = Math.sqrt(
            Math.pow(pointer.x - this.startPoint.x, 2) + 
            Math.pow(pointer.y - this.startPoint.y, 2)
        );
        
        if (distance < 10) {
            // 距离太短，删除形状
            if (this.currentShape) {
                this.fabricCanvas.remove(this.currentShape);
            }
        } else {
            // 完成形状创建
            this.finishShape();
        }
        
        this.isDrawing = false;
        this.startPoint = null;
        this.currentShape = null;
        this.fabricCanvas.renderAll();
    }

    /**
     * 完成形状创建
     */
    finishShape() {
        if (!this.currentShape) return;
        this.currentShape.excludeFromHistory = false;
        
        // 标记对象可选择
        this.currentShape.selectable = true;
        this.currentShape.evented = true;
        this.currentShape.excludeFromHistory = false;

        if (this.editLayerManager && this.editLayerManager.requestHistorySave) {
            this.currentShape.historyAddReason = `创建${this.getShapeTypeLabel()}`;
            this.editLayerManager.requestHistorySave(`创建${this.getShapeTypeLabel()}`, { immediate: true });
        }
        
        // 切换到选择工具并选中新创建的形状
        if (window.screenshotController && window.screenshotController.toolManager) {
            window.screenshotController.toolManager.switchToSelectionTool(this.currentShape);
        }
    }

    getShapeTypeLabel() {
        switch (this.currentShapeType) {
            case 'rectangle':
                return '矩形';
            case 'circle':
                return '圆形';
            case 'arrow':
                return '箭头形状';
            default:
                return '形状';
        }
    }

    /**
     * 创建形状
     */
    createShape(x, y, width, height, options) {
        const left = x;
        const top = y;
        
        switch (this.currentShapeType) {
            case 'rectangle':
                return new fabric.Rect({
                    left,
                    top,
                    width: Math.abs(width),
                    height: Math.abs(height),
                    ...options
                });
                
            case 'circle':
                const radius = Math.min(Math.abs(width), Math.abs(height)) / 2;
                return new fabric.Circle({
                    radius,
                    left: x - radius,
                    top: y - radius,
                    ...options
                });
                
            case 'arrow':
                return this.createArrowShape(x, y, x + width, y + height, options);
                
            default:
                return null;
        }
    }

    /**
     * 创建箭头形状（几何形状，可填充）
     */
    createArrowShape(startX, startY, endX, endY, options) {
        const length = Math.sqrt((endX - startX) ** 2 + (endY - startY) ** 2);
        if (length < 10) return null;
        
        const angle = Math.atan2(endY - startY, endX - startX);
        const arrowWidth = 8;
        const arrowHeadLength = 20;
        const arrowHeadWidth = 15;
        
        // 创建箭头路径（可填充的几何形状）
        const pathString = [
            'M', startX, startY - arrowWidth / 2,
            'L', endX - arrowHeadLength, endY - arrowWidth / 2,
            'L', endX - arrowHeadLength, endY - arrowHeadWidth / 2,
            'L', endX, endY,
            'L', endX - arrowHeadLength, endY + arrowHeadWidth / 2,
            'L', endX - arrowHeadLength, endY + arrowWidth / 2,
            'L', startX, startY + arrowWidth / 2,
            'Z'
        ].join(' ');
        
        const path = new fabric.Path(pathString, {
            ...options,
            left: Math.min(startX, endX),
            top: Math.min(startY, endY) - arrowHeadWidth / 2
        });
        path.excludeFromHistory = true;
        return path;
    }

    /**
     * 更新形状
     */
    updateShape(shape, startPoint, currentPoint) {
        if (!shape || !startPoint) return;
        
        const width = currentPoint.x - startPoint.x;
        const height = currentPoint.y - startPoint.y;
        const left = Math.min(startPoint.x, currentPoint.x);
        const top = Math.min(startPoint.y, currentPoint.y);

        switch (this.currentShapeType) {
            case 'rectangle':
                shape.set({
                    left,
                    top,
                    width: Math.abs(width),
                    height: Math.abs(height)
                });
                break;
                
            case 'circle':
                const radius = Math.min(Math.abs(width), Math.abs(height)) / 2;
                shape.set({
                    radius,
                    left: startPoint.x - radius,
                    top: startPoint.y - radius
                });
                break;
                
            case 'arrow':
                this.fabricCanvas.remove(shape);
                this.currentShape = this.createArrowShape(
                    startPoint.x,
                    startPoint.y,
                    currentPoint.x,
                    currentPoint.y,
                    this.shapeOptions
                );
                if (this.currentShape) {
                    this.currentShape.excludeFromHistory = true;
                    this.fabricCanvas.add(this.currentShape);
                }
                break;
        }
    }
}
