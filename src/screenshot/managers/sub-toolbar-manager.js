/**
 * 子工具栏管理器
 * 负责管理各种工具的参数设置界面
 */

import { boundsConstraint } from '../utils/bounds-constraint.js';

export class SubToolbarManager {
    constructor() {
        this.subToolbar = null;
        this.currentTool = null;
        this.parameters = new Map(); // 存储各工具的参数值
        this.callbacks = new Map(); // 参数变化回调
        this.colorPicker = null; // 颜色选择面板
        this.colorPickerClickHandler = null; // 颜色面板点击处理器
        
        // 工具参数配置
        this.toolConfigs = {
            // 公共参数（所有工具都有）
            common: {
                color: {
                    type: 'color',
                    label: '颜色',
                    default: '#ff0000',
                    icon: 'ti ti-palette'
                },
                opacity: {
                    type: 'slider',
                    label: '透明度',
                    default: 100,
                    min: 0,
                    max: 100,
                    step: 1,
                    unit: '%',
                    icon: 'ti ti-adjustments'
                }
            },
            
            // 画笔工具参数
            brush: {
                brushSize: {
                    type: 'slider',
                    label: '笔刷大小',
                    default: 5,
                    min: 1,
                    max: 50,
                    step: 1,
                    unit: 'px',
                    icon: 'ti ti-circle'
                },
                brushType: {
                    type: 'select',
                    label: '笔刷类型',
                    default: 'pencil',
                    options: [
                        { value: 'pencil', label: '铅笔', icon: 'ti ti-pencil' },
                        { value: 'brush', label: '画笔', icon: 'ti ti-brush' },
                        { value: 'marker', label: '马克笔', icon: 'ti ti-highlight' }
                    ],
                    icon: 'ti ti-brush'
                }
            },
            
            // 文本工具参数
            text: {
                fontSize: {
                    type: 'slider',
                    label: '字体大小',
                    default: 16,
                    min: 8,
                    max: 72,
                    step: 1,
                    unit: 'px',
                    icon: 'ti ti-typography'
                },
                fontFamily: {
                    type: 'select',
                    label: '字体',
                    default: 'Arial',
                    options: [
                        { value: 'Arial', label: 'Arial' },
                        { value: 'Microsoft YaHei', label: '微软雅黑' },
                        { value: 'SimHei', label: '黑体' },
                        { value: 'SimSun', label: '宋体' }
                    ],
                    icon: 'ti ti-typography'
                },
                fontWeight: {
                    type: 'toggle',
                    label: '粗体',
                    default: false,
                    icon: 'ti ti-bold'
                },
                fontStyle: {
                    type: 'toggle',
                    label: '斜体',
                    default: false,
                    icon: 'ti ti-italic'
                }
            },
            
            // 形状工具参数（矩形、圆形、箭头共用）
            shape: {
                strokeWidth: {
                    type: 'slider',
                    label: '边框粗细',
                    default: 2,
                    min: 0,
                    max: 20,
                    step: 1,
                    unit: 'px',
                    icon: 'ti ti-line-height'
                },
                filled: {
                    type: 'toggle',
                    label: '填充',
                    default: false,
                    icon: 'ti ti-color-fill'
                },
                fillColor: {
                    type: 'color',
                    label: '填充颜色',
                    default: '#ff0000',
                    icon: 'ti ti-palette',
                    dependsOn: 'filled' // 依赖填充开关
                }
            }
        };
        
        this.initParameters();
    }

    /**
     * 初始化默认参数值
     */
    initParameters() {
        for (const [toolName, config] of Object.entries(this.toolConfigs)) {
            this.parameters.set(toolName, {});
            for (const [paramName, paramConfig] of Object.entries(config)) {
                this.parameters.get(toolName)[paramName] = paramConfig.default;
            }
        }
    }

    /**
     * 创建子工具栏DOM结构
     */
    createSubToolbar() {
        if (this.subToolbar) return;
        
        this.subToolbar = document.createElement('div');
        this.subToolbar.className = 'sub-toolbar';
        this.subToolbar.id = 'subToolbar';
        
        document.body.appendChild(this.subToolbar);
    }

    /**
     * 显示指定工具的参数工具栏
     */
    showForTool(toolName, mainToolbarPosition, selectionRect = null) {
        if (!toolName || toolName === 'selection') {
            this.hide();
            return;
        }
        
        this.currentTool = toolName;
        this.createSubToolbar();
        this.renderToolParameters(toolName);
        this.positionSubToolbar(mainToolbarPosition, selectionRect);
        this.subToolbar.classList.add('visible');
    }

    /**
     * 隐藏子工具栏
     */
    hide() {
        if (this.subToolbar) {
            this.subToolbar.classList.remove('visible');
        }
        this.currentTool = null;
    }

    /**
     * 渲染工具参数
     */
    renderToolParameters(toolName) {
        if (!this.subToolbar) return;
        
        // 获取工具配置（公共参数 + 工具特定参数）
        const commonConfig = this.toolConfigs.common || {};
        
        // 形状工具（rectangle、circle、arrow）都使用 'shape' 配置
        let toolConfigKey = toolName;
        if (['rectangle', 'circle', 'arrow'].includes(toolName)) {
            toolConfigKey = 'shape';
        }
        
        const toolConfig = this.toolConfigs[toolConfigKey] || {};
        
        // 合并配置
        const allConfig = { ...commonConfig, ...toolConfig };
        
        // 清空现有内容
        this.subToolbar.innerHTML = '';
        
        // 渲染参数控件
        for (const [paramName, paramConfig] of Object.entries(allConfig)) {
            const paramElement = this.createParameterElement(toolName, paramName, paramConfig);
            this.subToolbar.appendChild(paramElement);
        }
    }

    /**
     * 创建参数控件元素
     */
    createParameterElement(toolName, paramName, config) {
        const wrapper = document.createElement('div');
        wrapper.className = 'param-item';
        
        // 检查依赖条件
        if (config.dependsOn) {
            const dependValue = this.getParameter(toolName, config.dependsOn);
            if (!dependValue) {
                wrapper.style.display = 'none';
            }
            wrapper.dataset.dependsOn = config.dependsOn;
        }
        
        // 根据参数类型创建不同的控件
        switch (config.type) {
            case 'color':
                wrapper.appendChild(this.createColorPicker(toolName, paramName, config));
                break;
            case 'slider':
                wrapper.appendChild(this.createSlider(toolName, paramName, config));
                break;
            case 'select':
                wrapper.appendChild(this.createSelect(toolName, paramName, config));
                break;
            case 'toggle':
                wrapper.appendChild(this.createToggle(toolName, paramName, config));
                break;
        }
        
        return wrapper;
    }

    /**
     * 创建颜色选择器
     */
    createColorPicker(toolName, paramName, config) {
        const button = document.createElement('button');
        button.className = 'param-color';
        button.title = config.label;
        
        const currentValue = this.getParameter(toolName, paramName);
        button.style.backgroundColor = currentValue;
        
        // 颜色选择器图标
        button.innerHTML = `<i class="${config.icon}"></i>`;
        
        // 点击事件
        button.addEventListener('click', () => {
            this.showColorPicker(toolName, paramName, button);
        });
        
        return button;
    }

    /**
     * 创建滑块控件
     */
    createSlider(toolName, paramName, config) {
        const container = document.createElement('div');
        container.className = 'param-slider-container';
        
        // 图标
        const icon = document.createElement('i');
        icon.className = config.icon;
        container.appendChild(icon);
        
        // 滑块
        const slider = document.createElement('input');
        slider.type = 'range';
        slider.className = 'param-slider';
        slider.min = config.min;
        slider.max = config.max;
        slider.step = config.step;
        slider.value = this.getParameter(toolName, paramName);
        
        // 数值显示
        const valueDisplay = document.createElement('span');
        valueDisplay.className = 'param-value';
        valueDisplay.textContent = slider.value + (config.unit || '');
        
        // 滑块变化事件
        slider.addEventListener('input', () => {
            const value = parseFloat(slider.value);
            valueDisplay.textContent = value + (config.unit || '');
            this.setParameter(toolName, paramName, value);
        });
        
        container.appendChild(slider);
        container.appendChild(valueDisplay);
        
        return container;
    }

    /**
     * 创建选择器
     */
    createSelect(toolName, paramName, config) {
        const button = document.createElement('button');
        button.className = 'param-select';
        button.title = config.label;
        
        const currentValue = this.getParameter(toolName, paramName);
        const currentOption = config.options.find(opt => opt.value === currentValue);
        
        if (currentOption && currentOption.icon) {
            button.innerHTML = `<i class="${currentOption.icon}"></i>`;
        } else {
            button.innerHTML = `<i class="${config.icon}"></i>`;
        }
        
        // 点击事件 - 循环切换选项
        button.addEventListener('click', () => {
            const currentIndex = config.options.findIndex(opt => opt.value === currentValue);
            const nextIndex = (currentIndex + 1) % config.options.length;
            const nextOption = config.options[nextIndex];
            
            this.setParameter(toolName, paramName, nextOption.value);
            
            // 更新图标
            if (nextOption.icon) {
                button.innerHTML = `<i class="${nextOption.icon}"></i>`;
            }
        });
        
        return button;
    }

    /**
     * 创建开关按钮
     */
    createToggle(toolName, paramName, config) {
        const button = document.createElement('button');
        button.className = 'param-toggle';
        button.title = config.label;
        
        const currentValue = this.getParameter(toolName, paramName);
        if (currentValue) {
            button.classList.add('active');
        }
        
        button.innerHTML = `<i class="${config.icon}"></i>`;
        
        // 点击事件
        button.addEventListener('click', () => {
            const newValue = !this.getParameter(toolName, paramName);
            this.setParameter(toolName, paramName, newValue);
            
            if (newValue) {
                button.classList.add('active');
            } else {
                button.classList.remove('active');
            }
            
            // 检查依赖关系
            this.updateDependentParameters(toolName, paramName);
        });
        
        return button;
    }

    /**
     * 定位子工具栏（智能根据主工具栏相对选区的位置调整）
     */
    positionSubToolbar(mainToolbarPosition, selectionRect = null) {
        if (!this.subToolbar || !mainToolbarPosition) return;
        
        const subToolbarRect = this.subToolbar.getBoundingClientRect();
        const gap = 2;
        
        // 子工具栏与主工具栏右对齐
        let subToolbarLeft = mainToolbarPosition.left + mainToolbarPosition.width - subToolbarRect.width;
        let subToolbarTop;
        
        // 根据主工具栏相对于选区的位置决定子工具栏位置
        if (selectionRect) {
            const selectionBottom = selectionRect.top + selectionRect.height;
            
            // 检测主工具栏是在选区上方还是下方
            if (mainToolbarPosition.top < selectionBottom) {
                // 主工具栏在选区上方，子工具栏应该在主工具栏上方
                subToolbarTop = mainToolbarPosition.top - subToolbarRect.height - gap;
            } else {
                // 主工具栏在选区下方，子工具栏在主工具栏下方
                subToolbarTop = mainToolbarPosition.top + mainToolbarPosition.height + gap;
            }
        } else {
            // 没有选区信息，默认在主工具栏下方
            subToolbarTop = mainToolbarPosition.top + mainToolbarPosition.height + gap;
        }
        
        // 使用通用边界约束
        const constrainedBounds = boundsConstraint.constrain(
            subToolbarLeft, subToolbarTop, subToolbarRect.width, subToolbarRect.height
        );
        
        subToolbarLeft = constrainedBounds.x;
        subToolbarTop = constrainedBounds.y;
        
        // 如果边界约束改变了位置太多，尝试另一边
        const expectedTop = selectionRect && mainToolbarPosition.top < selectionRect.top + selectionRect.height 
            ? mainToolbarPosition.top - subToolbarRect.height - gap
            : mainToolbarPosition.top + mainToolbarPosition.height + gap;
            
        if (Math.abs(subToolbarTop - expectedTop) > gap + 2) {
            // 原位置不可行，尝试另一边
            const alternativeTop = selectionRect && mainToolbarPosition.top < selectionRect.top + selectionRect.height
                ? mainToolbarPosition.top + mainToolbarPosition.height + gap  // 改为下方
                : mainToolbarPosition.top - subToolbarRect.height - gap;       // 改为上方
                
            const alternativeBounds = boundsConstraint.constrain(
                mainToolbarPosition.left + mainToolbarPosition.width - subToolbarRect.width,
                alternativeTop,
                subToolbarRect.width,
                subToolbarRect.height
            );
            
            // 如果另一边的位置更合适，使用另一边
            if (Math.abs(alternativeBounds.y - alternativeTop) <= gap + 2) {
                subToolbarLeft = alternativeBounds.x;
                subToolbarTop = alternativeBounds.y;
            }
        }
        
        // 应用最终位置
        this.subToolbar.style.left = subToolbarLeft + 'px';
        this.subToolbar.style.top = subToolbarTop + 'px';
    }

    /**
     * 显示颜色选择器
     */
    showColorPicker(toolName, paramName, button) {
        // 如果已经有颜色面板，先关闭
        this.hideColorPicker();
        
        // 创建颜色选择面板
        this.colorPicker = this.createColorPickerPanel(toolName, paramName, button);
        document.body.appendChild(this.colorPicker);
        
        // 定位颜色面板
        this.positionColorPicker(button);
        
        // 显示面板
        requestAnimationFrame(() => {
            this.colorPicker.classList.add('visible');
        });
        
        // 点击外部关闭面板
        this.colorPickerClickHandler = (e) => {
            if (!this.colorPicker.contains(e.target) && !button.contains(e.target)) {
                this.hideColorPicker();
            }
        };
        document.addEventListener('click', this.colorPickerClickHandler);
    }

    /**
     * 创建颜色选择面板
     */
    createColorPickerPanel(toolName, paramName, button) {
        const panel = document.createElement('div');
        panel.className = 'color-picker-panel';
        
        // 常用颜色预设
        const presetColors = [
            '#ff0000', '#ff8800', '#ffff00', '#88ff00', '#00ff00', '#00ff88',
            '#00ffff', '#0088ff', '#0000ff', '#8800ff', '#ff00ff', '#ff0088',
            '#000000', '#333333', '#666666', '#999999', '#cccccc', '#ffffff'
        ];
        
        // 预设颜色区域
        const presetsContainer = document.createElement('div');
        presetsContainer.className = 'color-presets';
        
        const presetsTitle = document.createElement('div');
        presetsTitle.className = 'color-section-title';
        presetsTitle.textContent = '常用颜色';
        presetsContainer.appendChild(presetsTitle);
        
        const presetsGrid = document.createElement('div');
        presetsGrid.className = 'color-presets-grid';
        
        presetColors.forEach(color => {
            const colorItem = document.createElement('div');
            colorItem.className = 'color-preset-item';
            colorItem.style.backgroundColor = color;
            colorItem.title = color;
            
            colorItem.addEventListener('click', () => {
                this.selectColor(toolName, paramName, button, color);
                this.hideColorPicker();
            });
            
            presetsGrid.appendChild(colorItem);
        });
        
        presetsContainer.appendChild(presetsGrid);
        panel.appendChild(presetsContainer);
        
        // 自定义颜色区域
        const customContainer = document.createElement('div');
        customContainer.className = 'color-custom';
        
        const customTitle = document.createElement('div');
        customTitle.className = 'color-section-title';
        customTitle.textContent = '自定义颜色';
        customContainer.appendChild(customTitle);
        
        const customInput = document.createElement('input');
        customInput.type = 'color';
        customInput.className = 'color-custom-input';
        customInput.value = this.getParameter(toolName, paramName) || '#ff0000';
        
        customInput.addEventListener('change', () => {
            this.selectColor(toolName, paramName, button, customInput.value);
        });
        
        customContainer.appendChild(customInput);
        panel.appendChild(customContainer);
        
        // 透明度控制（如果是透明度参数的话）
        if (paramName === 'opacity') {
            const opacityContainer = document.createElement('div');
            opacityContainer.className = 'color-opacity';
            
            const opacityTitle = document.createElement('div');
            opacityTitle.className = 'color-section-title';
            opacityTitle.textContent = '透明度';
            opacityContainer.appendChild(opacityTitle);
            
            const opacitySlider = document.createElement('input');
            opacitySlider.type = 'range';
            opacitySlider.className = 'color-opacity-slider';
            opacitySlider.min = 0;
            opacitySlider.max = 100;
            opacitySlider.value = this.getParameter(toolName, 'opacity') || 100;
            
            const opacityValue = document.createElement('span');
            opacityValue.className = 'color-opacity-value';
            opacityValue.textContent = opacitySlider.value + '%';
            
            opacitySlider.addEventListener('input', () => {
                const opacity = parseInt(opacitySlider.value);
                opacityValue.textContent = opacity + '%';
                this.setParameter(toolName, 'opacity', opacity);
            });
            
            opacityContainer.appendChild(opacitySlider);
            opacityContainer.appendChild(opacityValue);
            panel.appendChild(opacityContainer);
        }
        
        return panel;
    }

    /**
     * 定位颜色选择面板
     */
    positionColorPicker(button) {
        if (!this.colorPicker) return;
        
        const buttonRect = button.getBoundingClientRect();
        const panelRect = this.colorPicker.getBoundingClientRect();
        
        // 优先在按钮下方显示
        let left = buttonRect.left;
        let top = buttonRect.bottom + 4;
        
        // 确保不超出屏幕边界
        if (left + panelRect.width > window.innerWidth) {
            left = window.innerWidth - panelRect.width - 8;
        }
        if (left < 8) {
            left = 8;
        }
        
        if (top + panelRect.height > window.innerHeight) {
            // 下方空间不足，改为上方
            top = buttonRect.top - panelRect.height - 4;
        }
        if (top < 8) {
            top = 8;
        }
        
        this.colorPicker.style.left = left + 'px';
        this.colorPicker.style.top = top + 'px';
    }

    /**
     * 选择颜色
     */
    selectColor(toolName, paramName, button, color) {
        this.setParameter(toolName, paramName, color);
        button.style.backgroundColor = color;
        
        // 更新按钮显示的颜色
        const currentColor = color.toLowerCase();
        if (currentColor === '#ffffff' || currentColor === '#fff') {
            button.style.borderColor = 'rgba(255, 255, 255, 0.5)';
        } else {
            button.style.borderColor = 'rgba(255, 255, 255, 0.3)';
        }
    }

    /**
     * 隐藏颜色选择器
     */
    hideColorPicker() {
        if (this.colorPicker && this.colorPicker.parentNode) {
            this.colorPicker.parentNode.removeChild(this.colorPicker);
            this.colorPicker = null;
        }
        
        if (this.colorPickerClickHandler) {
            document.removeEventListener('click', this.colorPickerClickHandler);
            this.colorPickerClickHandler = null;
        }
    }

    /**
     * 更新依赖参数的显示状态
     */
    updateDependentParameters(toolName, changedParam) {
        if (!this.subToolbar) return;
        
        const dependentItems = this.subToolbar.querySelectorAll(`[data-depends-on="${changedParam}"]`);
        const paramValue = this.getParameter(toolName, changedParam);
        
        dependentItems.forEach(item => {
            if (paramValue) {
                item.style.display = '';
            } else {
                item.style.display = 'none';
            }
        });
    }

    /**
     * 获取参数值
     */
    getParameter(toolName, paramName) {
        // 形状工具都使用 'shape' 参数
        let paramKey = toolName;
        if (['rectangle', 'circle', 'arrow'].includes(toolName)) {
            paramKey = 'shape';
        }
        
        // 先查找工具特定参数，再查找公共参数
        const toolParams = this.parameters.get(paramKey) || {};
        const commonParams = this.parameters.get('common') || {};
        
        return toolParams[paramName] !== undefined ? 
               toolParams[paramName] : commonParams[paramName];
    }

    /**
     * 设置参数值
     */
    setParameter(toolName, paramName, value) {
        // 确定参数属于哪个类别
        const isCommonParam = this.toolConfigs.common && this.toolConfigs.common[paramName];
        
        if (isCommonParam) {
            // 公共参数
            if (!this.parameters.has('common')) {
                this.parameters.set('common', {});
            }
            this.parameters.get('common')[paramName] = value;
        } else {
            // 工具特定参数 - 形状工具都使用 'shape' 参数存储
            let paramKey = toolName;
            if (['rectangle', 'circle', 'arrow'].includes(toolName)) {
                paramKey = 'shape';
            }
            
            if (!this.parameters.has(paramKey)) {
                this.parameters.set(paramKey, {});
            }
            this.parameters.get(paramKey)[paramName] = value;
        }
        
        // 更新依赖参数的显示状态
        this.updateDependentParameters(toolName, paramName);
        
        // 触发回调
        this.triggerParameterChange(toolName, paramName, value);
    }

    /**
     * 获取工具的所有参数
     */
    getToolParameters(toolName) {
        const commonParams = this.parameters.get('common') || {};
        
        // 形状工具（rectangle、circle、arrow）都使用 'shape' 参数
        let paramKey = toolName;
        if (['rectangle', 'circle', 'arrow'].includes(toolName)) {
            paramKey = 'shape';
        }
        
        const toolParams = this.parameters.get(paramKey) || {};
        
        return { ...commonParams, ...toolParams };
    }

    /**
     * 设置参数变化回调
     */
    onParameterChange(callback) {
        this.callbacks.set('global', callback);
    }

    /**
     * 触发参数变化回调
     */
    triggerParameterChange(toolName, paramName, value) {
        const globalCallback = this.callbacks.get('global');
        if (globalCallback) {
            globalCallback(toolName, paramName, value);
        }
    }

    /**
     * 销毁子工具栏
     */
    destroy() {
        // 清理颜色选择器
        this.hideColorPicker();
        
        // 清理子工具栏
        if (this.subToolbar && this.subToolbar.parentNode) {
            this.subToolbar.parentNode.removeChild(this.subToolbar);
            this.subToolbar = null;
        }
        
        // 清理其他资源
        this.currentTool = null;
        this.parameters.clear();
        this.callbacks.clear();
    }
}
