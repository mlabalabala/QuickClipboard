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
    updateMask(left, top, width, height, borderRadius = 0) {
        const w = this.screenWidth;
        const h = this.screenHeight;
        const right = left + width;
        const bottom = top + height;
        
        if (borderRadius > 0) {
            const r = borderRadius;
            
            // 根据圆角大小动态调整点数：圆角越大，点数越多
            // 小圆角(r<30): 5个点；中圆角(30-100): 8个点；大圆角(>100): 12个点
            let pointsPerCorner;
            if (r < 30) {
                pointsPerCorner = 5;
            } else if (r < 100) {
                pointsPerCorner = 8;
            } else {
                pointsPerCorner = 12;
            }
            
            const points = [];
            
            // 外框（顺时针）
            points.push(`0 0`, `${w}px 0`, `${w}px ${h}px`, `0 ${h}px`, `0 0`);
            
            // 内框圆角矩形（逆时针）
            // 上边
            points.push(`${left + r}px ${top}px`, `${right - r}px ${top}px`);
            
            // 右上圆角
            for (let i = 0; i <= pointsPerCorner; i++) {
                const angle = (Math.PI / 2) * (i / pointsPerCorner);
                const x = right - r + r * Math.sin(angle);
                const y = top + r - r * Math.cos(angle);
                points.push(`${x}px ${y}px`);
            }
            
            // 右边
            points.push(`${right}px ${bottom - r}px`);
            
            // 右下圆角
            for (let i = 0; i <= pointsPerCorner; i++) {
                const angle = (Math.PI / 2) * (i / pointsPerCorner);
                const x = right - r + r * Math.cos(angle);
                const y = bottom - r + r * Math.sin(angle);
                points.push(`${x}px ${y}px`);
            }
            
            // 下边
            points.push(`${left + r}px ${bottom}px`);
            
            // 左下圆角
            for (let i = 0; i <= pointsPerCorner; i++) {
                const angle = (Math.PI / 2) * (i / pointsPerCorner);
                const x = left + r - r * Math.sin(angle);
                const y = bottom - r + r * Math.cos(angle);
                points.push(`${x}px ${y}px`);
            }
            
            // 左边
            points.push(`${left}px ${top + r}px`);
            
            // 左上圆角
            for (let i = 0; i <= pointsPerCorner; i++) {
                const angle = (Math.PI / 2) * (i / pointsPerCorner);
                const x = left + r - r * Math.cos(angle);
                const y = top + r - r * Math.sin(angle);
                points.push(`${x}px ${y}px`);
            }
            
            this.maskLayer.style.clipPath = `polygon(evenodd,${points.join(',')})`;
        } else {
            // 无圆角时，使用简单矩形
            this.maskLayer.style.clipPath = 
                `polygon(evenodd,0 0,${w}px 0,${w}px ${h}px,0 ${h}px,0 0,${left}px ${top}px,${left}px ${bottom}px,${right}px ${bottom}px,${right}px ${top}px,${left}px ${top}px)`;
        }
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
