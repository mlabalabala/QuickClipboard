// 文件操作模块 - 提供文件相关操作的共享逻辑

import { invoke } from '@tauri-apps/api/core';
import { showNotification } from '../../notificationManager.js';
import { getFilesData, getFirstImageFile } from '../renderers/fileRenderer.js';

// 使用默认程序打开文件
export async function openFileWithDefaultProgram(item) {
  try {
    const filesData = getFilesData(item);
    
    if (!filesData || !filesData.files || filesData.files.length === 0) {
      showNotification('未找到文件', 'error');
      return;
    }

    const firstFilePath = filesData.files[0].path;
    await invoke('open_file_with_default_program', { filePath: firstFilePath });
    showNotification('已使用默认程序打开文件', 'success');
  } catch (error) {
    console.error('打开文件失败:', error);
    showNotification('打开文件失败', 'error');
  }
}

// 打开文件位置
export async function openFileLocation(item) {
  try {
    const filesData = getFilesData(item);
    
    if (!filesData || !filesData.files || filesData.files.length === 0) {
      showNotification('未找到文件', 'error');
      return;
    }

    const firstFilePath = filesData.files[0].path;
    await invoke('open_file_location', { filePath: firstFilePath });
    showNotification('已打开文件位置', 'success');
  } catch (error) {
    console.error('打开文件位置失败:', error);
    showNotification('打开文件位置失败', 'error');
  }
}

// 复制文件路径
export async function copyFilePaths(item) {
  try {
    const filesData = getFilesData(item);
    
    if (!filesData || !filesData.files || filesData.files.length === 0) {
      showNotification('未找到文件', 'error');
      return;
    }

    const paths = filesData.files.map(file => file.path).join('\n');
    await navigator.clipboard.writeText(paths);
    showNotification(`已复制 ${filesData.files.length} 个文件路径`, 'success');
  } catch (error) {
    console.error('复制文件路径失败:', error);
    showNotification('复制文件路径失败', 'error');
  }
}

// 钉图片文件到屏幕（从文件列表中）
export async function pinImageFileFromList(item) {
  try {
    const imageFile = getFirstImageFile(item);
    
    if (!imageFile) {
      showNotification('未找到图片文件', 'error');
      return;
    }

    // 使用图片操作模块的钉图功能
    const { pinImageFileToScreen } = await import('./imageActions.js');
    await pinImageFileToScreen(imageFile);
  } catch (error) {
    console.error('钉图到屏幕失败:', error);
    showNotification('钉图失败: ' + error, 'error');
  }
}

// 检查文件是否存在并更新UI
export async function checkFilesExistence(containerSelector) {
  const fileItems = document.querySelectorAll(`${containerSelector} .file-item[data-path]`);
  for (const item of fileItems) {
    const path = item.dataset.path;
    if (path) {
      try {
        const exists = await invoke('file_exists', { path });
        if (!exists) {
          item.classList.add('file-not-exist');
        } else {
          item.classList.remove('file-not-exist');
        }
      } catch (error) {
        console.warn(`检查文件是否存在失败: ${path}`, error);
      }
    }
  }
}

