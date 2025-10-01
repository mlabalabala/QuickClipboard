/**
 * 选区管理模块
 * 负责选区的创建、移动、调整等逻辑
 */

import { ScreenshotAPI } from '../api/screenshot-api.js';
import { boundsConstraint } from '../utils/bounds-constraint.js';

export class SelectionManager {
    constructor() {
        this.selectionRect = null;
        this.isSelecting = false;
        this.isMoving = false;
        this.startX = 0;
        this.startY = 0;
        this.currentX = 0;
        this.currentY = 0;
        this.moveOffsetX = 0;
        this.moveOffsetY = 0;
        
        // DOM元素
        this.selectionArea = document.getElementById('selectionArea');
        this.selectionInfo = document.getElementById('selectionInfo');
        
        // 调整大小相关
        this.isResizing = false;
        this.resizeDirection = '';
        this.resizeStartX = 0;
        this.resizeStartY = 0;
        this.resizeStartRect = null;
        
        // 圆角相关
        this.borderRadius = 0;
        this.isAdjustingRadius = false;
        this.radiusCorner = '';
        this.radiusStartX = 0;
        this.radiusStartY = 0;
        this.radiusStartValue = 0;
        
        // 缓存的显示器边界信息
        this.monitors = [];
        this.virtualBounds = null;
    }

    /**
     * 开始选择、移动或调整大小
     */
    startSelection(mouseX, mouseY, target) {
        // 检查是否点击了圆角控制节点
        if (target && target.classList.contains('radius-handle')) {
            this.isAdjustingRadius = true;
            this.isResizing = false;
            this.isSelecting = false;
            this.isMoving = false;
            this.radiusCorner = target.dataset.corner;
            this.radiusStartX = mouseX;
            this.radiusStartY = mouseY;
            this.radiusStartValue = this.borderRadius;
            return 'radius';
        }
        // 检查是否点击了拖拽节点
        else if (target && target.classList.contains('resize-handle')) {
            this.isResizing = true;
            this.isSelecting = false;
            this.isMoving = false;
            this.isAdjustingRadius = false;
            this.resizeDirection = target.dataset.direction;
            this.resizeStartX = mouseX;
            this.resizeStartY = mouseY;
            this.resizeStartRect = { ...this.selectionRect };
            return 'resize';
        }
        // 检查是否点击在现有选区内
        else if (this.selectionRect && this.isPointInSelection(mouseX, mouseY)) {
            // 在选区内：开始移动选区
            this.isMoving = true;
            this.isSelecting = false;
            this.isResizing = false;
            this.isAdjustingRadius = false;
            this.moveOffsetX = mouseX - this.selectionRect.left;
            this.moveOffsetY = mouseY - this.selectionRect.top;
            return 'move';
        } else {
            // 在选区外：开始新的选择
            this.isSelecting = true;
            this.isMoving = false;
            this.isResizing = false;
            this.isAdjustingRadius = false;
            this.borderRadius = 0; // 新选区重置圆角
            this.startX = mouseX;
            this.startY = mouseY;
            this.currentX = mouseX;
            this.currentY = mouseY;
            
            // 立即重置选区样式，防止显示旧选区
            this.selectionArea.style.left = mouseX + 'px';
            this.selectionArea.style.top = mouseY + 'px';
            this.selectionArea.style.width = '0px';
            this.selectionArea.style.height = '0px';
            this.selectionArea.style.borderRadius = '0px';
            this.selectionArea.style.display = 'block';
            document.body.classList.add('has-selection');
            return 'select';
        }
    }

    /**
     * 更新选区（选择模式）
     */
    updateSelection(mouseX, mouseY) {
        if (!this.isSelecting) return;
        
        this.currentX = mouseX;
        this.currentY = mouseY;
        this.updateDisplay();
    }

    /**
     * 设置显示器边界信息（用于前端边界检查缓存）
     */
    setMonitorBounds(monitors, virtualBounds) {
        this.monitors = monitors;
        this.virtualBounds = virtualBounds;
        // 同时设置到全局边界约束工具
        boundsConstraint.setMonitorBounds(monitors, virtualBounds);
    }

    /**
     * 移动选区（移动模式）- 前端边界检查，不调用后端
     */
    moveSelection(mouseX, mouseY, maskManager) {
        if (!this.selectionRect) return;
        
        // 计算新的选区位置
        let newLeft = mouseX - this.moveOffsetX;
        let newTop = mouseY - this.moveOffsetY;
        const { width, height } = this.selectionRect;
        
        // 使用前端边界约束
        const constrained = boundsConstraint.constrain(newLeft, newTop, width, height);
        newLeft = constrained.x;
        newTop = constrained.y;
        
        // 更新选区位置
        this.selectionRect = {
            left: newLeft,
            top: newTop,
            width: width,
            height: height
        };
        
        // 更新显示
        this.updateSelectionDisplay(newLeft, newTop, width, height);
        
        // 更新遮罩层
        maskManager.updateMask(newLeft, newTop, width, height, this.borderRadius);
    }

    /**
     * 调整选区大小（调整模式）
     */
    resizeSelection(mouseX, mouseY, maskManager) {
        if (!this.selectionRect || !this.resizeStartRect) return;
        
        const deltaX = mouseX - this.resizeStartX;
        const deltaY = mouseY - this.resizeStartY;
        
        let { left, top, width, height } = this.resizeStartRect;
        
        // 根据拖拽方向调整选区
        switch (this.resizeDirection) {
            case 'nw': // 左上
                left += deltaX;
                top += deltaY;
                width -= deltaX;
                height -= deltaY;
                break;
            case 'n': // 上
                top += deltaY;
                height -= deltaY;
                break;
            case 'ne': // 右上
                top += deltaY;
                width += deltaX;
                height -= deltaY;
                break;
            case 'e': // 右
                width += deltaX;
                break;
            case 'se': // 右下
                width += deltaX;
                height += deltaY;
                break;
            case 's': // 下
                height += deltaY;
                break;
            case 'sw': // 左下
                left += deltaX;
                width -= deltaX;
                height += deltaY;
                break;
            case 'w': // 左
                left += deltaX;
                width -= deltaX;
                break;
        }
        
        // 确保最小大小
        const minSize = 10;
        if (width < minSize) {
            if (this.resizeDirection.includes('w')) left -= minSize - width;
            width = minSize;
        }
        if (height < minSize) {
            if (this.resizeDirection.includes('n')) top -= minSize - height;
            height = minSize;
        }
        
        // 边界约束
        const constrained = boundsConstraint.constrain(left, top, width, height);
        
        // 更新选区
        this.selectionRect = {
            left: constrained.x,
            top: constrained.y,
            width: width,
            height: height
        };
        
        // 更新显示
        this.updateSelectionDisplay(constrained.x, constrained.y, width, height);
        
        // 更新遮罩层
        maskManager.updateMask(constrained.x, constrained.y, width, height, this.borderRadius);
    }

    /**
     * 调整圆角大小
     */
    adjustRadius(mouseX, mouseY, maskManager) {
        if (!this.selectionRect) return;
        
        const { left, top, width, height } = this.selectionRect;
        
        // 根据拖拽方向计算圆角变化（往选区中心方向为增大圆角）
        let delta = 0;
        switch (this.radiusCorner) {
            case 'nw':
                // 左上角：往右下（中心）拉为增大
                delta = Math.sqrt(Math.pow(mouseX - this.radiusStartX, 2) + Math.pow(mouseY - this.radiusStartY, 2));
                if (mouseX < this.radiusStartX || mouseY < this.radiusStartY) delta = -delta;
                break;
            case 'ne':
                // 右上角：往左下（中心）拉为增大
                delta = Math.sqrt(Math.pow(mouseX - this.radiusStartX, 2) + Math.pow(mouseY - this.radiusStartY, 2));
                if (mouseX > this.radiusStartX || mouseY < this.radiusStartY) delta = -delta;
                break;
            case 'se':
                // 右下角：往左上（中心）拉为增大
                delta = Math.sqrt(Math.pow(mouseX - this.radiusStartX, 2) + Math.pow(mouseY - this.radiusStartY, 2));
                if (mouseX > this.radiusStartX || mouseY > this.radiusStartY) delta = -delta;
                break;
            case 'sw':
                // 左下角：往右上（中心）拉为增大
                delta = Math.sqrt(Math.pow(mouseX - this.radiusStartX, 2) + Math.pow(mouseY - this.radiusStartY, 2));
                if (mouseX < this.radiusStartX || mouseY > this.radiusStartY) delta = -delta;
                break;
        }
        
        // 计算新的圆角值
        let newRadius = this.radiusStartValue + delta;
        
        // 限制圆角范围：0 到 选区较短边的一半
        const maxRadius = Math.min(width, height) / 2;
        newRadius = Math.max(0, Math.min(newRadius, maxRadius));
        
        this.borderRadius = Math.round(newRadius);
        
        // 更新显示
        this.updateSelectionDisplay(left, top, width, height);
        
        // 更新遮罩层
        maskManager.updateMask(left, top, width, height, this.borderRadius);
    }

    /**
     * 结束选择或移动
     */
    endSelection() {
        if (this.isMoving) {
            this.isMoving = false;
            return 'move-end';
        } else if (this.isResizing) {
            this.isResizing = false;
            this.resizeDirection = '';
            this.resizeStartRect = null;
            return 'resize-end';
        } else if (this.isAdjustingRadius) {
            this.isAdjustingRadius = false;
            this.radiusCorner = '';
            return 'radius-end';
        } else if (this.isSelecting) {
            this.isSelecting = false;
            
            const width = Math.abs(this.currentX - this.startX);
            const height = Math.abs(this.currentY - this.startY);
            
            if (width > 10 && height > 10) {
                // 确保选区信息被正确保存
                this.updateDisplay();
                return 'select-end';
            } else {
                this.reset();
                return 'select-cancel';
            }
        }
        return 'none';
    }

    /**
     * 更新选区显示
     */
    updateDisplay() {
        const left = Math.min(this.startX, this.currentX);
        const top = Math.min(this.startY, this.currentY);
        const width = Math.abs(this.currentX - this.startX);
        const height = Math.abs(this.currentY - this.startY);
        
        this.updateSelectionDisplay(left, top, width, height);
        this.selectionRect = { left, top, width, height };
    }

    /**
     * 更新选区DOM显示
     */
    updateSelectionDisplay(left, top, width, height) {
        // 更新选择区域
        this.selectionArea.style.left = left + 'px';
        this.selectionArea.style.top = top + 'px';
        this.selectionArea.style.width = width + 'px';
        this.selectionArea.style.height = height + 'px';
        this.selectionArea.style.borderRadius = this.borderRadius + 'px';
        
        // 根据圆角调整角落节点位置
        this.updateCornerHandles();
        
        // 更新信息显示
        const radiusInfo = this.borderRadius > 0 ? ` R${this.borderRadius}` : '';
        this.selectionInfo.textContent = `${Math.round(width)} × ${Math.round(height)}${radiusInfo}`;
        this.selectionInfo.style.left = '8px';
        this.selectionInfo.style.top = (top < 40 ? height + 8 : -30) + 'px';
    }
    
    /**
     * 根据圆角调整角落控制节点位置
     */
    updateCornerHandles() {
    const r = this.borderRadius;
    
    // 四个圆角控制节点
    const radiusNW = this.selectionArea.querySelector('.radius-handle-nw');
    const radiusNE = this.selectionArea.querySelector('.radius-handle-ne');
    const radiusSE = this.selectionArea.querySelector('.radius-handle-se');
    const radiusSW = this.selectionArea.querySelector('.radius-handle-sw');
    
    if (r > 0) {
        const arcPoint = r * (1 - Math.SQRT1_2); // r * 0.293，圆弧45度点坐标
        const nodeOffset = arcPoint + 12; // 节点距离角落的距离
        
        if (radiusNW) {
            radiusNW.style.left = nodeOffset + 'px';
            radiusNW.style.top = nodeOffset + 'px';
        }
        if (radiusNE) {
            radiusNE.style.right = nodeOffset + 'px';
            radiusNE.style.top = nodeOffset + 'px';
        }
        if (radiusSE) {
            radiusSE.style.right = nodeOffset + 'px';
            radiusSE.style.bottom = nodeOffset + 'px';
        }
        if (radiusSW) {
            radiusSW.style.left = nodeOffset + 'px';
            radiusSW.style.bottom = nodeOffset + 'px';
        }
    } else {
        // 无圆角时保持原来默认
        if (radiusNW) { radiusNW.style.left = '12px'; radiusNW.style.top = '12px'; }
        if (radiusNE) { radiusNE.style.right = '12px'; radiusNE.style.top = '12px'; }
        if (radiusSE) { radiusSE.style.right = '12px'; radiusSE.style.bottom = '12px'; }
        if (radiusSW) { radiusSW.style.left = '12px'; radiusSW.style.bottom = '12px'; }
    }
}


    /**
     * 检查点是否在选区内
     */
    isPointInSelection(x, y) {
        if (!this.selectionRect) return false;
        
        const { left, top, width, height } = this.selectionRect;
        return x >= left && x <= left + width && 
               y >= top && y <= top + height;
    }

    /**
     * 清除选区
     */
    clearSelection() {
        this.selectionRect = null;
        this.borderRadius = 0;
        // 清除样式，防止下次显示时闪现旧选区
        this.selectionArea.style.left = '0px';
        this.selectionArea.style.top = '0px';
        this.selectionArea.style.width = '0px';
        this.selectionArea.style.height = '0px';
        this.selectionArea.style.borderRadius = '0px';
        this.selectionArea.style.display = 'none';
        // 清除信息显示
        this.selectionInfo.textContent = '';
        document.body.classList.remove('has-selection');
    }

    /**
     * 重置状态
     */
    reset() {
        this.isSelecting = false;
        this.isMoving = false;
        this.isResizing = false;
        this.isAdjustingRadius = false;
        this.borderRadius = 0;
        this.selectionRect = null;
        // 清除样式，防止下次显示时闪现旧选区
        this.selectionArea.style.left = '0px';
        this.selectionArea.style.top = '0px';
        this.selectionArea.style.width = '0px';
        this.selectionArea.style.height = '0px';
        this.selectionArea.style.borderRadius = '0px';
        this.selectionArea.style.display = 'none';
        // 清除信息显示
        this.selectionInfo.textContent = '';
        document.body.classList.remove('has-selection');
    }
    
    /**
     * 获取圆角半径
     */
    getBorderRadius() {
        return this.borderRadius;
    }

    /**
     * 获取当前选区
     */
    getSelection() {
        return this.selectionRect;
    }

    /**
     * 获取选择状态
     */
    get isSelectingState() {
        return this.isSelecting;
    }

    /**
     * 获取移动状态  
     */
    get isMovingState() {
        return this.isMoving;
    }

    /**
     * 获取调整大小状态
     */
    get isResizingState() {
        return this.isResizing;
    }
}
