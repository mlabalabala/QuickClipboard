import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';

let latestReleaseCache = null;
let hasUpdateCache = null;

function normalizeVersion(version) {
  if (!version) return '';
  return String(version).trim().replace(/^v/i, '').replace(/^V/i, '');
}

function parseVersion(version) {
  const parts = normalizeVersion(version).split('-');
  const mainVersion = parts[0];
  const preRelease = parts[1] || '';

  const mainParts = mainVersion.split('.').map(n => {
    const cleanNum = n.replace(/[^0-9]/g, '');
    const num = parseInt(cleanNum, 10);
    return Number.isNaN(num) ? 0 : num;
  });

  // 添加预发布版本信息
  if (preRelease) {
    const preMatch = preRelease.match(/^(beta|alpha|rc)(\.?)(\d+)?$/i);
    if (preMatch) {
      const type = preMatch[1].toLowerCase();
      const number = parseInt(preMatch[3] || '0', 10);
      mainParts.push(type === 'alpha' ? 0 : type === 'beta' ? 1 : 2); // 类型优先级
      mainParts.push(number); // 预发布版本号
    }
  } else {
    // 正式版本（无预发布标识）优先级最高
    mainParts.push(999); // 比任何预发布版本都高
    mainParts.push(0);
  }

  return mainParts;
}

function compareVersions(a, b) {
  const pa = parseVersion(a);
  const pb = parseVersion(b);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const ai = pa[i] ?? 0;
    const bi = pb[i] ?? 0;
    if (ai > bi) return 1;
    if (ai < bi) return -1;
  }
  return 0;
}

async function getCurrentAppVersion() {
  try {
    const info = await invoke('get_app_version');
    return info?.version || '';
  } catch {
    return '';
  }
}

async function fetchLatestRelease() {
  if (latestReleaseCache) return latestReleaseCache;
  const url = 'https://api.github.com/repos/mosheng1/QuickClipboard/releases';
  const res = await fetch(url, {
    headers: {
      'Accept': 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      'User-Agent': 'QuickClipboard'
    }
  });
  if (!res.ok) throw new Error('获取版本信息失败');
  const releases = await res.json();
  if (!Array.isArray(releases) || releases.length === 0) {
    throw new Error('未找到可用的版本');
  }
  // 取第一个非 draft 的条目
  const latest = releases.find(r => !r.draft) || releases[0];
  latestReleaseCache = {
    tagName: latest.tag_name || '',
    name: latest.name || latest.tag_name || '',
    body: latest.body || '',
    isPrerelease: !!latest.prerelease,
    htmlUrl: latest.html_url || 'https://github.com/mosheng1/QuickClipboard/releases'
  };
  return latestReleaseCache;
}

async function checkForUpdateAvailability() {
  const currentVersion = await getCurrentAppVersion();
  const latest = await fetchLatestRelease();
  const current = normalizeVersion(currentVersion);
  const latestTag = normalizeVersion(latest.tagName || latest.name);
  let hasUpdate = false;
  if (current && latestTag) {
    hasUpdate = compareVersions(latestTag, current) > 0;
  } else if (latestTag && !current) {
    hasUpdate = true;
  }
  hasUpdateCache = hasUpdate;
  return { hasUpdate, latestRelease: latest, currentVersion: currentVersion };
}

function ensureAboutNavRelative() {
  const aboutNav = document.querySelector('.settings-nav .nav-item[data-section="about"]');
  if (aboutNav && getComputedStyle(aboutNav).position === 'static') {
    aboutNav.style.position = 'relative';
  }
  return aboutNav;
}

function showAboutNavBadge() {
  const aboutNav = ensureAboutNavRelative();
  if (!aboutNav) return;
  let badge = aboutNav.querySelector('.update-badge');
  if (!badge) {
    badge = document.createElement('span');
    badge.className = 'update-badge';
    badge.textContent = '有更新';
    badge.style.cssText = [
      'position:absolute',
      'top:2px',
      'right:8px',
      'background:linear-gradient(135deg,#ff6b6b,#ff922b)',
      'color:#fff',
      'font-size:10px',
      'padding:4px 6px',
      'border-radius:10px',
      'line-height:1',
      'box-shadow:0 2px 6px rgba(0,0,0,0.2)'
    ].join(';');
    aboutNav.appendChild(badge);
  }
}

function hideAboutNavBadge() {
  const aboutNav = document.querySelector('.settings-nav .nav-item[data-section="about"]');
  const badge = aboutNav?.querySelector('.update-badge');
  if (badge) badge.remove();
}

function renderUpdateModal({ title, version, notes, downloadUrl, isPrerelease }) {
  const overlay = document.createElement('div');
  overlay.className = 'update-dialog-overlay';
  overlay.style.cssText = [
    'position:fixed',
    'inset:0',
    'background:rgba(0,0,0,0.4)',
    'display:flex',
    'align-items:center',
    'justify-content:center',
    'z-index:10001'
  ].join(';');

  const dialog = document.createElement('div');
  dialog.className = 'update-dialog';
  dialog.style.cssText = [
    'width:720px',
    'max-width:92vw',
    'max-height:80vh',
    'background:#1f2533',
    'color:#fff',
    'border-radius:12px',
    'box-shadow:0 12px 30px rgba(0,0,0,0.35)',
    'overflow:hidden',
    'display:flex',
    'flex-direction:column'
  ].join(';');

  const header = document.createElement('div');
  header.style.cssText = 'padding:16px 20px; background:#273042; display:flex; align-items:center; justify-content:space-between;';
  const h3 = document.createElement('h3');
  h3.textContent = `${title} (${version})` + (isPrerelease ? ' - 预发布' : '');
  h3.style.margin = '0';
  h3.style.fontSize = '16px';
  const closeBtn = document.createElement('button');
  closeBtn.textContent = '×';
  closeBtn.style.cssText = 'background:transparent;border:none;color:#bbb;font-size:20px;cursor:pointer;';
  closeBtn.addEventListener('click', () => document.body.removeChild(overlay));
  header.appendChild(h3);
  header.appendChild(closeBtn);

  const body = document.createElement('div');
  body.style.cssText = 'padding:16px 20px; overflow:auto; flex:1; background:#1b2130;';
  const pre = document.createElement('pre');
  pre.textContent = notes || '无更新说明';
  pre.style.cssText = 'white-space:pre-wrap; font-family:inherit; line-height:1.6; font-size:13px; color:#e6e6e6;';
  body.appendChild(pre);

  const footer = document.createElement('div');
  footer.style.cssText = 'padding:12px 20px; background:#273042; display:flex; gap:10px; justify-content:flex-end;';
  const laterBtn = document.createElement('button');
  laterBtn.textContent = '稍后';
  laterBtn.style.cssText = 'background:#565f7a;border:none;color:#fff;padding:8px 14px;border-radius:8px;cursor:pointer;';
  laterBtn.addEventListener('click', () => document.body.removeChild(overlay));
  const downloadBtn = document.createElement('button');
  downloadBtn.textContent = '前往下载';
  downloadBtn.style.cssText = 'background:#4a89dc;border:none;color:#fff;padding:8px 14px;border-radius:8px;cursor:pointer;';
  downloadBtn.addEventListener('click', async () => {
    try { await openUrl(downloadUrl); } catch { }
    // document.body.removeChild(overlay);
  });
  footer.appendChild(laterBtn);
  footer.appendChild(downloadBtn);

  dialog.appendChild(header);
  dialog.appendChild(body);
  dialog.appendChild(footer);
  overlay.appendChild(dialog);
  document.body.appendChild(overlay);
}

async function autoCheckAndMark() {
  try {
    const { hasUpdate, latestRelease } = await checkForUpdateAvailability();
    if (hasUpdate) {
      showAboutNavBadge();
      try {
        window.dispatchEvent(new CustomEvent('qc-update-available', {
          detail: { latestRelease }
        }));
      } catch { }
    }
  } catch {
    // 静默失败
  }
}

async function handleCheckUpdatesClick() {
  try {
    const { hasUpdate, latestRelease, currentVersion } = await checkForUpdateAvailability();

    if (hasUpdate) {
      renderUpdateModal({
        title: '发现新版本',
        version: latestRelease.tagName || latestRelease.name,
        notes: latestRelease.body || '',
        downloadUrl: latestRelease.htmlUrl,
        isPrerelease: latestRelease.isPrerelease
      });
    } else {
      renderUpdateModal({
        title: '已是最新版本',
        version: normalizeVersion(currentVersion) || '未知',
        notes: '您当前使用的已是最新版本。',
        downloadUrl: 'https://github.com/mosheng1/QuickClipboard/releases',
        isPrerelease: false
      });
      hideAboutNavBadge();
    }
  } catch (e) {
    renderUpdateModal({
      title: '检查更新失败',
      version: '',
      notes: '无法获取版本信息，请稍后重试。',
      downloadUrl: 'https://github.com/mosheng1/QuickClipboard/releases',
      isPrerelease: false
    });
  }
}

function setupUpdateChecker() {
  // 自动检测一次并标记
  setTimeout(() => { autoCheckAndMark(); }, 500);
}

export { setupUpdateChecker, handleCheckUpdatesClick };


