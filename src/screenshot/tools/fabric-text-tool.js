/**
 * Fabric.js文本工具
 * 基于Fabric.js的文本工具实现
 */

export class FabricTextTool {
    constructor() {
        this.name = 'text';
        this.fabricCanvas = null;
        this.editLayerManager = null;
        this.isActive = false;
        
        // 文本参数
        this.textOptions = {
            fontFamily: 'Arial',
            fontSize: 20,
            color: '#000000',
            fontWeight: 'normal',
            fontStyle: 'normal',
            textAlign: 'left'
        };
        
        // 绑定事件处理器
        this.handleCanvasClick = this.handleCanvasClick.bind(this);
    }

    /**
     * 设置文本参数
     */
    setOptions(options) {
        if (options.fontFamily !== undefined) this.textOptions.fontFamily = options.fontFamily;
        if (options.fontSize !== undefined) this.textOptions.fontSize = options.fontSize;
        if (options.color !== undefined) this.textOptions.color = options.color;
        if (options.fontWeight !== undefined) this.textOptions.fontWeight = options.fontWeight;
        if (options.fontStyle !== undefined) this.textOptions.fontStyle = options.fontStyle;
        if (options.textAlign !== undefined) this.textOptions.textAlign = options.textAlign;
    }

    /**
     * 获取当前文本参数
     */
    getOptions() {
        return { ...this.textOptions };
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
        document.body.style.cursor = 'text';
        
        // 添加点击事件监听器
        if (this.fabricCanvas) {
            this.fabricCanvas.on('mouse:down', this.handleCanvasClick);
        }
    }

    /**
     * 工具取消激活时的处理
     */
    onDeactivate(editLayerManager) {
        this.isActive = false;
        
        // 恢复默认光标
        document.body.style.cursor = 'default';
        
        // 移除事件监听器
        if (this.fabricCanvas) {
            this.fabricCanvas.off('mouse:down', this.handleCanvasClick);
        }
        
        this.fabricCanvas = null;
        this.editLayerManager = null;
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
     * 处理Canvas点击事件
     */
    handleCanvasClick(e) {
        if (!this.isActive || !this.fabricCanvas || !this.editLayerManager) return;
        
        // 如果点击的是已存在的对象，不创建新文本
        if (e.target && e.target !== this.fabricCanvas) return;
        
        const pointer = this.fabricCanvas.getPointer(e.e);
        this.addTextAt(pointer.x, pointer.y);
    }

    /**
     * 在指定位置添加文本
     */
    addTextAt(x, y) {
        if (!this.fabricCanvas || !this.editLayerManager) return null;
        
        const textObj = new fabric.Text('输入文本', {
            left: x,
            top: y,
            fontFamily: this.textOptions.fontFamily,
            fontSize: this.textOptions.fontSize,
            fill: this.textOptions.color,
            fontWeight: this.textOptions.fontWeight,
            fontStyle: this.textOptions.fontStyle,
            textAlign: this.textOptions.textAlign,
            editable: true
        });

        this.fabricCanvas.add(textObj);
        
        // 设置为可选择
        textObj.selectable = true;
        
        // 延迟保存状态，避免与Fabric事件冲突
        setTimeout(() => {
            if (this.editLayerManager && this.editLayerManager.saveState) {
                this.editLayerManager.saveState('添加文本');
            }
        }, 50);
        
        // 切换到选择工具并选中新创建的文本，然后进入编辑模式
        this.switchToSelectionTool(textObj);
        
        // 延迟进入编辑模式，确保选择工具已经激活
        setTimeout(() => {
            textObj.enterEditing();
            this.fabricCanvas.renderAll();
        }, 100);
        
        return textObj;
    }

    /**
     * 设置字体
     */
    setFont(fontFamily) {
        this.setOptions({ fontFamily });
    }

    /**
     * 设置字体大小
     */
    setFontSize(fontSize) {
        this.setOptions({ fontSize });
    }

    /**
     * 设置文本颜色
     */
    setColor(color) {
        this.setOptions({ color });
    }

    /**
     * 设置字体粗细
     */
    setFontWeight(fontWeight) {
        this.setOptions({ fontWeight });
    }

    /**
     * 设置字体样式
     */
    setFontStyle(fontStyle) {
        this.setOptions({ fontStyle });
    }

    /**
     * 获取字体
     */
    getFont() {
        return this.textOptions.fontFamily;
    }

    /**
     * 获取字体大小
     */
    getFontSize() {
        return this.textOptions.fontSize;
    }

    /**
     * 获取文本颜色
     */
    getColor() {
        return this.textOptions.color;
    }
}
