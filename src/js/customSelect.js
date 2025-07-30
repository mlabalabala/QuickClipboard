/**
 * 自定义下拉选择组件
 */

export class CustomSelect {
  constructor(container, options = {}) {
    this.container = container;
    this.options = options.options || [];
    this.value = options.value || '';
    this.placeholder = options.placeholder || '请选择';
    this.onChange = options.onChange || (() => { });

    this.isOpen = false;
    this.selectedIndex = -1;

    this.init();
  }

  init() {
    this.render();
    this.bindEvents();
    this.updateSelected();
  }

  render() {
    this.container.innerHTML = `
      <div class="custom-select">
        <div class="custom-select-trigger">
          <span class="custom-select-text">${this.placeholder}</span>
          <span class="custom-select-arrow"><i class="ti ti-chevron-down"></i></span>
        </div>
        <div class="custom-select-dropdown">
          ${this.options.map((option, index) => `
            <div class="custom-select-option" data-value="${option.value}" data-index="${index}">
              ${option.text}
            </div>
          `).join('')}
        </div>
      </div>
    `;

    this.trigger = this.container.querySelector('.custom-select-trigger');
    this.dropdown = this.container.querySelector('.custom-select-dropdown');
    this.textElement = this.container.querySelector('.custom-select-text');
    this.optionElements = this.container.querySelectorAll('.custom-select-option');
  }

  bindEvents() {
    // 点击触发器切换下拉状态
    this.trigger.addEventListener('click', (e) => {
      e.stopPropagation();
      this.toggle();
    });

    // 点击选项
    this.optionElements.forEach(option => {
      option.addEventListener('click', (e) => {
        e.stopPropagation();
        const value = option.dataset.value;
        const index = parseInt(option.dataset.index);
        this.select(value, index);
      });
    });

    // 点击外部关闭下拉
    document.addEventListener('click', (e) => {
      if (!this.container.contains(e.target)) {
        this.close();
      }
    });

    // 键盘导航
    this.container.addEventListener('keydown', (e) => {
      this.handleKeydown(e);
    });

    // 使触发器可聚焦
    this.trigger.setAttribute('tabindex', '0');
  }

  handleKeydown(e) {
    switch (e.key) {
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (this.isOpen) {
          if (this.selectedIndex >= 0) {
            const option = this.options[this.selectedIndex];
            this.select(option.value, this.selectedIndex);
          }
        } else {
          this.open();
        }
        break;

      case 'Escape':
        e.preventDefault();
        this.close();
        break;

      case 'ArrowDown':
        e.preventDefault();
        if (!this.isOpen) {
          this.open();
        } else {
          this.navigateDown();
        }
        break;

      case 'ArrowUp':
        e.preventDefault();
        if (this.isOpen) {
          this.navigateUp();
        }
        break;
    }
  }

  navigateDown() {
    this.selectedIndex = Math.min(this.selectedIndex + 1, this.options.length - 1);
    this.updateHighlight();
  }

  navigateUp() {
    this.selectedIndex = Math.max(this.selectedIndex - 1, 0);
    this.updateHighlight();
  }

  updateHighlight() {
    this.optionElements.forEach((option, index) => {
      option.classList.toggle('highlighted', index === this.selectedIndex);
    });
  }

  toggle() {
    if (this.isOpen) {
      this.close();
    } else {
      this.open();
    }
  }

  open() {
    this.isOpen = true;
    this.trigger.classList.add('active');
    this.dropdown.classList.add('active');

    // 设置当前选中项的高亮
    const currentIndex = this.options.findIndex(option => option.value === this.value);
    this.selectedIndex = currentIndex >= 0 ? currentIndex : 0;
    this.updateHighlight();
  }

  close() {
    this.isOpen = false;
    this.trigger.classList.remove('active');
    this.dropdown.classList.remove('active');
    this.selectedIndex = -1;
    this.updateHighlight();
  }

  select(value, index) {
    this.value = value;
    this.updateSelected();
    this.close();
    this.onChange(value);
  }

  updateSelected() {
    const selectedOption = this.options.find(option => option.value === this.value);
    if (selectedOption) {
      this.textElement.textContent = selectedOption.text;
    } else {
      this.textElement.textContent = this.placeholder;
    }

    // 更新选中状态
    this.optionElements.forEach(option => {
      option.classList.toggle('selected', option.dataset.value === this.value);
    });
  }

  setValue(value) {
    this.value = value;
    this.updateSelected();
  }

  getValue() {
    return this.value;
  }

  setOptions(options) {
    this.options = options;
    this.render();
    this.bindEvents();
    this.updateSelected();
  }

  destroy() {
    // 清理事件监听器
    document.removeEventListener('click', this.documentClickHandler);
    this.container.innerHTML = '';
  }
}

// 添加高亮样式到CSS
const style = document.createElement('style');
style.textContent = `
  .custom-select-option.highlighted {
    background-color: #f0f7ff !important;
    color: #4a89dc;
  }

  [data-theme="dark"] .custom-select-option.highlighted {
    background-color: #1a3a5c !important;
    color: #6bb6ff;
  }
`;
document.head.appendChild(style);
