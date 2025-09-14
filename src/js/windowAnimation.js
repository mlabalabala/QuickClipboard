// ==================== 窗口动画模块 ====================
// 负责处理窗口显示/隐藏的真实高度变化动画

/**
 * 播放窗口显示动画 - JavaScript控制的高度动画
 */
export async function playWindowShowAnimation() {
  const container = document.querySelector('body');
  if (!container) return;

  // 标记收到了真正的动画事件，阻止回退动画
  container.style.animation = 'none';
  
  // 添加动画进行中的标记
  container.classList.add('js-animating');
  
  // 获取侧边栏相关元素和状态
  const groupsSidebar = document.querySelector('.groups-sidebar');
  const isRightTitlebar = container.classList.contains('titlebar-right');
  
  // 确保初始状态
  container.style.height = '0';
  container.style.maxHeight = '0';
  container.style.opacity = '0';
  container.style.overflow = 'hidden';
  
  // 动画开始时无条件隐藏侧边栏（所有模式）
  if (groupsSidebar) {
    groupsSidebar.style.visibility = 'hidden';
  }
  
  // 强制重绘
  container.offsetHeight;
  
  // 执行展开动画
  await animateHeightExpand(container);
  
  // 移除动画标记
  container.classList.remove('js-animating');
}

/**
 * 播放窗口隐藏动画 - JavaScript控制的高度动画
 */
export async function playWindowHideAnimation() {
  const container = document.querySelector('body');
  if (!container) return;

  // 标记收到了真正的动画事件，阻止回退动画
  container.style.animation = 'none';
  
  // 添加动画进行中的标记
  container.classList.add('js-animating');
  
  // 获取侧边栏相关元素和状态
  const groupsSidebar = document.querySelector('.groups-sidebar');
  const isRightTitlebar = container.classList.contains('titlebar-right');
  
  // 确保初始状态
  container.style.height = '100vh';
  container.style.maxHeight = '100vh';
  container.style.opacity = '1';
  container.style.overflow = 'hidden';
  
  // 动画开始时无条件隐藏侧边栏（所有模式）
  if (groupsSidebar) {
    groupsSidebar.style.visibility = 'hidden';
  }
  
  // 强制重绘
  container.offsetHeight;
  
  // 执行收起动画
  await animateHeightCollapse(container);
  
  // 移除动画标记
  container.classList.remove('js-animating');
}

/**
 * 高度展开动画 - 从上到下展开
 * @param {HTMLElement} container - 要动画的容器元素
 */
async function animateHeightExpand(container) {
  const footer = document.querySelector('.footer');
  const groupsSidebar = document.querySelector('.groups-sidebar');
  const isRightTitlebar = container.classList.contains('titlebar-right');
  
  return new Promise((resolve) => {
    const duration = 300; // 300ms 动画时长
    const startTime = performance.now();
    const targetHeight = window.innerHeight; // 100vh 的像素值
    
    function animate(currentTime) {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);
      
      // 使用 ease-out 缓动函数
      const eased = 1 - Math.pow(1 - progress, 3);
      
      // 计算当前高度
      const currentHeight = targetHeight * eased;
      
      // 设置容器样式
      container.style.height = `${currentHeight}px`;
      container.style.maxHeight = `${currentHeight}px`;
      container.style.opacity = eased;
      
      // 同步底部栏位置（从窗口底部向上移动）
      if (footer) {
        const footerOffset = targetHeight - currentHeight;
        footer.style.transform = `translateY(${footerOffset}px)`;
        footer.style.opacity = eased;
      }
      
      // 侧边栏在动画期间保持隐藏状态
      
      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        // 动画完成，设置最终状态
        container.style.height = '100vh';
        container.style.maxHeight = '100vh';
        container.style.opacity = '1';
        container.style.overflow = 'hidden'; // 保持布局
        
        // 重置底部栏位置
        if (footer) {
          footer.style.transform = 'translateY(0)';
          footer.style.opacity = '1';
        }
        
        // 恢复分组侧边栏的可见性和位置（所有模式）
        if (groupsSidebar) {
          groupsSidebar.style.visibility = 'visible';
          // 清除可能在动画中设置的内联样式，让CSS规则生效
          groupsSidebar.style.right = '';
          groupsSidebar.style.transform = '';
        }
        
        resolve();
      }
    }
    
    requestAnimationFrame(animate);
  });
}

/**
 * 高度收起动画 - 从下到上收起
 * @param {HTMLElement} container - 要动画的容器元素
 */
async function animateHeightCollapse(container) {
  const footer = document.querySelector('.footer');
  const groupsSidebar = document.querySelector('.groups-sidebar');
  const isRightTitlebar = container.classList.contains('titlebar-right');
  
  return new Promise((resolve) => {
    const duration = 200; // 200ms 动画时长
    const startTime = performance.now();
    const startHeight = window.innerHeight; // 100vh 的像素值
    
    function animate(currentTime) {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);
      
      // 使用 ease-in 缓动函数
      const eased = Math.pow(progress, 2);
      
      // 计算当前高度（从完整到0）
      const currentHeight = startHeight * (1 - eased);
      
      // 设置容器样式
      container.style.height = `${currentHeight}px`;
      container.style.maxHeight = `${currentHeight}px`;
      container.style.opacity = 1 - eased;
      
      // 同步底部栏位置（向窗口底部移动）
      if (footer) {
        const footerOffset = startHeight - currentHeight;
        footer.style.transform = `translateY(${footerOffset}px)`;
        footer.style.opacity = 1 - eased;
      }
      
      // 侧边栏在动画期间保持隐藏状态
      
      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        // 动画完成，设置最终状态
        container.style.height = '0';
        container.style.maxHeight = '0';
        container.style.opacity = '0';
        
        // 重置底部栏位置
        if (footer) {
          footer.style.transform = 'translateY(0)';
          footer.style.opacity = '0';
        }
        
        // 分组侧边栏在隐藏动画结束后保持隐藏（整个窗口都要隐藏了）
        
        resolve();
      }
    }
    
    requestAnimationFrame(animate);
  });
}

/**
 * 设置窗口动画监听器
 */
export async function setupWindowAnimationListeners() {
  try {
    // console.log('开始设置窗口动画监听器...');
    const { listen } = await import('@tauri-apps/api/event');

    // 监听窗口显示动画事件
    await listen('window-show-animation', () => {
      // console.log('收到窗口显示动画事件');
      playWindowShowAnimation();
    });

    // 监听窗口隐藏动画事件
    await listen('window-hide-animation', () => {
      // console.log('收到窗口隐藏动画事件');
      playWindowHideAnimation();
    });

    // console.log('窗口动画监听器设置完成');
    
    // 直接恢复贴边隐藏状态，不再需要前端动画监听器
    await restoreEdgeSnapOnStartup();
  } catch (error) {
    console.error('设置窗口动画监听器失败:', error);
  }
}

/**
 * 恢复贴边隐藏状态
 */
async function restoreEdgeSnapOnStartup() {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('restore_edge_snap_on_startup');
  } catch (error) {
    console.error('恢复贴边隐藏状态失败:', error);
  }
}

/**
 * 确保动画安全回退机制
 */
export function setupAnimationFallback() {
  // 安全回退：确保界面在200ms后可见（如果没有收到动画事件）
  setTimeout(() => {
    const container = document.querySelector('body');
    if (container && !container.classList.contains('js-animating')) {
      // 如果没有动画在进行，说明可能没收到事件，直接显示
      container.style.height = '100vh';
      container.style.maxHeight = '100vh';
      container.style.opacity = '1';
      container.style.animation = 'none';
    }
  }, 200);
}
