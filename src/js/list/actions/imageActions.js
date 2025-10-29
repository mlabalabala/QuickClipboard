// 图片操作模块 - 提供图片相关操作的共享逻辑

import { invoke } from '@tauri-apps/api/core';
import { showNotification } from '../../notificationManager.js';

// 钉图片到屏幕
export async function pinImageToScreen(item) {
  try {
    // 获取图片文件路径
    const filePath = await invoke('get_image_file_path', { 
      content: item.content 
    });
    
    if (!filePath) {
      showNotification('获取图片路径失败', 'error');
      return;
    }
    
    // 创建贴图窗口
    await invoke('pin_image_from_file', { 
      filePath 
    });
    
    showNotification('已钉到屏幕', 'success', 2000);
  } catch (error) {
    console.error('钉图到屏幕失败:', error);
    showNotification('钉图失败: ' + error, 'error');
  }
}

// 另存为图片
export async function saveImageAs(item) {
  try {
    if (!item.content.startsWith('data:image/') && !item.content.startsWith('image:')) {
      showNotification('此图片格式暂不支持直接保存', 'info');
      return;
    }

    // 使用文件对话框选择保存位置
    const { save } = await import('@tauri-apps/plugin-dialog');
    const filePath = await save({
      title: '保存图片',
      defaultPath: `image_${Date.now()}.png`,
      filters: [{
        name: '图片文件',
        extensions: ['png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp']
      }]
    });

    if (!filePath) {
      return; // 用户取消了操作
    }

    // 调用后端保存图片
    await invoke('save_image_to_file', {
      content: item.content,
      filePath: filePath
    });

    showNotification('图片已保存', 'success');
  } catch (error) {
    console.error('保存图片失败:', error);
    showNotification('保存图片失败', 'error');
  }
}

// 钉图片文件到屏幕
export async function pinImageFileToScreen(imageFile) {
  try {
    if (!imageFile || !imageFile.path) {
      showNotification('未找到图片文件', 'error');
      return;
    }

    await invoke('pin_image_from_file', { 
      filePath: imageFile.path 
    });
    showNotification('已钉到屏幕', 'success', 2000);
  } catch (error) {
    console.error('钉图到屏幕失败:', error);
    showNotification('钉图失败: ' + error, 'error');
  }
}

