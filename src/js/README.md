## JavaScript 模块结构

### 核心模块

#### 1. `config.js` - 配置和全局状态管理
- 全局变量和状态管理
- DOM元素引用
- 应用配置
- 状态更新函数

#### 2. `clipboard.js` - 剪贴板功能
- 剪贴板读写操作
- 剪贴板历史管理
- 剪贴板项目渲染
- 内容类型检测
- 剪贴板排序功能

#### 3. `quickTexts.js` - 常用文本管理
- 常用文本的CRUD操作
- 常用文本渲染
- 模态框管理
- 常用文本排序功能

#### 4. `groups.js` - 分组管理
- 剪贴板项目分组功能
- 分组的创建、编辑、删除
- 分组切换和过滤
- 分组数据管理

### 设置和配置模块

#### 5. `settings.js` - 设置界面管理
- 设置页面的UI控制
- 设置项的绑定和验证
- 设置界面的导航和切换
- 快捷键设置处理

#### 6. `settingsManager.js` - 设置数据管理
- 设置的加载、保存和应用
- 主题管理
- 本地存储操作
- 设置数据的同步

#### 7. `screenshot.js` - 截屏功能
- 截屏设置管理
- 截屏快捷键处理
- 截屏质量和格式配置
- 截屏文件管理

### UI和交互模块

#### 8. `ui.js` - 用户界面组件
- 通知系统
- 模态框管理（确认框、提示框）
- 标签页切换
- 通用UI功能

#### 9. `events.js` - 事件处理
- 键盘快捷键
- 剪贴板事件监听
- 托盘事件监听
- 窗口拖拽事件
- 右键菜单禁用

#### 10. `sortable.js` - 拖拽排序
- 剪贴板列表拖拽排序
- 常用文本列表拖拽排序
- 分组列表拖拽排序
- Sortable.js 集成

#### 11. `focus.js` - 焦点管理
- 输入框焦点管理
- 窗口焦点控制
- 工具窗口模式

#### 12. `window.js` - 窗口控制
- 窗口固定状态管理
- 窗口控制按钮
- 窗口状态同步

## 模块间依赖关系

```
主窗口 (main.js)
├── 核心模块
│   ├── config.js (配置管理)
│   ├── clipboard.js (依赖: config.js, ui.js, groups.js)
│   ├── quickTexts.js (依赖: config.js, clipboard.js, ui.js)
│   └── groups.js (依赖: config.js, ui.js)
│
├── 设置模块
│   ├── settings.js (依赖: config.js, settingsManager.js)
│   ├── settingsManager.js (依赖: config.js)
│   └── screenshot.js (依赖: config.js, settingsManager.js)
│
└── UI交互模块
    ├── ui.js (依赖: config.js)
    ├── events.js (依赖: config.js, clipboard.js, settings.js, ui.js)
    ├── sortable.js (依赖: config.js, clipboard.js, quickTexts.js, groups.js)
    ├── focus.js (依赖: config.js)
    └── window.js (依赖: config.js)
```

## 独立页面模块

### 主窗口 (`src/main.js`)
- 应用主入口
- 模块初始化和协调
- 全局事件绑定
- 应用生命周期管理

### 预览窗口 (`src/preview.js`)
- 独立的预览窗口逻辑
- 剪贴板项目预览和选择
- 键盘和鼠标滚轮交互
- 滚动音效播放集成
- 实时索引同步

### 截屏窗口 (`src/screenshot.js`)
- 截屏功能的前端实现
- 截屏区域选择和预览
- 截屏工具栏控制
- 截屏保存和分享

### 设置窗口 (`src/settings.html` + `js/settings.js`)
- 设置界面的完整实现
- 多页面设置导航
- 实时设置预览和应用
- 设置验证和错误处理

## 数据流向

1. **用户操作** → `events.js` → 相应功能模块
2. **设置变更** → `settingsManager.js` → 各功能模块
3. **剪贴板变化** → `clipboard.js` → `groups.js` → UI更新
4. **后端通信** → Tauri API → 各模块数据同步