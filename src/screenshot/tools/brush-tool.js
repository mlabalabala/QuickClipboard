/**
 * 画笔工具
 * 负责在截屏上绘制自由画笔线条
 */

export class BrushTool {
    constructor() {
        this.name = 'brush';
        this.isDrawing = false;
        this.lastX = 0;
        this.lastY = 0;
        
        // 画笔参数
        this.strokeStyle = '#ff0000';  // 红色
        this.lineWidth = 3;
        this.lineCap = 'round';
        this.lineJoin = 'round';
        
        // 当前路径数据（用于撤销等功能）
        this.currentPath = [];
        this.allPaths = [];
    }

    /**
     * 设置画笔参数
     */
    setOptions(options) {
        if (options.color) this.strokeStyle = options.color;
        if (options.width) this.lineWidth = options.width;
        if (options.lineCap) this.lineCap = options.lineCap;
        if (options.lineJoin) this.lineJoin = options.lineJoin;
    }

    /**
     * 获取当前画笔参数
     */
    getOptions() {
        return {
            color: this.strokeStyle,
            width: this.lineWidth,
            lineCap: this.lineCap,
            lineJoin: this.lineJoin
        };
    }

    /**
     * 开始绘制
     */
    startDrawing(ctx, x, y) {
        this.isDrawing = true;
        this.lastX = x;
        this.lastY = y;
        
        // 设置画笔样式
        ctx.strokeStyle = this.strokeStyle;
        ctx.lineWidth = this.lineWidth;
        ctx.lineCap = this.lineCap;
        ctx.lineJoin = this.lineJoin;
        
        // 开始新路径
        ctx.beginPath();
        ctx.moveTo(x, y);
        
        // 记录路径起点
        this.currentPath = [{
            type: 'start',
            x: x,
            y: y,
            options: { ...this.getOptions() }
        }];
    }

    /**
     * 继续绘制
     */
    draw(ctx, x, y) {
        if (!this.isDrawing) return;

        // 绘制线条
        ctx.beginPath();
        ctx.moveTo(this.lastX, this.lastY);
        ctx.lineTo(x, y);
        ctx.stroke();

        // 记录路径点
        this.currentPath.push({
            type: 'line',
            fromX: this.lastX,
            fromY: this.lastY,
            toX: x,
            toY: y
        });

        this.lastX = x;
        this.lastY = y;
    }

    /**
     * 结束绘制
     */
    endDrawing() {
        if (!this.isDrawing) return;
        
        this.isDrawing = false;
        
        // 保存完整路径
        if (this.currentPath.length > 0) {
            this.allPaths.push([...this.currentPath]);
        }
        
        this.currentPath = [];
    }

    /**
     * 重绘所有路径（用于撤销重做等操作）
     */
    redrawAllPaths(ctx) {
        ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
        
        for (const path of this.allPaths) {
            this.redrawPath(ctx, path);
        }
    }

    /**
     * 重绘单个路径
     */
    redrawPath(ctx, path) {
        if (path.length === 0) return;

        // 设置路径样式
        const startPoint = path[0];
        if (startPoint.options) {
            ctx.strokeStyle = startPoint.options.color;
            ctx.lineWidth = startPoint.options.width;
            ctx.lineCap = startPoint.options.lineCap;
            ctx.lineJoin = startPoint.options.lineJoin;
        }

        ctx.beginPath();
        
        for (let i = 0; i < path.length; i++) {
            const point = path[i];
            
            if (point.type === 'start') {
                ctx.moveTo(point.x, point.y);
            } else if (point.type === 'line') {
                ctx.lineTo(point.toX, point.toY);
            }
        }
        
        ctx.stroke();
    }

    /**
     * 撤销上一步绘制
     */
    undo() {
        if (this.allPaths.length > 0) {
            this.allPaths.pop();
            return true;
        }
        return false;
    }

    /**
     * 清除所有绘制
     */
    clear() {
        this.allPaths = [];
        this.currentPath = [];
        this.isDrawing = false;
    }

    /**
     * 获取所有路径数据（用于导出）
     */
    getAllPaths() {
        return this.allPaths;
    }

    /**
     * 设置路径数据（用于导入）
     */
    setAllPaths(paths) {
        this.allPaths = paths || [];
    }

    /**
     * 工具激活时的处理
     */
    onActivate() {
        // 可以在这里设置特殊的光标样式等
        document.body.style.cursor = 'crosshair';
    }

    /**
     * 工具取消激活时的处理
     */
    onDeactivate() {
        // 如果正在绘制，结束绘制
        if (this.isDrawing) {
            this.endDrawing();
        }
        
        // 恢复默认光标
        document.body.style.cursor = 'default';
    }
}
