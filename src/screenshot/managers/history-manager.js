/**
 * 历史管理模块
 * 统一管理所有编辑操作的历史记录，支持撤销和重做功能
 */

export class HistoryManager {
    constructor() {
        // 历史记录数组，每个元素包含完整的Canvas状态
        this.history = [];
        // 当前历史位置
        this.currentIndex = -1;
        // 最大历史记录数量
        this.maxHistorySize = 50;
        
        // 回调函数
        this.onHistoryChange = null;
    }

    /**
     * 设置历史状态改变时的回调函数
     * @param {Function} callback - 回调函数，参数为 {canUndo, canRedo}
     */
    setOnHistoryChange(callback) {
        this.onHistoryChange = callback;
    }

    /**
     * 保存当前状态到历史记录
     * @param {CanvasRenderingContext2D} ctx - Canvas上下文
     * @param {string} description - 操作描述（可选）
     */
    saveState(ctx, description = '') {
        if (!ctx || !ctx.canvas) {
            console.warn('HistoryManager: Invalid context provided');
            return;
        }

        try {
            // 删除当前位置之后的所有历史记录（分支历史）
            if (this.currentIndex < this.history.length - 1) {
                this.history.splice(this.currentIndex + 1);
            }

            // 获取当前Canvas状态
            const imageData = ctx.getImageData(0, 0, ctx.canvas.width, ctx.canvas.height);
            
            // 创建历史记录项
            const historyItem = {
                imageData: imageData,
                timestamp: Date.now(),
                description: description,
                width: ctx.canvas.width,
                height: ctx.canvas.height
            };

            // 添加到历史记录
            this.history.push(historyItem);
            this.currentIndex = this.history.length - 1;

            // 限制历史记录大小
            if (this.history.length > this.maxHistorySize) {
                this.history.shift();
                this.currentIndex--;
            }

            // 触发历史状态改变回调
            this.triggerHistoryChange();

        } catch (error) {
            console.error('HistoryManager: Error saving state:', error);
        }
    }

    /**
     * 撤销操作
     * @param {CanvasRenderingContext2D} ctx - Canvas上下文
     * @returns {boolean} - 是否成功撤销
     */
    undo(ctx) {
        if (!this.canUndo()) {
            return false;
        }

        if (!ctx || !ctx.canvas) {
            console.warn('HistoryManager: Invalid context provided for undo');
            return false;
        }

        try {
            this.currentIndex--;
            const historyItem = this.history[this.currentIndex];
            
            // 检查Canvas尺寸是否匹配
            if (historyItem.width !== ctx.canvas.width || historyItem.height !== ctx.canvas.height) {
                console.warn('HistoryManager: Canvas size mismatch during undo');
                // 尝试调整Canvas尺寸
                ctx.canvas.width = historyItem.width;
                ctx.canvas.height = historyItem.height;
            }

            // 恢复Canvas状态
            ctx.putImageData(historyItem.imageData, 0, 0);

            // 触发历史状态改变回调
            this.triggerHistoryChange();

            return true;
        } catch (error) {
            console.error('HistoryManager: Error during undo:', error);
            // 恢复索引
            this.currentIndex++;
            return false;
        }
    }

    /**
     * 重做操作
     * @param {CanvasRenderingContext2D} ctx - Canvas上下文
     * @returns {boolean} - 是否成功重做
     */
    redo(ctx) {
        if (!this.canRedo()) {
            return false;
        }

        if (!ctx || !ctx.canvas) {
            console.warn('HistoryManager: Invalid context provided for redo');
            return false;
        }

        try {
            this.currentIndex++;
            const historyItem = this.history[this.currentIndex];
            
            // 检查Canvas尺寸是否匹配
            if (historyItem.width !== ctx.canvas.width || historyItem.height !== ctx.canvas.height) {
                console.warn('HistoryManager: Canvas size mismatch during redo');
                // 尝试调整Canvas尺寸
                ctx.canvas.width = historyItem.width;
                ctx.canvas.height = historyItem.height;
            }

            // 恢复Canvas状态
            ctx.putImageData(historyItem.imageData, 0, 0);

            // 触发历史状态改变回调
            this.triggerHistoryChange();

            return true;
        } catch (error) {
            console.error('HistoryManager: Error during redo:', error);
            // 恢复索引
            this.currentIndex--;
            return false;
        }
    }

    /**
     * 检查是否可以撤销
     * @returns {boolean}
     */
    canUndo() {
        return this.currentIndex > 0;
    }

    /**
     * 检查是否可以重做
     * @returns {boolean}
     */
    canRedo() {
        return this.currentIndex < this.history.length - 1;
    }

    /**
     * 清除所有历史记录
     */
    clear() {
        this.history = [];
        this.currentIndex = -1;
        this.triggerHistoryChange();
    }

    /**
     * 获取历史记录信息
     * @returns {Object} 包含历史记录统计信息的对象
     */
    getInfo() {
        return {
            totalCount: this.history.length,
            currentIndex: this.currentIndex,
            canUndo: this.canUndo(),
            canRedo: this.canRedo(),
            memoryUsage: this.calculateMemoryUsage()
        };
    }

    /**
     * 计算历史记录占用的内存大小（估算）
     * @returns {number} 内存大小（字节）
     */
    calculateMemoryUsage() {
        let totalSize = 0;
        for (const item of this.history) {
            if (item.imageData && item.imageData.data) {
                totalSize += item.imageData.data.length * 4; // RGBA 每像素4字节
            }
        }
        return totalSize;
    }

    /**
     * 获取格式化的内存使用信息
     * @returns {string} 格式化的内存大小字符串
     */
    getFormattedMemoryUsage() {
        const bytes = this.calculateMemoryUsage();
        if (bytes < 1024) return `${bytes} B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }

    /**
     * 设置最大历史记录数量
     * @param {number} maxSize - 最大历史记录数量
     */
    setMaxHistorySize(maxSize) {
        if (maxSize > 0) {
            this.maxHistorySize = maxSize;
            
            // 如果当前历史记录超过限制，删除最旧的记录
            while (this.history.length > this.maxHistorySize) {
                this.history.shift();
                this.currentIndex--;
            }
            
            // 确保索引不小于-1
            if (this.currentIndex < -1) {
                this.currentIndex = -1;
            }
        }
    }

    /**
     * 触发历史状态改变回调
     */
    triggerHistoryChange() {
        if (this.onHistoryChange) {
            this.onHistoryChange({
                canUndo: this.canUndo(),
                canRedo: this.canRedo(),
                currentIndex: this.currentIndex,
                totalCount: this.history.length
            });
        }
    }

    /**
     * 获取当前状态描述
     * @returns {string} 当前状态的描述
     */
    getCurrentDescription() {
        if (this.currentIndex >= 0 && this.currentIndex < this.history.length) {
            return this.history[this.currentIndex].description || '未知操作';
        }
        return '初始状态';
    }

    /**
     * 导出历史记录（用于调试或数据分析）
     * @returns {Array} 包含历史记录元数据的数组
     */
    exportMetadata() {
        return this.history.map((item, index) => ({
            index: index,
            timestamp: item.timestamp,
            description: item.description,
            width: item.width,
            height: item.height,
            isCurrent: index === this.currentIndex
        }));
    }
}
