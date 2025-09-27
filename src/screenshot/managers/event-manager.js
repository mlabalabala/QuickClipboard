/**
 * 事件管理模块
 * 负责键盘和鼠标事件的处理和分发
 */

export class EventManager {
    constructor() {
        this.overlay = document.getElementById('overlay');
        this.selectionArea = document.getElementById('selectionArea');
        this.infoText = document.getElementById('infoText');
        
        this.onSelectionStart = null;
        this.onSelectionUpdate = null;
        this.onSelectionEnd = null;
        this.onRightClick = null;
        this.onKeyDown = null;
        this.onWindowFocus = null;
        this.onWindowBlur = null;
        
        this.initEvents();
    }

    initEvents() {
        // 鼠标事件
        this.overlay.addEventListener('mousedown', (e) => this.handleMouseDown(e));
        this.selectionArea.addEventListener('mousedown', (e) => this.handleMouseDown(e));
        document.addEventListener('mousemove', (e) => this.handleMouseMove(e));
        document.addEventListener('mouseup', (e) => this.handleMouseUp(e));
        
        // 键盘事件
        document.addEventListener('keydown', (e) => this.handleKeyDown(e));
        
        // 右键事件
        document.addEventListener('contextmenu', (e) => this.handleRightClick(e));
        
        // 窗口焦点事件
        window.addEventListener('focus', () => this.onWindowFocus?.());
        window.addEventListener('blur', () => this.onWindowBlur?.());
    }

    handleMouseDown(e) {
        if (e.button !== 0) return; // 只处理左键
        
        const mouseX = e.clientX;
        const mouseY = e.clientY;
        
        // 如果点击的是拖拽节点，需要特殊处理
        if (e.target.classList.contains('resize-handle')) {
            e.preventDefault();
            // 不调用 stopPropagation，让拖拽节点事件正常处理
        } else {
            e.preventDefault();
            e.stopPropagation();
        }
        
        // 传递目标元素给选区管理器
        this.onSelectionStart?.(mouseX, mouseY, e.target);
    }

    handleMouseMove(e) {
        const mouseX = e.clientX;
        const mouseY = e.clientY;
        
        this.onSelectionUpdate?.(mouseX, mouseY);
    }

    handleMouseUp(e) {
        if (e.button !== 0) return; // 只处理左键
        
        this.onSelectionEnd?.();
    }

    handleKeyDown(e) {
        if (e.key === 'Escape') {
            e.preventDefault();
            this.onKeyDown?.('escape');
        } else if (e.key === 'Enter') {
            e.preventDefault();
            this.onKeyDown?.('enter');
        } else if (e.ctrlKey || e.metaKey) {
            // 处理Ctrl/Cmd组合键
            if (e.key.toLowerCase() === 'z') {
                e.preventDefault();
                if (e.shiftKey) {
                    this.onKeyDown?.('ctrl+shift+z'); // 重做
                } else {
                    this.onKeyDown?.('ctrl+z'); // 撤销
                }
            } else if (e.key.toLowerCase() === 'y') {
                e.preventDefault();
                this.onKeyDown?.('ctrl+y'); // 重做
            }
        }
    }

    handleRightClick(e) {
        e.preventDefault();
        
        const mouseX = e.clientX;
        const mouseY = e.clientY;
        
        this.onRightClick?.(mouseX, mouseY);
    }

    /**
     * 设置选择开始回调
     */
    setOnSelectionStart(callback) {
        this.onSelectionStart = callback;
    }

    /**
     * 设置选择更新回调
     */
    setOnSelectionUpdate(callback) {
        this.onSelectionUpdate = callback;
    }

    /**
     * 设置选择结束回调
     */
    setOnSelectionEnd(callback) {
        this.onSelectionEnd = callback;
    }

    /**
     * 设置右键回调
     */
    setOnRightClick(callback) {
        this.onRightClick = callback;
    }

    /**
     * 设置键盘回调
     */
    setOnKeyDown(callback) {
        this.onKeyDown = callback;
    }

    /**
     * 设置窗口焦点回调
     */
    setOnWindowFocus(callback) {
        this.onWindowFocus = callback;
    }

    /**
     * 设置窗口失焦回调
     */
    setOnWindowBlur(callback) {
        this.onWindowBlur = callback;
    }

    /**
     * 显示信息文本
     */
    showInfoText(text) {
        this.infoText.textContent = text;
        this.infoText.style.opacity = '1';
    }

    /**
     * 隐藏信息文本
     */
    hideInfoText() {
        this.infoText.style.opacity = '0';
    }
}
