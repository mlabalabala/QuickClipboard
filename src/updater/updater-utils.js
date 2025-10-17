/**
 * 更新器工具函数
 */

import snarkdown from 'snarkdown';

/**
 * 比较版本号
 */
export function compareVersions(v1, v2) {
  const normalize = (v) => {
    if (!v) return [];
    return String(v).trim().replace(/^v/i, '').split('.').map(n => {
      const cleanNum = n.replace(/[^0-9]/g, '');
      const num = parseInt(cleanNum, 10);
      return Number.isNaN(num) ? 0 : num;
    });
  };

  const p1 = normalize(v1);
  const p2 = normalize(v2);
  const len = Math.max(p1.length, p2.length);

  for (let i = 0; i < len; i++) {
    const a = p1[i] ?? 0;
    const b = p2[i] ?? 0;
    if (a > b) return 1;
    if (a < b) return -1;
  }
  return 0;
}

/**
 * 获取当前平台标识
 */
export function getCurrentPlatform() {
  const platform = window.navigator.platform.toLowerCase();
  const arch = navigator.userAgent.includes('ARM') || navigator.userAgent.includes('aarch64') ? 'aarch64' : 'x86_64';
  
  if (platform.includes('win')) {
    return 'windows-x86_64';
  } else if (platform.includes('mac')) {
    return arch === 'aarch64' ? 'darwin-aarch64' : 'darwin-x86_64';
  } else if (platform.includes('linux')) {
    return 'linux-x86_64';
  }
  
  return 'windows-x86_64';
}

/**
 * 格式化更新说明（Markdown 转 HTML）
 */
export function formatReleaseNotes(body) {
  if (!body) return '暂无更新说明';

  const cleanBody = body.replace(/^---\s*$/gm, '');

  let html = snarkdown(cleanBody);

  html = html
    .replace(/<h1>/g, '<h1 style="margin: 18px 0 12px 0; font-size: 15px; color: var(--text-primary, #333); font-weight: 700; padding-bottom: 8px; border-bottom: 2px solid var(--border-primary, #e0e0e0);">')
    .replace(/<h2>/g, '<h2 style="margin: 16px 0 10px 0; font-size: 14px; color: var(--text-primary, #333); font-weight: 700;">')
    .replace(/<h3>/g, '<h3 style="margin: 14px 0 8px 0; font-size: 13px; color: var(--text-primary, #333); font-weight: 600; padding-left: 12px; border-left: 3px solid var(--primary-color, #4a89dc);">')
    .replace(/<ul>/g, '<ul style="margin: 12px 0; padding-left: 0; list-style: none;">')
    .replace(/<li>/g, '<li style="display: flex; align-items: flex-start; margin: 6px 0;"><span style="color: var(--primary-color, #4a89dc); margin-right: 8px; font-weight: bold; flex-shrink: 0;">•</span><span style="flex: 1; color: var(--text-secondary, #666); line-height: 1.6;">')
    .replace(/<\/li>/g, '</span></li>')
    .replace(/<p>/g, '<p style="margin: 8px 0; color: var(--text-secondary, #666); line-height: 1.6;">')
    .replace(/<code>/g, '<code style="background: var(--bg-secondary, #f8f9fa); color: var(--primary-color, #4a89dc); padding: 2px 6px; border-radius: 3px; font-family: monospace; font-size: 12px; border: 1px solid var(--border-primary, #e0e0e0);">')
    .replace(/<strong>/g, '<strong style="color: var(--text-primary, #333); font-weight: 600;">');
  
  return html;
}

