/**
 * 工具栏管理模块
 * 负责工具栏的显示、隐藏、定位等逻辑
 */

import { boundsConstraint } from '../utils/bounds-constraint.js';

export class ToolbarManager {
    constructor() {
        this.toolbar = document.getElementById('toolbar');
        this.confirmBtn = document.getElementById('confirmBtn');
        this.cancelBtn = document.getElementById('cancelBtn');
        this.brushBtn = document.getElementById('brushBtn');
        
        // 工具栏尺寸 (更新为包含新按钮的尺寸)
        this.toolbarWidth = 32 * 3 + 4 * 3 + 4 + 5; // 画笔 + 分隔符 + 确认 + 取消
        this.toolbarHeight = 32 + 4 * 2; // 40px
        
        this.currentTool = null;
        
        this.initEvents();
    }

    initEvents() {
        this.confirmBtn.addEventListener('click', () => this.onConfirm?.());
        this.cancelBtn.addEventListener('click', () => this.onCancel?.());
        
        // 通用工具按钮事件处理
        const toolButtons = this.toolbar.querySelectorAll('.tool-btn');
        toolButtons.forEach(btn => {
            btn.addEventListener('click', (e) => {
                const toolName = e.currentTarget.dataset.tool;
                if (toolName) {
                    this.handleToolClick(toolName);
                }
            });
        });
    }

    /**
     * 显示工具栏
     */
    show(selectionRect) {
        if (!selectionRect) return;
        
        const { left, top, width, height } = selectionRect;
        
        // 计算工具栏位置：选区右下角，右对齐
        let toolbarLeft = left + width - this.toolbarWidth;
        let toolbarTop = top + height + 8; // 选区下方8px
        
        // 使用前端边界约束（同步、快速）
        const constrainedBounds = boundsConstraint.constrain(
            toolbarLeft, toolbarTop, this.toolbarWidth, this.toolbarHeight
        );
        
        toolbarLeft = constrainedBounds.x;
        toolbarTop = constrainedBounds.y;
        
        // 如果约束后的位置与预期差距太大，说明下方空间不足，尝试上方
        if (toolbarTop < top + height + 4) {
            const upperToolbarTop = top - this.toolbarHeight - 8;
            const upperBounds = boundsConstraint.constrain(
                left + width - this.toolbarWidth, upperToolbarTop,
                this.toolbarWidth, this.toolbarHeight
            );
            
            // 如果上方位置更合适，使用上方
            if (upperBounds.y >= upperToolbarTop - 4) {
                toolbarLeft = upperBounds.x;
                toolbarTop = upperBounds.y;
            }
        }
        
        // 设置工具栏位置并显示
        this.toolbar.style.left = toolbarLeft + 'px';
        this.toolbar.style.top = toolbarTop + 'px';
        this.toolbar.classList.add('visible');
    }

    /**
     * 隐藏工具栏
     */
    hide() {
        this.toolbar.classList.remove('visible');
    }

    /**
     * 设置确认回调
     */
    setOnConfirm(callback) {
        this.onConfirm = callback;
    }

    /**
     * 设置取消回调
     */
    setOnCancel(callback) {
        this.onCancel = callback;
    }

    /**
     * 设置工具选择回调
     */
    setOnToolSelect(callback) {
        this.onToolSelect = callback;
    }

    /**
     * 处理工具按钮点击
     */
    handleToolClick(toolName) {
        // 切换工具状态
        if (this.currentTool === toolName) {
            // 取消当前工具
            this.setActiveTool(null);
        } else {
            // 激活新工具
            this.setActiveTool(toolName);
        }

        // 调用回调
        if (this.onToolSelect) {
            this.onToolSelect(this.currentTool);
        }
    }

    /**
     * 设置激活的工具
     */
    setActiveTool(toolName) {
        // 清除所有工具按钮的激活状态
        const toolButtons = this.toolbar.querySelectorAll('.tool-btn');
        toolButtons.forEach(btn => btn.classList.remove('active'));

        this.currentTool = toolName;

        // 设置当前工具按钮为激活状态
        if (toolName) {
            const activeButton = this.toolbar.querySelector(`[data-tool="${toolName}"]`);
            if (activeButton) {
                activeButton.classList.add('active');
            }
        }
    }

    /**
     * 获取当前激活的工具
     */
    getCurrentTool() {
        return this.currentTool;
    }

    /**
     * 检查工具栏是否可见
     */
    isVisible() {
        return this.toolbar.classList.contains('visible');
    }
}
