import { Window } from '@tauri-apps/api/window';

// 应用窗口实例
export const appWindow = new Window('main');

// 全局状态变量
export let clipboardHistory = [];
export let quickTexts = [];
export let isPinned = false; // 固定状态，控制粘贴后是否隐藏窗口
export let activeItemIndex = -1;
export let isDragging = false;
export let currentTab = 'clipboard';
export let editingQuickTextId = null;
export let currentFilter = 'all'; // 当前剪贴板筛选类型：all, text, image, link
export let currentQuickTextsFilter = 'all'; // 当前常用文本筛选类型：all, text, image, link
export let isOneTimePaste = false; // 一次性粘贴开关状态
export let isAiTranslationEnabled = false; // AI翻译开关状态

// Sortable实例
export let clipboardSortable = null;
export let quickTextsSortable = null;

// DOM元素引用
export let searchInput;
export let contentFilter;
export let clipboardList;
export let pinButton;
export let oneTimePasteSwitch;
export let aiTranslationSwitch;
export let quickTextsSearch;
export let quickTextsFilter;
export let quickTextsList;
export let quickTextModal;
export let modalTitle;
export let quickTextTitleInput;
export let quickTextContentInput;
export let confirmModal;
export let confirmTitle;
export let confirmMessage;
export let confirmCallback;
export let alertModal;
export let alertTitle;
export let alertMessage;
export let settingsModal;
export let titleBar;
export let groupModal;
export let groupModalTitle;
export let groupNameInput;
export let groupIconSelect;
export let quickTextGroupSelect;
export let screenshotButton;
// 当前设置
export let currentSettings = {
  theme: 'system',
  startupLaunch: false,
  historyLimit: 50
};

// 更新全局状态的函数
export function setClipboardHistory(history) {
  clipboardHistory = history;
}

export function setQuickTexts(texts) {
  quickTexts = texts;
}

export function setIsPinned(pinned) {
  isPinned = pinned;
}

export function setActiveItemIndex(index) {
  activeItemIndex = index;
}

export function setIsDragging(dragging) {
  isDragging = dragging;
}

export function setCurrentTab(tab) {
  currentTab = tab;
}

export function setEditingQuickTextId(id) {
  editingQuickTextId = id;
}

export function setCurrentFilter(filter) {
  currentFilter = filter;
}

export function setCurrentQuickTextsFilter(filter) {
  currentQuickTextsFilter = filter;
}

export function setIsOneTimePaste(oneTime) {
  isOneTimePaste = oneTime;
}

export function setIsAiTranslationEnabled(enabled) {
  isAiTranslationEnabled = enabled;
}

export function getIsAiTranslationEnabled() {
  return isAiTranslationEnabled;
}

export function setClipboardSortable(sortable) {
  clipboardSortable = sortable;
}

export function setQuickTextsSortable(sortable) {
  quickTextsSortable = sortable;
}

export function setConfirmCallback(callback) {
  confirmCallback = callback;
}

export function setCurrentSettings(settings) {
  currentSettings = { ...currentSettings, ...settings };
}

// 初始化DOM元素引用
export function initDOMReferences() {
  searchInput = document.querySelector('#search-input');
  contentFilter = document.querySelector('#content-filter');
  clipboardList = document.querySelector('#clipboard-list');
  pinButton = document.querySelector('#pin-button');
  oneTimePasteSwitch = document.querySelector('#one-time-paste-switch');
  aiTranslationSwitch = document.querySelector('#ai-translation-switch');
  quickTextsSearch = document.querySelector('#quick-texts-search');
  quickTextsFilter = document.querySelector('#quick-texts-filter');
  quickTextsList = document.querySelector('#quick-texts-list');
  quickTextModal = document.querySelector('#quick-text-modal');
  modalTitle = document.querySelector('#modal-title');
  quickTextTitleInput = document.querySelector('#quick-text-title');
  quickTextContentInput = document.querySelector('#quick-text-content');
  confirmModal = document.querySelector('#confirm-modal');
  confirmTitle = document.querySelector('#confirm-title');
  confirmMessage = document.querySelector('#confirm-message');
  alertModal = document.querySelector('#alert-modal');
  alertTitle = document.querySelector('#alert-title');
  alertMessage = document.querySelector('#alert-message');
  settingsModal = document.querySelector('#settings-modal');
  titleBar = document.querySelector('.title-bar');
  groupModal = document.querySelector('#group-modal');
  groupModalTitle = document.querySelector('#group-modal-title');
  groupNameInput = document.querySelector('#group-name');
  groupIconSelect = document.querySelector('#group-icon');
  quickTextGroupSelect = document.querySelector('#quick-text-group');
  screenshotButton = document.querySelector('#screenshot-button');
}
