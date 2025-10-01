/**
 * 遮罩层管理
 * 用 clip-path 实现镂空
 */

export class MaskManager {
    constructor() {
        this.maskLayer = document.getElementById('maskLayer');
        this.screenWidth = window.innerWidth;
        this.screenHeight = window.innerHeight;
        
        // 窗口变化时更新尺寸
        window.addEventListener('resize', () => {
            this.screenWidth = window.innerWidth;
            this.screenHeight = window.innerHeight;
        });
    }

    // 根据选区更新遮罩镂空区域
    updateMask(left, top, width, height) {
        const right = left + width;
        const bottom = top + height;
        const w = this.screenWidth;
        const h = this.screenHeight;
        
        // polygon evenodd 规则：外框顺时针 + 内框逆时针 = 镂空效果
        // 用像素值比百分比快，减少计算开销
        this.maskLayer.style.clipPath = 
            `polygon(evenodd,0 0,${w}px 0,${w}px ${h}px,0 ${h}px,0 0,${left}px ${top}px,${left}px ${bottom}px,${right}px ${bottom}px,${right}px ${top}px,${left}px ${top}px)`;
    }

    // 自定义形状遮罩（圆形、椭圆等）
    updateMaskWithCustomShape(clipPathValue) {
        this.maskLayer.style.clipPath = clipPathValue;
    }

    // 重置为全屏遮罩
    resetToFullscreen() {
        this.maskLayer.style.clipPath = 'polygon(0% 0%, 100% 0%, 100% 100%, 0% 100%)';
    }

    // 清除遮罩
    clear() {
        this.maskLayer.style.clipPath = '';
    }

    // 调整遮罩透明度 (0-1)
    setOpacity(opacity) {
        const currentOpacity = parseFloat(opacity);
        if (currentOpacity >= 0 && currentOpacity <= 1) {
            this.maskLayer.style.background = `rgba(0, 0, 0, ${currentOpacity})`;
        }
    }

    // 修改遮罩颜色
    setColor(color) {
        this.maskLayer.style.background = color;
    }
}
