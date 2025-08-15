/**
 * 图像颜色分析工具
 * 用于提取图像的主色调并应用到UI元素
 */

/**
 * 获取图像的主色调
 */
export async function getDominantColor(imageUrl) {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    
    img.onload = function() {
      try {
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        
        // 设置较小的画布尺寸以提高性能
        const maxSize = 480;
        const ratio = Math.min(maxSize / img.width, maxSize / img.height);
        canvas.width = img.width * ratio;
        canvas.height = img.height * ratio;
        
        // 绘制缩放后的图像
        ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
        
        // 获取图像数据
        const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
        const data = imageData.data;
        
        // 颜色统计
        const colorMap = new Map();
        const step = 4; // 每隔几个像素采样一次
        
        for (let i = 0; i < data.length; i += 4 * step) {
          const r = data[i];
          const g = data[i + 1];
          const b = data[i + 2];
          const a = data[i + 3];
          
          // 跳过透明像素
          if (a < 128) continue;
          
          // 将颜色量化到较粗的级别以减少噪音
          const quantizedR = Math.floor(r / 16) * 16;
          const quantizedG = Math.floor(g / 16) * 16;
          const quantizedB = Math.floor(b / 16) * 16;
          
          const colorKey = `${quantizedR},${quantizedG},${quantizedB}`;
          colorMap.set(colorKey, (colorMap.get(colorKey) || 0) + 1);
        }
        
        // 找到出现频率最高的颜色
        let maxCount = 0;
        let dominantColor = { r: 128, g: 128, b: 128 };
        
        for (const [colorKey, count] of colorMap.entries()) {
          if (count > maxCount) {
            maxCount = count;
            const [r, g, b] = colorKey.split(',').map(Number);
            dominantColor = { r, g, b };
          }
        }
        
        // 计算亮度 (使用感知亮度公式)
        const brightness = (dominantColor.r * 0.299 + dominantColor.g * 0.587 + dominantColor.b * 0.114) / 255;
        
        resolve({
          ...dominantColor,
          brightness
        });
        
      } catch (error) {
        reject(error);
      }
    };
    
    img.onerror = () => reject(new Error('无法加载图像'));
    img.src = imageUrl;
  });
}

/**
 * 根据背景颜色生成合适的前景色
 */
export function generateTitleBarColors(backgroundColor) {
  const { r, g, b, brightness } = backgroundColor;
  
  // 基于亮度决定文字颜色
  const textColor = brightness > 0.5 ? 'rgb(0, 0, 0)' : 'rgb(255, 255, 255)';
  
  // 为标题栏背景颜色
  const alpha = 1;
  const titleBarBg = `rgba(${r}, ${g}, ${b}, ${alpha})`;
  
  // 生成边框颜色
  let borderColor;
  if (brightness > 0.5) {
    // 亮色背景用深色边框
    const factor = 0.8;
    borderColor = `rgba(${Math.floor(r * factor)}, ${Math.floor(g * factor)}, ${Math.floor(b * factor)}, 0.3)`;
  } else {
    // 暗色背景用亮色边框
    const factor = 1.3;
    borderColor = `rgba(${Math.min(255, Math.floor(r * factor))}, ${Math.min(255, Math.floor(g * factor))}, ${Math.min(255, Math.floor(b * factor))}, 0.3)`;
  }
  
  return {
    textColor,
    backgroundColor: titleBarBg,
    borderColor,
    brightness
  };
}

/**
 * 应用标题栏颜色
 */
export function applyTitleBarColors(colors) {
  // 设置CSS自定义属性
  document.documentElement.style.setProperty('--titlebar-bg-dynamic', colors.backgroundColor);
  document.documentElement.style.setProperty('--titlebar-text-dynamic', colors.textColor);
  document.documentElement.style.setProperty('--titlebar-border-dynamic', colors.borderColor);
  
  // 添加动态颜色类
  document.body.classList.add('has-dynamic-titlebar');
  
  console.log('已应用动态标题栏颜色:', colors);
}

/**
 * 移除动态标题栏颜色
 */
export function removeTitleBarColors() {
  // 移除CSS自定义属性
  document.documentElement.style.removeProperty('--titlebar-bg-dynamic');
  document.documentElement.style.removeProperty('--titlebar-text-dynamic');
  document.documentElement.style.removeProperty('--titlebar-border-dynamic');
  
  // 移除动态颜色类
  document.body.classList.remove('has-dynamic-titlebar');
  
  console.log('已移除动态标题栏颜色');
}
