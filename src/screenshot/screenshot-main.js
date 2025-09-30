/**
 * 截屏主控制器
 * 整合所有子模块，协调截屏功能的各个部分
 */

// 引入Fabric.js
import * as fabric from 'fabric';

// 设置全局fabric
window.fabric = fabric;

import { ScreenshotAPI } from './api/screenshot-api.js';
import { SelectionManager } from './managers/selection-manager.js';
import { ToolbarManager } from './managers/toolbar-manager.js';
import { SubToolbarManager } from './managers/sub-toolbar-manager.js';
import { MaskManager } from './managers/mask-manager.js';
import { EventManager } from './managers/event-manager.js';
import { BackgroundManager } from './managers/background-manager.js';
import { ExportManager } from './managers/export-manager.js';
import { FabricEditLayerManager } from './managers/fabric-edit-layer-manager.js';
import { FabricToolManager } from './managers/fabric-tool-manager.js';
import { MagnifierManager } from './managers/magnifier-manager.js';
import { registerArrowClass } from './tools/fabric-simple-arrow-tool.js';

export class ScreenshotController {
    constructor() {
        this.monitors = [];
        
        // 确保Fabric.js已加载
        if (typeof fabric === 'undefined') {
            console.error('ScreenshotController: Fabric.js 未加载，等待加载...');
            // 延迟初始化，等待Fabric.js加载
            setTimeout(() => this.initializeManagers(), 100);
            return;
        }
        
        this.initializeController();
        registerArrowClass();
    }
    
    initializeController() {
        console.log('ScreenshotController: 开始初始化，Fabric.js 版本:', fabric.version);
        
        // 初始化各个管理器
        this.selectionManager = new SelectionManager();
        this.toolbarManager = new ToolbarManager();
        this.subToolbarManager = new SubToolbarManager();
        this.maskManager = new MaskManager();
        this.eventManager = new EventManager();
        this.backgroundManager = new BackgroundManager();
        this.exportManager = new ExportManager();
        this.editLayerManager = new FabricEditLayerManager();
        this.toolManager = new FabricToolManager();
        this.magnifierManager = new MagnifierManager();
        
        // 设置管理器之间的引用关系
        this.exportManager.setBackgroundManager(this.backgroundManager);
        this.exportManager.setEditLayerManager(this.editLayerManager);
        this.editLayerManager.setBackgroundManager(this.backgroundManager);
        this.toolManager.setEditLayerManager(this.editLayerManager);
        
        // 设置子工具栏参数变化回调
        this.subToolbarManager.onParameterChange((toolName, paramName, value) => {
            this.handleParameterChange(toolName, paramName, value);
        });
        
        this.initializeManagers();
        this.loadMonitorInfo();
        this.showInitialInfo();
        
        // 设置全局引用，供工具使用
        window.screenshotController = this;
        
        console.log('ScreenshotController: 初始化完成');
    }

    /**
     * 初始化各管理器的事件绑定
     */
    initializeManagers() {
        // 事件管理器回调
        this.eventManager.setOnSelectionStart((x, y, target) => this.handleSelectionStart(x, y, target));
        this.eventManager.setOnSelectionUpdate((x, y) => this.handleSelectionUpdate(x, y));
        this.eventManager.setOnSelectionEnd(() => this.handleSelectionEnd());
        this.eventManager.setOnRightClick((x, y) => this.handleRightClick(x, y));
        this.eventManager.setOnKeyDown((key) => this.handleKeyDown(key));
        this.eventManager.setOnWindowFocus(() => this.handleWindowFocus());
        this.eventManager.setOnWindowBlur(() => this.handleWindowBlur());
        
        // 监听后端截屏完成事件
        window.__TAURI__.event.listen('screenshot-ready', (event) => {
            const payload = event.payload;

            if (!payload || typeof payload !== 'object') {
                console.error('无法解析截屏事件数据', payload);
                return;
            }

            const { width, height, image_url } = payload;
            if (!width || !height || !image_url) {
                console.error('截屏事件缺少字段', payload);
                return;
            }

            this.handleScreenshotData({ width, height, image_url });
        });

        // 监听后端截屏错误事件
        window.__TAURI__.event.listen('screenshot-error', (event) => {
            console.error('后端截屏失败:', event.payload);
        });

        // 页面加载完成时初始化背景管理器
        if (document.readyState === 'complete') {
            this.backgroundManager.init();
        } else {
            window.addEventListener('load', () => {
                this.backgroundManager.init();
            });
        }

        // 工具栏管理器回调
        this.toolbarManager.setOnConfirm(() => this.confirmScreenshot());
        this.toolbarManager.setOnCancel(() => this.cancelScreenshot());
        this.toolbarManager.setOnToolSelect((toolName) => this.handleToolSelect(toolName));
        this.toolbarManager.setOnUndo(() => this.handleUndo());
        this.toolbarManager.setOnRedo(() => this.handleRedo());
        
        // 编辑层历史状态回调
        this.editLayerManager.setOnHistoryChange((historyState) => {
            this.toolbarManager.updateHistoryButtons(historyState.canUndo, historyState.canRedo);
        });
        
        // 编辑层活动对象变化回调
        this.editLayerManager.setOnActiveObjectChange((objectInfo) => {
            this.handleActiveObjectChange(objectInfo);
        });
    }

    /**
     * 处理选择开始
     */
    handleSelectionStart(x, y, target) {
        const action = this.selectionManager.startSelection(x, y, target);
        
        if (action === 'select') {
            this.eventManager.hideInfoText();
            this.hideAllToolbars();
            // 开始新的选择时，显示放大镜
            if (this.magnifierManager) {
                this.magnifierManager.show();
            }
        } else if (action === 'move') {
            this.hideAllToolbars();
        } else if (action === 'resize') {
            this.hideAllToolbars();
        }
    }

    /**
     * 处理选择更新
     */
    handleSelectionUpdate(x, y) {
        // 始终更新放大镜位置（不管是否可见，因为它可能随时变为可见）
        if (this.magnifierManager) {
            this.magnifierManager.update(x, y);
        }
        
        if (this.selectionManager.isSelectingState) {
            this.selectionManager.updateSelection(x, y);
            const selection = this.selectionManager.getSelection();
            if (selection) {
                this.maskManager.updateMask(selection.left, selection.top, selection.width, selection.height);
            }
            // 只在选择过程中隐藏工具栏
            this.hideAllToolbars();
        } else if (this.selectionManager.isMovingState) {
            // 完全按照原版：直接调用，不等待！
            this.selectionManager.moveSelection(x, y, this.maskManager);
            // 只在移动过程中隐藏工具栏
            this.hideAllToolbars();
        } else if (this.selectionManager.isResizingState) {
            // 调整大小模式
            this.selectionManager.resizeSelection(x, y, this.maskManager);
            // 只在调整过程中隐藏工具栏
            this.hideAllToolbars();
        }
        // 如果既不在选择也不在移动状态，就不要隐藏工具栏
    }

    /**
     * 处理选择结束
     */
    handleSelectionEnd() {
        const action = this.selectionManager.endSelection();
        
        if (action === 'move-end' || action === 'select-end' || action === 'resize-end') {
            const selection = this.selectionManager.getSelection();
            if (selection) {
                this.toolbarManager.show(selection);
                
                // 有选区时隐藏放大镜
                if (this.magnifierManager) {
                    this.magnifierManager.hide();
                }
                
                // 如果有激活的工具，显示对应的子工具栏
                const currentTool = this.toolbarManager.getCurrentTool();
                if (currentTool) {
                    this.showSubToolbarForTool(currentTool);
                }
            } else {
                // 没有选区时，禁用所有编辑工具
                this.disableAllTools();
            }
        }
    }

    /**
     * 隐藏所有工具栏（主工具栏和子工具栏）
     */
    hideAllToolbars() {
        this.toolbarManager.hide();
        this.subToolbarManager.hide();
    }

    /**
     * 禁用所有编辑工具（无选区时调用）
     */
    disableAllTools() {
        // 停用当前激活的工具
        if (this.toolManager) {
            this.toolManager.deactivateTool();
        }
        
        // 清除工具栏选中状态
        if (this.toolbarManager) {
            this.toolbarManager.setActiveTool(null);
        }
        
        // 隐藏所有工具栏
        this.hideAllToolbars();
    }

    /**
     * 处理右键点击
     */
    handleRightClick(x, y) {
        const selection = this.selectionManager.getSelection();
        
        if (selection) {
            // 有选区时：取消选区，回到初始状态
            this.clearSelection();
        } else {
            // 没有选区时：关闭截屏窗口
            this.cancelScreenshot();
        }
    }

    /**
     * 处理活动对象变化
     */
    handleActiveObjectChange(objectInfo) {
        const { activeObject, objects, type } = objectInfo;
        
        if (!activeObject || !type) {
            // 没有选中对象，隐藏子工具栏
            this.subToolbarManager.hide();
            return;
        }
        
        // 映射对象类型到工具名称，然后显示子工具栏
        const toolName = this.mapObjectTypeToToolName(type);
        if (toolName) {
            const selectionRect = this.getActiveObjectBounds(activeObject);
            this.showSubToolbarForTool(toolName, selectionRect);
        } else {
            this.subToolbarManager.hide();
        }
    }

    /**
     * 映射对象类型到工具名称
     */
    mapObjectTypeToToolName(objectType) {
        switch (objectType) {
            case 'brush':
                return 'brush';
            case 'text':
                return 'text';
            case 'arrow':
                return 'arrow';
            case 'rectangle':
            case 'circle':
            case 'ellipse':           // 椭圆
            case 'triangle':          // 三角形
            case 'diamond':           // 菱形
            case '5-gon':             // 五边形
            case '6-gon':             // 六边形
            case 'star':              // 星形
            case 'shape-arrow':       // 形状工具中的箭头形状
                return 'shape';
            case 'selection':
            default:
                return null; // 多选或未知类型
        }
    }

    /**
     * 获取活动对象的边界
     */
    getActiveObjectBounds(activeObject) {
        if (!activeObject) return null;
        
        try {
            const bounds = activeObject.getBoundingRect();
            return {
                left: bounds.left,
                top: bounds.top,
                width: bounds.width,
                height: bounds.height
            };
        } catch (error) {
            console.warn('获取对象边界失败:', error);
            return null;
        }
    }

    /**
     * 处理键盘事件
     */
    handleKeyDown(key) {
        if (key === 'escape') {
            this.cancelScreenshot();
        } else if (key === 'enter') {
            const selection = this.selectionManager.getSelection();
            if (selection) {
                this.confirmScreenshot();
            }
        } else if (key === 'ctrl+z') {
            // Ctrl+Z 撤销 - 但要检查是否正在编辑文本
            if (this.canUseKeyboardShortcuts()) {
                this.handleUndo();
            }
        } else if (key === 'ctrl+y' || key === 'ctrl+shift+z') {
            // Ctrl+Y 或 Ctrl+Shift+Z 重做 - 但要检查是否正在编辑文本
            if (this.canUseKeyboardShortcuts()) {
                this.handleRedo();
            }
        }
    }

    /**
     * 处理窗口获得焦点
     */
    handleWindowFocus() {
        this.reset();
    }

    /**
     * 处理窗口失去焦点
     */
    handleWindowBlur() {
        this.reset();
    }

    /**
     * 加载显示器信息
     */
    async loadMonitorInfo() {
        try {
            this.monitors = await ScreenshotAPI.getMonitors();
            
            // 计算虚拟屏幕边界
            const virtualBounds = this.calculateVirtualBounds(this.monitors);
            
            // 将边界信息传递给选区管理器
            this.selectionManager.setMonitorBounds(this.monitors, virtualBounds);
        } catch (error) {
            console.error('Failed to load monitor info:', error);
            this.monitors = [];
        }
    }
    
    /**
     * 计算虚拟屏幕边界
     */
    calculateVirtualBounds(monitors) {
        if (!monitors || monitors.length === 0) {
            return { x: 0, y: 0, width: window.innerWidth, height: window.innerHeight };
        }
        
        let minX = Infinity, minY = Infinity;
        let maxX = -Infinity, maxY = -Infinity;
        
        monitors.forEach(monitor => {
            minX = Math.min(minX, monitor.x);
            minY = Math.min(minY, monitor.y);
            maxX = Math.max(maxX, monitor.x + monitor.width);
            maxY = Math.max(maxY, monitor.y + monitor.height);
        });
        
        return {
            x: minX,
            y: minY,
            width: maxX - minX,
            height: maxY - minY
        };
    }

    /**
     * 处理后端发送的BMP截屏数据
     */
    async handleScreenshotData(payload) {
        try {
            if (!this.backgroundManager.canvas) {
                this.backgroundManager.init();
            }

            await this.backgroundManager.loadScreenshot({ 
                width: payload.width, 
                height: payload.height, 
                image_url: payload.image_url 
            });
            
            // 初始化编辑层
            this.editLayerManager.init();
            
            // 设置放大镜的背景画布并显示
            if (this.backgroundManager.canvas) {
                this.magnifierManager.setBackgroundCanvas(this.backgroundManager.canvas);
                this.magnifierManager.show();
            }
            
            // 确保初始状态下工具是禁用的（因为没有选区）
            this.disableAllTools();
        } catch (error) {
            console.error('处理截屏数据失败:', error);
        }
    }

    /**
     * 显示初始信息
     */
    showInitialInfo() {
        this.eventManager.showInfoText('拖拽选择截屏区域，选区内可拖拽移动，右键取消/关闭，按 ESC 键关闭');
    }

    /**
     * 处理工具选择
     */
    handleToolSelect(toolName) {
        if (toolName) {
            // 激活工具
            this.toolManager.activateTool(toolName);
            // 更新工具栏按钮状态
            this.toolbarManager.setActiveTool(toolName);
            // 显示工具参数栏
            this.showSubToolbarForTool(toolName);
        } else {
            // 取消激活工具
            this.toolManager.deactivateTool();
            // 清除工具栏按钮状态
            this.toolbarManager.setActiveTool(null);
            // 隐藏参数栏
            this.subToolbarManager.hide();
        }
    }

    /**
     * 为指定工具显示子工具栏
     */
    showSubToolbarForTool(toolName, selectionRect = null) {
        // 如果没有提供选区信息，从选择管理器获取
        const selection = selectionRect || this.selectionManager.getSelection();
        
        if (selection && this.toolbarManager.isVisible()) {
            // 获取主工具栏位置和尺寸
            const mainToolbarRect = this.toolbarManager.toolbar.getBoundingClientRect();
            const mainToolbarPosition = {
                left: mainToolbarRect.left,
                top: mainToolbarRect.top,
                width: mainToolbarRect.width,
                height: mainToolbarRect.height
            };
            
            // 显示对应工具的参数栏，传递选区信息用于智能定位
            this.subToolbarManager.showForTool(toolName, mainToolbarPosition, selection);
        }
    }

    /**
     * 处理参数变化
     */
    handleParameterChange(toolName, paramName, value) {
        // 优先根据工具名称找到对应的工具来处理参数变化
        const targetTool = this.toolManager.getTool(toolName);
        if (targetTool && targetTool.applyParameter) {
            targetTool.applyParameter(paramName, value);
            return;
        }
        
        // 如果没找到对应工具，尝试应用到当前工具
        const currentTool = this.toolManager.getCurrentTool();
        if (currentTool && currentTool.applyParameter) {
            currentTool.applyParameter(paramName, value);
        }
        
        // 如果是公共参数，应用到编辑层管理器
        if (paramName === 'color' || paramName === 'opacity') {
            if (this.editLayerManager.applyParameter) {
                this.editLayerManager.applyParameter(paramName, value);
            }
        }
    }

    /**
     * 处理撤销操作
     */
    async handleUndo() {
        try {
            await this.editLayerManager.undo();
        } catch (error) {
            console.error('撤销操作失败:', error);
        }
    }

    /**
     * 处理重做操作
     */
    async handleRedo() {
        try {
            await this.editLayerManager.redo();
        } catch (error) {
            console.error('重做操作失败:', error);
        }
    }

    /**
     * 检查是否可以使用键盘快捷键
     */
    canUseKeyboardShortcuts() {
        // 使用工具管理器的方法检查
        if (this.toolManager && this.toolManager.canUseKeyboardShortcuts) {
            return this.toolManager.canUseKeyboardShortcuts();
        }
        
        // 传统方式检查（兼容性）
        if (this.editLayerManager && this.editLayerManager.getFabricCanvas) {
            const canvas = this.editLayerManager.getFabricCanvas();
            if (canvas) {
                const activeObject = canvas.getActiveObject();
                if (activeObject && activeObject.type === 'text' && activeObject.isEditing) {
                    return false;
                }
            }
        }
        
        return true;
    }

    /**
     * 确认截屏
     */
    async confirmScreenshot() {
        const selection = this.selectionManager.getSelection();
        if (!selection) return;
        
        try {
            this.hideAllToolbars();
            await new Promise(resolve => setTimeout(resolve, 100));
            
            // 使用导出管理器复制选区到剪贴板（自动合并编辑层）
            await this.exportManager.copySelectionToClipboard(selection);
            
            // 清空所有内容，防止下次显示时看到旧内容
            this.clearAllContent();
            
            // 关闭窗口
            await ScreenshotAPI.hideWindow();
        } catch (error) {
            console.error('截屏失败:', error);
        }
    }

    /**
     * 取消截屏
     */
    async cancelScreenshot() {
        try {
            // 清空所有内容，防止下次显示时看到旧内容
            this.clearAllContent();
            
            // 清理工具栏状态
            this.toolbarManager.setActiveTool(null);
            
            await ScreenshotAPI.hideWindow();
        } catch (error) {
            console.error('隐藏窗口失败:', error);
        }
    }

    /**
     * 清除选区
     */
    clearSelection() {
        this.selectionManager.clearSelection();
        this.disableAllTools(); // 清除选区时禁用所有工具
        this.maskManager.resetToFullscreen();
        
        // 清除选区后重新显示放大镜
        if (this.magnifierManager && this.backgroundManager?.isScreenshotLoaded) {
            this.magnifierManager.show();
        }
        
        this.eventManager.showInfoText('拖拽选择截屏区域，选区内可拖拽移动，右键取消/关闭，按 ESC 键关闭');
    }

    /**
     * 彻底清空所有内容（用于窗口隐藏时，防止下次显示旧内容）
     */
    clearAllContent() {
        try {
            // 清空编辑层（包括画布内容和历史记录）
            if (this.editLayerManager) {
                this.editLayerManager.clear();
                this.editLayerManager.clearHistory();
            }
            
            // 清空背景管理器
            if (this.backgroundManager?.clearBackground) {
                this.backgroundManager.clearBackground();
            }
            
            // 重置选区管理器
            if (this.selectionManager?.reset) {
                this.selectionManager.reset();
            }
            
            // 清空遮罩管理器
            if (this.maskManager?.clear) {
                this.maskManager.clear();
            }
            
            // 隐藏放大镜
            if (this.magnifierManager?.clear) {
                this.magnifierManager.clear();
            }
            
            // 禁用所有编辑工具
            this.disableAllTools();
            
            // 重置工具管理器（清空工具状态）
            if (this.toolManager?.clear) {
                this.toolManager.clear();
            }
            
            // 重置历史按钮状态
            if (this.toolbarManager?.resetHistoryButtons) {
                this.toolbarManager.resetHistoryButtons();
            }
            
            // 清空工具参数，防止参数残留
            if (this.subToolbarManager?.parameters?.clear) {
                this.subToolbarManager.parameters.clear();
            }
            
            // 重置事件管理器状态
            if (this.eventManager) {
                this.eventManager.showInfoText('拖拽选择截屏区域，选区内可拖拽移动，右键取消/关闭，按 ESC 键关闭');
            }
            
        } catch (error) {
            console.error('清空内容时出错:', error);
        }
    }

    /**
     * 重置状态
     */
    reset() {
        this.selectionManager.reset();
        this.disableAllTools(); // 重置状态时禁用所有工具
        this.maskManager.clear();
        this.eventManager.showInfoText('拖拽选择截屏区域，选区内可拖拽移动，右键取消/关闭，按 ESC 键关闭');
    }
}

// 初始化
let screenshotController = null;

document.addEventListener('DOMContentLoaded', () => {
    screenshotController = new ScreenshotController();
});
