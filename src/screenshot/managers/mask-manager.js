/**
 * 遮罩层管理模块
 * 负责半透明遮罩层的显示和更新
 */

export class MaskManager {
    constructor() {
        this.maskTop = document.getElementById('maskTop');
        this.maskBottom = document.getElementById('maskBottom');
        this.maskLeft = document.getElementById('maskLeft');
        this.maskRight = document.getElementById('maskRight');
    }

    /**
     * 更新遮罩层
     */
    updateMask(left, top, width, height) {
        const right = left + width;
        const bottom = top + height;
        const screenWidth = window.innerWidth;
        const screenHeight = window.innerHeight;
        
        // 上遮罩
        this.maskTop.style.cssText = `
            left: 0; 
            top: 0; 
            width: ${screenWidth}px; 
            height: ${Math.max(0, top)}px; 
            position: absolute; 
            background: rgba(0, 0, 0, 0.5);
        `;
        
        // 下遮罩
        this.maskBottom.style.cssText = `
            left: 0; 
            top: ${Math.min(bottom, screenHeight)}px; 
            width: ${screenWidth}px; 
            height: ${Math.max(0, screenHeight - bottom)}px; 
            position: absolute; 
            background: rgba(0, 0, 0, 0.5);
        `;
        
        // 左遮罩
        this.maskLeft.style.cssText = `
            left: 0; 
            top: ${Math.max(0, top)}px; 
            width: ${Math.max(0, left)}px; 
            height: ${Math.max(0, Math.min(height, screenHeight - top))}px; 
            position: absolute; 
            background: rgba(0, 0, 0, 0.5);
        `;
        
        // 右遮罩
        this.maskRight.style.cssText = `
            left: ${Math.min(right, screenWidth)}px; 
            top: ${Math.max(0, top)}px; 
            width: ${Math.max(0, screenWidth - right)}px; 
            height: ${Math.max(0, Math.min(height, screenHeight - top))}px; 
            position: absolute; 
            background: rgba(0, 0, 0, 0.5);
        `;
    }

    /**
     * 重置遮罩层为全屏状态
     */
    resetToFullscreen() {
        this.maskTop.style.cssText = 'top: 0; left: 0; width: 100%; height: 100%;';
        this.maskBottom.style.cssText = 'display: none;';
        this.maskLeft.style.cssText = 'display: none;';
        this.maskRight.style.cssText = 'display: none;';
    }

    /**
     * 清理所有遮罩
     */
    clear() {
        [this.maskTop, this.maskBottom, this.maskLeft, this.maskRight].forEach(mask => {
            mask.style.cssText = '';
        });
    }
}
