/**
 * 导出管理器
 * 负责将截屏选区内容复制到系统剪贴板
 */

export class ExportManager {
    constructor() {
        this.backgroundManager = null;
    }

    /**
     * 设置背景管理器引用
     */
    setBackgroundManager(backgroundManager) {
        this.backgroundManager = backgroundManager;
    }

    /**
     * 将选区内容复制到系统剪贴板
     * @param {Object} selection - 选区信息 {left, top, width, height}
     */
    async copySelectionToClipboard(selection) {
        try {
            const canvas = this.backgroundManager?.canvas;
            const ctx = this.backgroundManager?.ctx;
            
            if (!canvas || !ctx) {
                throw new Error('Canvas未准备就绪');
            }

            // 创建选区Canvas
            const selectionCanvas = this.createSelectionCanvas(canvas, selection);
            
            // 转换为PNG格式的Blob
            const blob = await this.canvasToBlob(selectionCanvas);

            // 写入系统剪贴板
            await this.writeToClipboard(blob);
            
            console.log('截屏已复制到剪贴板');
        } catch (error) {
            console.error('复制到剪贴板失败:', error);
            throw error;
        }
    }

    /**
     * 创建选区Canvas
     * @param {HTMLCanvasElement} sourceCanvas - 源Canvas
     * @param {Object} selection - 选区信息
     * @returns {HTMLCanvasElement} - 选区Canvas
     */
    createSelectionCanvas(sourceCanvas, selection) {
        // 计算Canvas实际尺寸与显示尺寸的比例
        const canvasRect = sourceCanvas.getBoundingClientRect();
        const scaleX = sourceCanvas.width / canvasRect.width;
        const scaleY = sourceCanvas.height / canvasRect.height;

        // 将选区坐标转换为Canvas实际坐标
        const actualLeft = selection.left * scaleX;
        const actualTop = selection.top * scaleY;
        const actualWidth = selection.width * scaleX;
        const actualHeight = selection.height * scaleY;

        console.log('选区坐标转换:', {
            original: selection,
            scale: { scaleX, scaleY },
            actual: { actualLeft, actualTop, actualWidth, actualHeight }
        });

        // 创建新的Canvas来绘制选区部分
        const selectionCanvas = document.createElement('canvas');
        selectionCanvas.width = actualWidth;
        selectionCanvas.height = actualHeight;
        const selectionCtx = selectionCanvas.getContext('2d');

        // 从背景Canvas复制选区部分（使用实际坐标）
        selectionCtx.drawImage(
            sourceCanvas,
            actualLeft, actualTop, actualWidth, actualHeight,  // 源区域（Canvas实际坐标）
            0, 0, actualWidth, actualHeight  // 目标区域
        );

        return selectionCanvas;
    }

    /**
     * 将Canvas转换为Blob
     * @param {HTMLCanvasElement} canvas - Canvas元素
     * @returns {Promise<Blob>} - Blob对象
     */
    async canvasToBlob(canvas) {
        return new Promise((resolve) => {
            canvas.toBlob(resolve, 'image/png');
        });
    }

    /**
     * 写入系统剪贴板
     * @param {Blob} blob - 图片Blob
     */
    async writeToClipboard(blob) {
        if (navigator.clipboard && navigator.clipboard.write) {
            const clipboardItem = new ClipboardItem({
                'image/png': blob
            });
            await navigator.clipboard.write([clipboardItem]);
        } else {
            throw new Error('浏览器不支持剪贴板API');
        }
    }
}
