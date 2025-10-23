// 禁用浏览器默认快捷键模块


let isRecordingShortcut = false;


window.setShortcutRecording = function(recording) {
  isRecordingShortcut = recording;
  console.log('快捷键录制状态:', recording ? '开始' : '结束');
};

// 检查是否正在录制快捷键
function isCurrentlyRecording() {

  if (isRecordingShortcut) return true;

  const recordingInputs = document.querySelectorAll('.shortcut-input.recording');
  if (recordingInputs.length > 0) return true;

  const activeElement = document.activeElement;
  if (activeElement && activeElement.classList.contains('shortcut-input')) {
    return true;
  }
  
  return false;
}

// 需要禁用的快捷键组合
const DISABLED_SHORTCUTS = [
  // 浏览器导航快捷键
  { ctrl: true, key: 'f' },      // 查找
  { ctrl: true, key: 'g' },      // 查找下一个
  { ctrl: true, shift: true, key: 'g' }, // 查找上一个
  { ctrl: true, key: 'd' },      // 添加书签
  { ctrl: true, key: 'e' },      // 搜索栏
  { ctrl: true, key: 'h' },      // 历史记录
  { ctrl: true, key: 'j' },      // 下载
  { ctrl: true, key: 'k' },      // 搜索栏
  { ctrl: true, key: 'l' },      // 地址栏
  { ctrl: true, key: 'm' },      // 最小化
  { ctrl: true, key: 'n' },      // 新窗口
  { ctrl: true, key: 'o' },      // 打开文件
  { ctrl: true, key: 'p' },      // 打印
  { ctrl: true, key: 'q' },      // 退出
  { ctrl: true, key: 'r' },      // 刷新
  { ctrl: true, shift: true, key: 'r' }, // 强制刷新
  { ctrl: true, key: 't' },      // 新标签页
  { ctrl: true, key: 'u' },      // 查看源码
  { ctrl: true, key: 'w' },      // 关闭标签页
  { ctrl: true, shift: true, key: 'w' }, // 关闭窗口
  { ctrl: true, key: 'y' },      // 历史记录
  { ctrl: true, key: '=' },      // 放大
  { ctrl: true, key: '+' },      // 放大
  { ctrl: true, key: '-' },      // 缩小
  { ctrl: true, key: '0' },      // 重置缩放
  { ctrl: true, shift: true, key: 'delete' }, // 清除浏览数据
  { ctrl: true, shift: true, key: 'n' }, // 隐身窗口
  { ctrl: true, shift: true, key: 't' }, // 重新打开关闭的标签页
  { ctrl: true, shift: true, key: 'b' }, // 书签管理器
  { ctrl: true, shift: true, key: 'o' }, // 书签管理器
  { ctrl: true, shift: true, key: 'h' }, // 历史记录页面
  { ctrl: true, shift: true, key: 'y' }, // 历史记录页面
  
  // 开发者工具
  { key: 'F12' },                // 开发者工具
  { ctrl: true, shift: true, key: 'i' }, // 开发者工具
  { ctrl: true, shift: true, key: 'j' }, // 控制台
  { ctrl: true, shift: true, key: 'c' }, // 元素选择器
  
  // 标签页切换
  { ctrl: true, key: 'Tab' },    // 切换标签页
  { ctrl: true, shift: true, key: 'Tab' }, // 反向切换标签页
  { ctrl: true, key: 'PageUp' }, // 上一个标签页
  { ctrl: true, key: 'PageDown' }, // 下一个标签页
  
  // 数字键标签页切换
  { ctrl: true, key: '1' },
  { ctrl: true, key: '2' },
  { ctrl: true, key: '3' },
  { ctrl: true, key: '4' },
  { ctrl: true, key: '5' },
  { ctrl: true, key: '6' },
  { ctrl: true, key: '7' },
  { ctrl: true, key: '8' },
  { ctrl: true, key: '9' },
  
  // 功能键
  { key: 'F1' },                 // 帮助
  { key: 'F3' },                 // 查找下一个
  { key: 'F5' },                 // 刷新
  { ctrl: true, key: 'F5' },     // 强制刷新
  { key: 'F6' },                 // 地址栏
  { key: 'F11' },                // 全屏
  
  // Alt 组合键
  { alt: true, key: 'ArrowLeft' }, // 后退
  { alt: true, key: 'ArrowRight' }, // 前进
  { alt: true, key: 'Home' },     // 首页
  { alt: true, key: 'd' },        // 地址栏
  { alt: true, key: 'f' },        // 文件菜单
  
  // 其他
  { key: 'Escape', target: 'body' },
];

// 检查按键组合是否匹配
function matchesShortcut(event, shortcut) {
  const ctrlMatch = shortcut.ctrl ? event.ctrlKey : !event.ctrlKey;
  const altMatch = shortcut.alt ? event.altKey : !event.altKey;
  const shiftMatch = shortcut.shift ? event.shiftKey : !event.shiftKey;
  const keyMatch = shortcut.key.toLowerCase() === event.key.toLowerCase() || 
                   shortcut.key.toLowerCase() === event.code.toLowerCase();
  
  return ctrlMatch && altMatch && shiftMatch && keyMatch;
}

// 全局键盘事件处理器
function handleKeyDown(event) {
  if (isCurrentlyRecording()) {
    return;
  }
  
  // 检查是否匹配需要禁用的快捷键
  for (const shortcut of DISABLED_SHORTCUTS) {
    if (matchesShortcut(event, shortcut)) {
      const isInputElement = event.target.tagName === 'INPUT' || 
                             event.target.tagName === 'TEXTAREA' || 
                             event.target.contentEditable === 'true';

      if (isInputElement) {
        const allowedInInput = [
          'ctrl+a', 'ctrl+c', 'ctrl+v', 'ctrl+x', 'ctrl+z', 'ctrl+y',
          'ctrl+s' 
        ];
        const currentShortcut = `${event.ctrlKey ? 'ctrl+' : ''}${event.altKey ? 'alt+' : ''}${event.shiftKey ? 'shift+' : ''}${event.key.toLowerCase()}`;
        
        if (allowedInInput.includes(currentShortcut)) {
          continue;
        }
      }
      
      const isEditorContext = document.body.classList.contains('editor-context') || 
                              document.querySelector('.editor-container') !== null;
      
      if (isEditorContext && (event.ctrlKey && event.key.toLowerCase() === 's')) {
        continue;
      }
      
      event.preventDefault();
      event.stopPropagation();
      return false;
    }
  }
}

// 初始化浏览器快捷键禁用
export function initDisableBrowserShortcuts() {
  document.addEventListener('keydown', handleKeyDown, true);
  document.addEventListener('contextmenu', (event) => {
    event.preventDefault();
  });
  
  document.addEventListener('dragstart', (event) => {
    event.preventDefault();
  });
  
  document.addEventListener('selectstart', (event) => {
    if (event.target.tagName !== 'INPUT' && 
        event.target.tagName !== 'TEXTAREA' && 
        event.target.contentEditable !== 'true') {
    }
  });
  
  console.log('浏览器默认快捷键已禁用');
}

export function removeDisableBrowserShortcuts() {
  document.removeEventListener('keydown', handleKeyDown, true);
  console.log('浏览器默认快捷键禁用已移除');
}
