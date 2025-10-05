/**
 * 贴图窗口
 */

import { Menu, CheckMenuItem, MenuItem, PredefinedMenuItem } from '@tauri-apps/api/menu';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';

// 创建右键菜单
async function createContextMenu(window, shadowState) {
    document.addEventListener('contextmenu', async (e) => {
        e.preventDefault();
        
        // 获取当前置顶状态
        const isOnTop = await window.isAlwaysOnTop();
        
        // 创建勾选菜单项
        const alwaysOnTopItem = await CheckMenuItem.new({
            id: 'toggle-top',
            text: '窗口置顶',
            checked: isOnTop,
            action: async () => {
                try {
                    await window.setAlwaysOnTop(!isOnTop);
                } catch (error) {
                    console.error('切换置顶失败:', error);
                }
            }
        });
        
        const shadowItem = await CheckMenuItem.new({
            id: 'toggle-shadow',
            text: '窗口阴影',
            checked: shadowState.enabled,
            action: async () => {
                try {
                    shadowState.enabled = !shadowState.enabled;
                    await window.setShadow(shadowState.enabled);
                } catch (error) {
                    console.error('切换阴影失败:', error);
                }
            }
        });
        
        const separator1 = await PredefinedMenuItem.new({
            item: 'Separator'
        });
        
        const copyItem = await MenuItem.new({
            id: 'copy',
            text: '复制到剪贴板',
            action: async () => {
                try {
                    await invoke('copy_pin_image_to_clipboard');
                } catch (error) {
                    console.error('复制到剪贴板失败:', error);
                }
            }
        });
        
        const saveAsItem = await MenuItem.new({
            id: 'save-as',
            text: '图像另存为...',
            action: async () => {
                try {
                    await invoke('save_pin_image_as');
                } catch (error) {
                    console.error('保存图片失败:', error);
                }
            }
        });
        
        const separator2 = await PredefinedMenuItem.new({
            item: 'Separator'
        });
        
        const closeItem = await MenuItem.new({
            id: 'close',
            text: '关闭窗口',
            action: async () => {
                try {
                    await invoke('close_pin_image_window_by_self');
                } catch (error) {
                    console.error('关闭窗口失败:', error);
                }
            }
        });
        
        // 创建菜单
        const menu = await Menu.new({
            items: [alwaysOnTopItem, shadowItem, separator1, copyItem, saveAsItem, separator2, closeItem]
        });
        
        await menu.popup();
    });
}

(async () => {
    const img = document.getElementById('pinImage');
    const currentWindow = getCurrentWindow();
    
    let mouseDown = false;
    let hasMoved = false;
    
    const shadowState = { enabled: false };
    
    img.addEventListener('selectstart', e => e.preventDefault()); // 阻止选中
    img.addEventListener('dragstart', e => e.preventDefault());   // 阻止拖拽
    
    // 创建右键菜单
    await createContextMenu(currentWindow, shadowState);
    
    // 按下鼠标
    img.addEventListener('mousedown', (e) => {
        e.preventDefault(); 
        if (e.button === 0) {
            mouseDown = true;
            hasMoved = false;
        }
    });
    
    // 鼠标移动时才开始拖拽
    img.addEventListener('mousemove', (e) => {
        if (mouseDown && !hasMoved) {
            hasMoved = true;
            currentWindow.startDragging();
        }
    });
    
    // 鼠标释放
    img.addEventListener('mouseup', () => {
        mouseDown = false;
    });
    
    // 双击关闭窗口
    img.addEventListener('dblclick', async (e) => {
        e.preventDefault(); // 阻止默认行为
        try {
            await invoke('close_pin_image_window_by_self');
        } catch (error) {
            console.error('关闭窗口失败:', error);
        }
    });
    
    try {
        // 请求图片数据
        const data = await invoke('get_pin_image_data');
        
        if (data && data.file_path) {
            // 转换为 asset 协议 URL
            const assetUrl = convertFileSrc(data.file_path, 'asset');
            img.src = assetUrl;
        }
    } catch (error) {
        console.error('加载图片失败:', error);
    }
})();
