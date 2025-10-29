// 上下文菜单构建器 - 统一构建剪贴板和常用文本的右键菜单

import { showContextMenu } from '../contextMenu.js';
import { checkIfHasImageFile } from './renderers/fileRenderer.js';

// 构建图片类型菜单项
function buildImageMenuItems(item, actions) {
  const items = [
    {
      icon: 'ti-pin',
      text: '钉到屏幕',
      onClick: () => actions.pinImage(item)
    },
    {
      icon: 'ti-download',
      text: '另存为图片',
      onClick: () => actions.saveImage(item)
    }
  ];

  // 如果有删除回调，添加删除选项
  if (actions.delete) {
    items.push({
      icon: 'ti-trash',
      text: '删除',
      style: { color: '#ff4d4f' },
      onClick: () => actions.delete(item)
    });
  }

  return items;
}

// 构建文件类型菜单项
function buildFileMenuItems(item, actions) {
  const items = [];
  
  // 如果包含图片文件，添加钉图选项
  if (checkIfHasImageFile(item)) {
    items.push({
      icon: 'ti-pin',
      text: '钉到屏幕',
      onClick: () => actions.pinImageFile(item)
    });
  }
  
  items.push(
    {
      icon: 'ti-external-link',
      text: '使用默认程序打开',
      onClick: () => actions.openFile(item)
    },
    {
      icon: 'ti-folder-open',
      text: '打开文件位置',
      onClick: () => actions.openFileLocation(item)
    },
    {
      icon: 'ti-copy',
      text: '复制文件路径',
      onClick: () => actions.copyFilePath(item)
    }
  );

  // 如果有删除回调，添加删除选项
  if (actions.delete) {
    items.push({
      icon: 'ti-trash',
      text: '删除',
      style: { color: '#ff4d4f' },
      onClick: () => actions.delete(item)
    });
  }

  return items;
}

// 构建文本类型菜单项
function buildTextMenuItems(item, actions, contentType) {
  const items = [];

  // 如果有编辑回调，添加编辑选项
  if (actions.edit) {
    items.push({
      icon: 'ti-edit',
      text: contentType === 'rich_text' ? '编辑纯文本' : '编辑',
      onClick: () => actions.edit(item)
    });
  }

  // 如果有删除回调，添加删除选项
  if (actions.delete) {
    items.push({
      icon: 'ti-trash',
      text: '删除',
      style: { color: '#ff4d4f' },
      onClick: () => actions.delete(item)
    });
  }

  return items;
}

// 构建剪贴板项的上下文菜单
export function buildClipboardContextMenu(event, item, actions) {
  const contentType = item.content_type || 'text';
  let menuItems = [];

  // 根据内容类型构建特定菜单
  if (contentType === 'image') {
    menuItems = buildImageMenuItems(item, {
      pinImage: actions.pinImage,
      saveImage: actions.saveImage
    });
  } else if (contentType === 'file') {
    menuItems = buildFileMenuItems(item, {
      pinImageFile: actions.pinImageFile,
      openFile: actions.openFile,
      openFileLocation: actions.openFileLocation,
      copyFilePath: actions.copyFilePath
    });
  } else if (contentType === 'text' || contentType === 'link' || contentType === 'rich_text') {
    menuItems = buildTextMenuItems(item, {
      edit: actions.edit
    }, contentType);
  }

  // 添加通用菜单项
  menuItems.push(
    {
      icon: 'ti-star',
      text: '添加到常用文本',
      onClick: () => actions.addToFavorites(item)
    },
    {
      icon: 'ti-trash',
      text: '删除当前项',
      onClick: () => actions.deleteItem(item)
    },
    { type: 'separator' },
    {
      icon: 'ti-trash-x',
      text: '清空剪贴板',
      style: { color: '#ff4d4f' },
      onClick: () => actions.clearAll()
    }
  );

  showContextMenu(event, {
    content: item.content,
    html_content: item.html_content,
    content_type: contentType,
    items: menuItems
  });
}

// 构建常用文本项的上下文菜单
export function buildQuickTextContextMenu(event, item, actions) {
  const contentType = item.content_type || 'text';
  let menuItems = [];

  // 根据内容类型构建特定菜单
  if (contentType === 'image') {
    menuItems = buildImageMenuItems(item, {
      pinImage: actions.pinImage,
      saveImage: actions.saveImage,
      delete: actions.delete
    });
  } else if (contentType === 'file') {
    menuItems = buildFileMenuItems(item, {
      pinImageFile: actions.pinImageFile,
      openFile: actions.openFile,
      openFileLocation: actions.openFileLocation,
      copyFilePath: actions.copyFilePath,
      delete: actions.delete
    });
  } else {
    menuItems = buildTextMenuItems(item, {
      edit: actions.edit,
      delete: actions.delete
    }, contentType);
  }

  showContextMenu(event, {
    content: item.content,
    html_content: item.html_content,
    content_type: contentType,
    items: menuItems
  });
}

