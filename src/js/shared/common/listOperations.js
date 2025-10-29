// 列表通用操作模块

import { invoke } from '@tauri-apps/api/core';
import { showNotification } from '../../notificationManager.js';

// 粘贴内容（通用）
export async function pasteContent(params, options = {}) {
  const {
    element = null,
    onSuccess = null,
    onError = null
  } = options;

  try {
    if (element) element.classList.add('paste-loading');
    showNotification('正在粘贴...', 'info');

    // 调用后端统一粘贴接口
    await invoke('paste_content', { params });

    if (element) element.classList.remove('paste-loading');
    showNotification('粘贴成功', 'success', 1500);

    if (onSuccess) {
      await onSuccess();
    }
  } catch (error) {
    console.error('粘贴失败:', error);
    if (element) element.classList.remove('paste-loading');
    showNotification('粘贴失败', 'error', 2000);

    if (onError) {
      await onError(error);
    }
  }
}

// 删除项目（通用）
export async function deleteItem(id, invokeMethod, onSuccess) {
  try {
    await invoke(invokeMethod, { id });
    showNotification('项目已删除', 'success');
    
    if (onSuccess) {
      await onSuccess();
    }
  } catch (error) {
    console.error('删除项目失败:', error);
    showNotification('删除失败', 'error');
  }
}

// 打开文本编辑器
export async function openTextEditor(editorData) {
  try {
    await invoke('open_text_editor_window');

    // 延迟发送数据，确保窗口已完全加载
    setTimeout(async () => {
      try {
        const { emit } = await import('@tauri-apps/api/event');
        await emit('editor-data', editorData);
      } catch (error) {
        console.error('发送编辑数据失败:', error);
        showNotification('打开编辑器失败', 'error');
      }
    }, 500);

  } catch (error) {
    console.error('打开文本编辑器失败:', error);
    showNotification('打开编辑器失败', 'error');
  }
}

// 添加到常用文本
export async function addToFavorites(id) {
  try {
    await invoke('add_clipboard_to_favorites', { id });
    showNotification('已添加到常用文本', 'success');
    await invoke('emit_quick_texts_updated');
  } catch (error) {
    console.error('添加到常用文本失败:', error);
    showNotification('添加失败', 'error');
    throw error;
  }
}

