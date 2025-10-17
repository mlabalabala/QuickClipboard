/**
 * 更新器 UI 模块
 */

import { showNotification } from '../js/notificationManager.js';
import { formatReleaseNotes, getCurrentPlatform } from './updater-utils.js';

/**
 * 显示/隐藏关于标签的更新徽章
 */
export class BadgeManager {
  static ensureAboutNavRelative() {
    const aboutNav = document.querySelector('.settings-nav .nav-item[data-section="about"]');
    if (aboutNav && getComputedStyle(aboutNav).position === 'static') {
      aboutNav.style.position = 'relative';
    }
    return aboutNav;
  }

  static show() {
    const aboutNav = this.ensureAboutNavRelative();
    if (!aboutNav) return;
    
    let badge = aboutNav.querySelector('.update-badge');
    if (!badge) {
      badge = document.createElement('span');
      badge.className = 'update-badge';
      badge.textContent = '有更新';
      badge.style.cssText = `
        position: absolute;
        top: 2px;
        right: 8px;
        background: linear-gradient(135deg, #ff6b6b, #ff922b);
        color: #fff;
        font-size: 10px;
        padding: 4px 6px;
        border-radius: 10px;
        line-height: 1;
        box-shadow: 0 2px 6px rgba(0, 0, 0, 0.2);
        font-weight: 600;
        z-index: 1;
      `;
      aboutNav.appendChild(badge);
    }
  }

  static hide() {
    const aboutNav = document.querySelector('.settings-nav .nav-item[data-section="about"]');
    const badge = aboutNav?.querySelector('.update-badge');
    if (badge) badge.remove();
  }
}

/**
 * 更新对话框管理器
 */
export class UpdateDialog {
  constructor(update, latestReleaseCache, onInstall) {
    this.update = update;
    this.latestReleaseCache = latestReleaseCache;
    this.onInstall = onInstall;
  }

  show() {
    const existingOverlay = document.querySelector('.update-dialog-overlay');
    if (existingOverlay) {
      existingOverlay.remove();
    }

    BadgeManager.hide();

    const overlay = document.createElement('div');
    overlay.className = 'update-dialog-overlay';
    overlay.style.cssText = `
      position: fixed;
      inset: 0;
      background: var(--overlay-bg, rgba(0, 0, 0, 0.5));
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 10001;
      backdrop-filter: blur(4px);
      animation: fadeIn 0.2s ease;
    `;

    const dialog = document.createElement('div');
    dialog.className = 'update-dialog';
    dialog.style.cssText = `
      width: 520px;
      max-width: 90vw;
      max-height: 80vh;
      background: var(--modal-bg, #fff);
      color: var(--text-primary, #333);
      border-radius: var(--border-radius-large, 8px);
      border: 1px solid var(--border-primary, #e0e0e0);
      box-shadow: var(--shadow-heavy, 0 4px 12px rgba(0, 0, 0, 0.15));
      overflow: hidden;
      display: flex;
      flex-direction: column;
      animation: slideIn 0.3s ease;
    `;

    const { header, closeBtn } = this.createHeader(overlay);
    const body = this.createBody();
    const { progressContainer, progressFill, progressText } = this.createProgressBar();
    const { footer, updateBtn, laterBtn } = this.createFooter(
      overlay,
      { progressContainer, progressFill, progressText, closeBtn }
    );

    dialog.appendChild(header);
    dialog.appendChild(body);
    dialog.appendChild(progressContainer);
    dialog.appendChild(footer);
    overlay.appendChild(dialog);
    document.body.appendChild(overlay);
  }

  createHeader(overlay) {
    const header = document.createElement('div');
    header.style.cssText = `
      padding: 18px 20px;
      background: var(--bg-tertiary, #f0f0f0);
      border-bottom: 1px solid var(--border-primary, #e0e0e0);
      display: flex;
      align-items: center;
      gap: 12px;
    `;

    const titleBox = document.createElement('div');
    titleBox.style.flex = '1';

    const title = document.createElement('h3');
    title.textContent = '发现新版本';
    title.style.cssText = `
      margin: 0;
      font-size: 16px;
      font-weight: 600;
      color: var(--text-primary, #333);
    `;

    const versionInfo = document.createElement('div');
    versionInfo.textContent = `${this.update.currentVersion} → ${this.update.version}`;
    versionInfo.style.cssText = `
      font-size: 12px;
      color: var(--text-secondary, #666);
      margin-top: 2px;
    `;

    titleBox.appendChild(title);
    titleBox.appendChild(versionInfo);

    const closeBtn = document.createElement('button');
    closeBtn.innerHTML = '<i class="ti ti-x"></i>';
    closeBtn.style.cssText = `
      background: transparent;
      border: none;
      color: var(--text-secondary, #666);
      font-size: 20px;
      cursor: pointer;
      width: 28px;
      height: 28px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: var(--border-radius-small, 4px);
      transition: all var(--transition-duration, 0.2s);
    `;
    closeBtn.addEventListener('mouseenter', () => {
      closeBtn.style.background = 'var(--bg-hover, rgba(0, 0, 0, 0.05))';
      closeBtn.style.color = 'var(--text-primary, #333)';
    });
    closeBtn.addEventListener('mouseleave', () => {
      closeBtn.style.background = 'transparent';
      closeBtn.style.color = 'var(--text-secondary, #666)';
    });
    closeBtn.addEventListener('click', () => overlay.remove());

    header.appendChild(titleBox);
    header.appendChild(closeBtn);

    return { header, closeBtn };
  }

  createBody() {
    const body = document.createElement('div');
    body.style.cssText = `
      padding: 20px;
      overflow: auto;
      flex: 1;
      background: var(--bg-primary, #fff);
    `;

    const updateDate = document.createElement('div');
    updateDate.textContent = `📅 发布日期: ${this.update.date || '未知'}`;
    updateDate.style.cssText = `
      font-size: 12px;
      color: var(--text-tertiary, #999);
      margin-bottom: 16px;
      padding: 8px 12px;
      background: var(--bg-secondary, #f8f9fa);
      border-radius: var(--border-radius-small, 4px);
      border-left: 3px solid var(--primary-color, #4a89dc);
    `;

    const notesTitle = document.createElement('h4');
    notesTitle.textContent = '更新内容';
    notesTitle.style.cssText = `
      margin: 0 0 10px 0;
      font-size: 14px;
      font-weight: 600;
      color: var(--text-primary, #333);
    `;

    const notes = document.createElement('div');
    notes.innerHTML = formatReleaseNotes(this.update.body || '暂无更新说明');
    notes.style.cssText = `
      font-size: 13px;
      line-height: 1.7;
      color: var(--text-secondary, #666);
      white-space: pre-wrap;
    `;

    body.appendChild(updateDate);
    body.appendChild(notesTitle);
    body.appendChild(notes);

    return body;
  }

  createProgressBar() {
    const progressContainer = document.createElement('div');
    progressContainer.style.cssText = `
      padding: 0 20px 16px;
      background: var(--bg-primary, #fff);
      border-top: 1px solid var(--separator-color, #eee);
      display: none;
    `;

    const progressBar = document.createElement('div');
    progressBar.style.cssText = `
      width: 100%;
      height: 8px;
      background: var(--bg-secondary, #f0f0f0);
      border-radius: 4px;
      overflow: hidden;
      position: relative;
      box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.05);
    `;

    const progressFill = document.createElement('div');
    progressFill.style.cssText = `
      width: 0%;
      height: 100%;
      background: linear-gradient(90deg, var(--primary-color, #4a89dc), var(--primary-hover, #357abd));
      transition: width 0.3s ease;
      border-radius: 4px;
    `;

    const progressText = document.createElement('div');
    progressText.style.cssText = `
      margin-top: 8px;
      font-size: 12px;
      color: var(--text-tertiary, #999);
      text-align: center;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 6px;
    `;
    
    const spinner = document.createElement('i');
    spinner.className = 'ti ti-loader-2';
    spinner.style.cssText = 'animation: spin 1s linear infinite;';
    
    progressText.appendChild(spinner);
    progressText.appendChild(document.createTextNode('准备下载...'));

    progressBar.appendChild(progressFill);
    progressContainer.appendChild(progressBar);
    progressContainer.appendChild(progressText);

    return { progressContainer, progressFill, progressText };
  }

  createButton(config) {
    const btn = document.createElement('button');
    btn.innerHTML = `<i class="ti ti-${config.icon}"></i><span style="margin-left: 6px;">${config.text}</span>`;
    if (config.title) btn.title = config.title;
    
    const baseStyle = `
      padding: 9px ${config.primary ? '20px' : '18px'};
      border-radius: var(--border-radius, 6px);
      cursor: pointer;
      font-size: 14px;
      display: flex;
      align-items: center;
      transition: all var(--transition-duration, 0.2s);
    `;
    
    if (config.primary) {
      btn.style.cssText = baseStyle + `
        background: var(--primary-color, #4a89dc);
        border: none;
        color: #fff;
        font-weight: 500;
        box-shadow: var(--shadow-light, 0 1px 3px rgba(0, 0, 0, 0.1));
      `;
      btn.addEventListener('mouseenter', () => {
        btn.style.background = 'var(--primary-hover, #357abd)';
        btn.style.transform = 'translateY(-1px)';
        btn.style.boxShadow = 'var(--shadow-medium, 0 2px 8px rgba(0, 0, 0, 0.15))';
      });
      btn.addEventListener('mouseleave', () => {
        btn.style.background = 'var(--primary-color, #4a89dc)';
        btn.style.transform = 'translateY(0)';
        btn.style.boxShadow = 'var(--shadow-light, 0 1px 3px rgba(0, 0, 0, 0.1))';
      });
    } else {
      btn.style.cssText = baseStyle + `
        background: ${config.transparent ? 'transparent' : 'var(--bg-secondary, #f8f9fa)'};
        border: 1px solid var(--border-primary, #e0e0e0);
        color: var(--text-secondary, #666);
      `;
      const hoverColor = config.hoverColor || 'var(--text-primary, #333)';
      btn.addEventListener('mouseenter', () => {
        btn.style.background = 'var(--bg-secondary, #f8f9fa)';
        btn.style.borderColor = config.hoverColor || 'var(--border-hover, #c0c0c0)';
        btn.style.color = hoverColor;
      });
      btn.addEventListener('mouseleave', () => {
        btn.style.background = config.transparent ? 'transparent' : 'var(--bg-secondary, #f8f9fa)';
        btn.style.borderColor = 'var(--border-primary, #e0e0e0)';
        btn.style.color = 'var(--text-secondary, #666)';
      });
    }
    
    if (config.onClick) btn.addEventListener('click', config.onClick);
    return btn;
  }

  createFooter(overlay, progressElements) {
    const footer = document.createElement('div');
    footer.style.cssText = `
      padding: 14px 20px;
      background: var(--bg-tertiary, #f0f0f0);
      display: flex;
      gap: 10px;
      justify-content: flex-end;
      border-top: 1px solid var(--border-primary, #e0e0e0);
    `;

    const laterBtn = this.createButton({
      icon: 'clock',
      text: '稍后更新',
      onClick: () => overlay.remove()
    });

    const manualBtn = this.createButton({
      icon: 'external-link',
      text: '浏览器下载',
      title: '如果自动更新速度慢，可以使用浏览器下载',
      transparent: true,
      hoverColor: 'var(--info-color, #1890ff)',
      onClick: async () => {
        const downloadUrl = this.getManualDownloadUrl();
        if (downloadUrl) {
          try {
            const { openUrl } = await import('@tauri-apps/plugin-opener');
            await openUrl(downloadUrl);
            showNotification('已在浏览器中打开下载链接', 'info');
          } catch {
            showNotification('打开链接失败', 'error');
          }
        }
      }
    });

    const updateBtn = this.createButton({
      icon: 'download',
      text: '立即更新',
      primary: true,
      onClick: () => {
        this.onInstall({
          ...progressElements,
          updateBtn,
          laterBtn,
          closeBtn: progressElements.closeBtn
        });
      }
    });

    footer.appendChild(laterBtn);
    footer.appendChild(manualBtn);
    footer.appendChild(updateBtn);

    return { footer, updateBtn, laterBtn };
  }

  getManualDownloadUrl() {
    if (this.latestReleaseCache?.platforms) {
      const platformKey = getCurrentPlatform();
      const platformData = this.latestReleaseCache.platforms[platformKey];
      if (platformData?.url) {
        return platformData.url;
      }
    }
 
    return this.update.htmlUrl || this.latestReleaseCache?.htmlUrl;
  }
}

