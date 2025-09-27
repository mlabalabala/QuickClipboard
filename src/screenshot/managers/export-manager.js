/**
 * 导出管理器
 * 负责将截屏选区内容复制到系统剪贴板
 */

export class ExportManager {
    constructor() {
        this.backgroundManager = null;
        this.editLayerManager = null;
    }

    /**
     * 设置背景管理器引用
     */
    setBackgroundManager(backgroundManager) {
        this.backgroundManager = backgroundManager;
    }

    /**
     * 设置编辑层管理器引用
     */
    setEditLayerManager(editLayerManager) {
        this.editLayerManager = editLayerManager;
    }

    /**
     * 将选区内容复制到系统剪贴板
     * @param {Object} selection - 选区信息 {left, top, width, height}
     */
    async copySelectionToClipboard(selection) {
        try {
            const backgroundCanvas = this.backgroundManager?.canvas;
            const backgroundCtx = this.backgroundManager?.ctx;
            
            if (!backgroundCanvas || !backgroundCtx) {
                throw new Error('背景Canvas未准备就绪');
            }

            // 创建选区Canvas（会自动合并编辑层内容）
            const selectionCanvas = this.createSelectionCanvas(backgroundCanvas, selection);
            
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

        // 如果有编辑层内容，也绘制到选区Canvas上
        if (this.editLayerManager && this.editLayerManager.hasContent()) {
            const editCanvas = this.editLayerManager.canvas;
            if (editCanvas) {
                try {
                    selectionCtx.drawImage(
                        editCanvas,
                        actualLeft, actualTop, actualWidth, actualHeight,  // 源区域
                        0, 0, actualWidth, actualHeight  // 目标区域
                    );
                    console.log('编辑层内容已合并到选区');
                } catch (error) {
                    console.error('合并编辑层到选区失败:', error);
                }
            }
        }

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

    /**
     * 合并背景层和编辑层
     * @returns {HTMLCanvasElement} - 合并后的Canvas
     */
    mergeLayersCanvas() {
        const backgroundCanvas = this.backgroundManager.canvas;
        const editCanvas = this.editLayerManager.canvas;
        
        if (!backgroundCanvas || !editCanvas) {
            throw new Error('Canvas未准备就绪');
        }

        // 创建合并Canvas
        const mergedCanvas = document.createElement('canvas');
        mergedCanvas.width = backgroundCanvas.width;
        mergedCanvas.height = backgroundCanvas.height;
        const mergedCtx = mergedCanvas.getContext('2d', { willReadFrequently: true });

        try {
            // 先绘制背景层
            mergedCtx.drawImage(backgroundCanvas, 0, 0);
            
            // 再绘制编辑层
            mergedCtx.drawImage(editCanvas, 0, 0);

            console.log('图层合并完成', {
                backgroundSize: `${backgroundCanvas.width}x${backgroundCanvas.height}`,
                editSize: `${editCanvas.width}x${editCanvas.height}`,
                mergedSize: `${mergedCanvas.width}x${mergedCanvas.height}`
            });

            return mergedCanvas;
        } catch (error) {
            console.error('合并图层时发生错误:', error);
            throw new Error('图层合并失败: ' + error.message);
        }
    }
}
