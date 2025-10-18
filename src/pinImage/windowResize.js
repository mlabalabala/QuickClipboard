/**
 * 窗口缩放模块
 * 处理窗口大小的调整
 */

import { LogicalSize } from '@tauri-apps/api/window';

/**
 * 初始化窗口大小
 */
export async function initWindowSize(window, state) {
    const currentSize = await window.innerSize();
    const scaleFactor = await window.scaleFactor();
    state.initialSize = {
        width: currentSize.width / scaleFactor,
        height: currentSize.height / scaleFactor
    };
}

/**
 * 处理窗口缩放
 */
export async function handleWindowResize(delta, isShiftKey, window, state) {
    if (!state.initialSize) {
        await initWindowSize(window, state);
    }
    
    const step = isShiftKey ? 5 : 1;
    
    if (delta < 0) {
        state.scaleLevel += step;
    } else {
        state.scaleLevel = Math.max(1, state.scaleLevel - step);
    }
    
    const newWidth = state.initialSize.width * (state.scaleLevel / 10);
    const newHeight = state.initialSize.height * (state.scaleLevel / 10);
    
    await window.setSize(new LogicalSize(newWidth, newHeight));

    if (state.imageScale > 1) {
        state.imageScale = 1;
        state.imageX = 0;
        state.imageY = 0;
    }
    
    return { width: newWidth, height: newHeight };
}

