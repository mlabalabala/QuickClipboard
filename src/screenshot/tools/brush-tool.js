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
        
        // 当前路径数据
        this.currentPath = [];
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
        
        // 路径绘制完成
        
        this.currentPath = [];
    }


    /**
     * 清除当前绘制状态
     */
    clear() {
        this.currentPath = [];
        this.isDrawing = false;
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
