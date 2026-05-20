const invoke = window.__TAURI__.core.invoke;
const CURRENT_VERSION = 'v0.1.3';
const RELEASE_API_URL = 'https://api.github.com/repos/gorkys/chromeProfileManage/releases/latest';
const RELEASE_PAGE_URL = 'https://github.com/gorkys/chromeProfileManage/releases';

const api = {
  loadConfig: () => invoke('load_config'),
  updateConfig: (patch) => invoke('update_config', { patch }),
  selectChromeFile: () => invoke('select_chrome_file'),
  selectDirectory: (title, initialPath = '') => invoke('select_directory', { title, initialPath }),
  createEnvironment: (payload) => invoke('create_environment', { payload }),
  scanExistingProfiles: () => invoke('scan_existing_profiles'),
  updateEnvironment: (id, patch) => invoke('update_environment', { id, patch }),
  deleteEnvironment: (id) => invoke('delete_environment', { id }),
  copyMaster: (id) => invoke('copy_master', { id }),
  launchEnvironment: (id) => invoke('launch_environment', { id }),
  openPath: (targetPath) => invoke('open_path', { targetPath }),
};

const state = {
  config: null,
  saveTimers: new Map(),
  settingsSaveTimer: null,
  editing: new Set(),
  savingFields: new Set(),
};

const nodes = {
  navItems: document.querySelectorAll('[data-view]'),
  views: document.querySelectorAll('.view'),
  environmentCount: document.getElementById('environmentCount'),
  environmentList: document.getElementById('environmentList'),
  emptyState: document.getElementById('emptyState'),
  createForm: document.getElementById('createForm'),
  envName: document.getElementById('envName'),
  envUrl: document.getElementById('envUrl'),
  copyMaster: document.getElementById('copyMaster'),
  scanProfilesBtn: document.getElementById('scanProfilesBtn'),
  chromePath: document.getElementById('chromePath'),
  masterProfilePath: document.getElementById('masterProfilePath'),
  profileStoragePath: document.getElementById('profileStoragePath'),
  defaultUrl: document.getElementById('defaultUrl'),
  syncCookies: document.getElementById('syncCookies'),
  syncSiteStorage: document.getElementById('syncSiteStorage'),
  syncExtensions: document.getElementById('syncExtensions'),
  syncCache: document.getElementById('syncCache'),
  syncSessions: document.getElementById('syncSessions'),
  syncHistory: document.getElementById('syncHistory'),
  selectChromeBtn: document.getElementById('selectChromeBtn'),
  selectMasterBtn: document.getElementById('selectMasterBtn'),
  selectStorageBtn: document.getElementById('selectStorageBtn'),
  openRepositoryBtn: document.getElementById('openRepositoryBtn'),
  checkUpdateBtn: document.getElementById('checkUpdateBtn'),
  updateStatus: document.getElementById('updateStatus'),
  themeToggleBtn: document.getElementById('themeToggleBtn'),
  themeIcon: document.getElementById('themeIcon'),
  toast: document.getElementById('toast'),
};

/**
 * 显示操作结果提示
 * @param {string} message 提示文本
 * @param {boolean} isError 是否为错误提示
 * @returns {void} 无返回值
 */
function showToast(message, isError = false) {
  nodes.toast.textContent = message;
  nodes.toast.classList.toggle('error', isError);
  nodes.toast.classList.remove('hidden');

  window.clearTimeout(showToast.timer);
  showToast.timer = window.setTimeout(() => {
    nodes.toast.classList.add('hidden');
  }, 2800);
}

/**
 * 执行异步操作并统一展示错误
 * @param {Function} task 要执行的异步任务
 * @returns {Promise<void>} 无返回值
 */
async function runTask(task) {
  try {
    await task();
  } catch (error) {
    showToast(error.message || '操作失败', true);
  }
}

/**
 * 转义 HTML 文本，避免路径或名称影响页面结构
 * @param {string} value 原始文本
 * @returns {string} 转义后的文本
 */
function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}

/**
 * 格式化时间展示
 * @param {string} value ISO 时间字符串
 * @returns {string} 本地化时间文本
 */
function formatTime(value) {
  if (!value) {
    return '-';
  }

  return new Date(value).toLocaleString('zh-CN', {
    hour12: false,
  });
}

/**
 * 将版本号解析为数字序列
 * @param {string} version 版本号文本
 * @returns {number[]} 版本数字序列
 */
function parseVersion(version) {
  return String(version || '')
    .replace(/^v/i, '')
    .split('.')
    .map((part) => Number.parseInt(part, 10))
    .map((part) => (Number.isFinite(part) ? part : 0));
}

/**
 * 比较两个版本号的新旧
 * @param {string} left 左侧版本号
 * @param {string} right 右侧版本号
 * @returns {number} 左侧更新返回 1，相同返回 0，左侧更旧返回 -1
 */
function compareVersions(left, right) {
  const leftParts = parseVersion(left);
  const rightParts = parseVersion(right);
  const length = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < length; index += 1) {
    const leftPart = leftParts[index] || 0;
    const rightPart = rightParts[index] || 0;

    if (leftPart > rightPart) {
      return 1;
    }

    if (leftPart < rightPart) {
      return -1;
    }
  }

  return 0;
}

/**
 * 从 GitHub Releases 获取最新版本标签
 * @returns {Promise<string>} 最新 release 标签
 */
async function fetchLatestReleaseTag() {
  const response = await fetch(RELEASE_API_URL, {
    headers: {
      Accept: 'application/vnd.github+json',
    },
  });

  if (!response.ok) {
    throw new Error(`检查更新失败：GitHub 返回 ${response.status}`);
  }

  const release = await response.json();
  return release.tag_name || '';
}

/**
 * 生成页面内联图标
 * @param {string} name 图标名称
 * @returns {string} SVG 图标 HTML
 */
function icon(name) {
  const icons = {
    launch: '<svg viewBox="0 0 24 24"><path d="M5 3l14 9-14 9V3z"></path></svg>',
    sync: '<svg viewBox="0 0 24 24"><path d="M21 12a9 9 0 0 1-15.5 6.2"></path><path d="M3 12A9 9 0 0 1 18.5 5.8"></path><path d="M18 3v5h5"></path><path d="M6 21v-5H1"></path></svg>',
    folder: '<svg viewBox="0 0 24 24"><path d="M3 6h7l2 2h9v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6z"></path></svg>',
    delete: '<svg viewBox="0 0 24 24"><path d="M4 7h16"></path><path d="M10 11v6"></path><path d="M14 11v6"></path><path d="M6 7l1 14h10l1-14"></path><path d="M9 7V4h6v3"></path></svg>',
  };

  return icons[name] || '';
}

/**
 * 判断环境是否为受保护的外部 Profile
 * @param {object} environment 环境配置对象
 * @returns {boolean} 是外部 Default、Guest Profile、System Profile 时返回 true
 */
function isProtectedExternalProfile(environment) {
  return environment?.managed === false
    && ['default', 'guest profile', 'system profile'].includes(String(environment.profileDirectory || '').toLowerCase());
}

/**
 * 判断环境是否为可删除磁盘目录的外部 Profile
 * @param {object} environment 环境配置对象
 * @returns {boolean} 可删除外部 Profile 目录时返回 true
 */
function isDeletableExternalProfile(environment) {
  return environment?.managed === false && !isProtectedExternalProfile(environment);
}

/**
 * 应用并持久化主题模式
 * @param {string} theme 主题模式，light 或 dark
 * @returns {void} 无返回值
 */
function applyTheme(theme) {
  const safeTheme = theme === 'dark' ? 'dark' : 'light';

  document.documentElement.dataset.theme = safeTheme;
  localStorage.setItem('theme', safeTheme);
  nodes.themeIcon.textContent = safeTheme === 'dark' ? '☾' : '☀';
  nodes.themeToggleBtn.title = safeTheme === 'dark' ? '切换浅色模式' : '切换深色模式';
}

/**
 * 刷新配置并重新渲染页面
 * @returns {Promise<void>} 无返回值
 */
async function loadAndRender() {
  state.config = await api.loadConfig();
  renderSettings();
  renderEnvironments();
}

/**
 * 渲染全局设置表单
 * @returns {void} 无返回值
 */
function renderSettings() {
  const syncOptions = getSyncOptions();

  nodes.chromePath.value = state.config.chromePath || '';
  nodes.masterProfilePath.value = state.config.masterProfilePath || '';
  nodes.profileStoragePath.value = state.config.profileStoragePath || '';
  nodes.defaultUrl.value = state.config.defaultUrl || '';
  nodes.syncCookies.checked = syncOptions.syncCookies;
  nodes.syncSiteStorage.checked = syncOptions.syncSiteStorage;
  nodes.syncExtensions.checked = syncOptions.syncExtensions;
  nodes.syncCache.checked = syncOptions.syncCache;
  nodes.syncSessions.checked = syncOptions.syncSessions;
  nodes.syncHistory.checked = syncOptions.syncHistory;
}

/**
 * 获取当前配置中的同步选项
 * @returns {object} 带默认值的同步选项
 */
function getSyncOptions() {
  return {
    syncCookies: state.config?.syncOptions?.syncCookies ?? true,
    syncSiteStorage: state.config?.syncOptions?.syncSiteStorage ?? true,
    syncExtensions: state.config?.syncOptions?.syncExtensions ?? true,
    syncCache: state.config?.syncOptions?.syncCache ?? true,
    syncSessions: state.config?.syncOptions?.syncSessions ?? true,
    syncHistory: state.config?.syncOptions?.syncHistory ?? true,
  };
}

/**
 * 从设置表单读取同步选项
 * @returns {object} 可保存的同步选项
 */
function getSyncOptionsFromForm() {
  return {
    syncCookies: nodes.syncCookies.checked,
    syncSiteStorage: nodes.syncSiteStorage.checked,
    syncExtensions: nodes.syncExtensions.checked,
    syncCache: nodes.syncCache.checked,
    syncSessions: nodes.syncSessions.checked,
    syncHistory: nodes.syncHistory.checked,
  };
}

/**
 * 从设置表单读取全局配置
 * @returns {object} 可保存的全局配置字段
 */
function getSettingsPatchFromForm() {
  return {
    chromePath: nodes.chromePath.value.trim(),
    masterProfilePath: nodes.masterProfilePath.value.trim(),
    profileStoragePath: nodes.profileStoragePath.value.trim(),
    defaultUrl: nodes.defaultUrl.value.trim(),
    syncOptions: getSyncOptionsFromForm(),
  };
}

/**
 * 保存全局设置并同步本地状态
 * @param {boolean} showSuccess 是否显示保存成功提示
 * @returns {Promise<void>} 无返回值
 */
async function saveSettings(showSuccess = true) {
  state.config = await api.updateConfig(getSettingsPatchFromForm());

  if (showSuccess) {
    showToast('设置已自动保存');
  }
}

/**
 * 防抖自动保存全局设置
 * @returns {void} 无返回值
 */
function scheduleSettingsSave() {
  window.clearTimeout(state.settingsSaveTimer);
  state.settingsSaveTimer = window.setTimeout(() => {
    runTask(async () => {
      await saveSettings();
    });
  }, 650);
}

/**
 * 渲染环境列表
 * @returns {void} 无返回值
 */
function renderEnvironments() {
  const environments = state.config.environments || [];
  nodes.environmentCount.textContent = String(environments.length);
  nodes.emptyState.classList.toggle('hidden', environments.length > 0);
  document.getElementById('environmentTablePanel').classList.toggle('hidden', environments.length === 0);

  nodes.environmentList.innerHTML = environments.map((environment) => renderEnvironmentRow(environment)).join('');
}

/**
 * 渲染单个环境表格行
 * @param {object} environment 环境配置对象
 * @returns {string} 环境表格行 HTML
 */
function renderEnvironmentRow(environment) {
  const nameKey = `${environment.id}:name`;
  const startUrlKey = `${environment.id}:startUrl`;
  const nameCell = state.editing.has(nameKey)
    ? `<input class="name-input" type="text" data-field="name" value="${escapeHtml(environment.name)}" />`
    : `<span class="editable-text" data-edit-field="name" title="双击编辑">${escapeHtml(environment.name)}</span>`;
  const startUrlText = environment.startUrl || '未设置';
  const profileModeText = environment.managed === false ? '外部' : '内部';
  const canSyncMaster = environment.managed !== false || !isProtectedExternalProfile(environment);
  const startUrlCell = state.editing.has(startUrlKey)
    ? `<input type="url" data-field="startUrl" value="${escapeHtml(environment.startUrl || '')}" placeholder="https://example.com" />`
    : `<span class="editable-text ${environment.startUrl ? '' : 'empty'}" data-edit-field="startUrl" title="双击编辑">${escapeHtml(startUrlText)}</span>`;

  return `
    <tr data-id="${environment.id}">
      <td>${nameCell}</td>
      <td class="status-cell"><span class="profile-mode">${profileModeText}</span></td>
      <td>${startUrlCell}</td>
      <td>${formatTime(environment.createdAt)}</td>
      <td>
        <div class="operation-list">
          <button class="icon-btn primary" data-action="launch" data-id="${environment.id}" title="启动" type="button">${icon('launch')}</button>
          ${canSyncMaster ? `<button class="icon-btn" data-action="copy-master" data-id="${environment.id}" title="${environment.managed === false ? '同步母版到外部 Profile' : '同步母版'}" type="button">${icon('sync')}</button>` : ''}
          <button class="icon-btn" data-action="open-profile" data-id="${environment.id}" title="${escapeHtml(environment.profilePath)}" type="button">${icon('folder')}</button>
          <button class="icon-btn danger" data-action="delete" data-id="${environment.id}" title="${isDeletableExternalProfile(environment) ? '删除外部 Profile 文件夹' : environment.managed === false ? '移出管理列表，不删除受保护 Profile' : '删除环境和 profile 文件夹'}" type="button">${icon('delete')}</button>
        </div>
      </td>
    </tr>
  `;
}

/**
 * 根据环境 id 获取当前配置项
 * @param {string} id 环境 id
 * @returns {object | undefined} 匹配的环境对象
 */
function getEnvironment(id) {
  return state.config.environments.find((item) => item.id === id);
}

/**
 * 获取卡片中已编辑的环境字段
 * @param {HTMLElement} row 环境表格行元素
 * @returns {object} 可保存的环境字段
 */
function getRowPatch(row) {
  return {
    name: getEditableValue(row, 'name'),
    startUrl: getEditableValue(row, 'startUrl'),
  };
}

/**
 * 获取表格行中字段当前值
 * @param {HTMLElement} row 环境表格行元素
 * @param {string} field 字段名
 * @returns {string} 当前字段值
 */
function getEditableValue(row, field) {
  const input = row.querySelector(`[data-field="${field}"]`);

  if (input) {
    return input.value.trim();
  }

  const environment = getEnvironment(row.dataset.id);

  return String(environment?.[field] || '').trim();
}

/**
 * 进入单元格编辑模式
 * @param {HTMLElement} row 环境表格行元素
 * @param {string} field 字段名
 * @returns {void} 无返回值
 */
function enterEditMode(row, field) {
  const id = row.dataset.id;
  const key = `${id}:${field}`;

  state.editing.add(key);
  renderEnvironments();

  window.requestAnimationFrame(() => {
    const input = nodes.environmentList.querySelector(`tr[data-id="${id}"] [data-field="${field}"]`);

    if (input) {
      input.focus();
      input.select();
    }
  });
}

/**
 * 退出单元格编辑模式
 * @param {HTMLElement} row 环境表格行元素
 * @param {string} field 字段名
 * @returns {void} 无返回值
 */
function exitEditMode(row, field) {
  state.editing.delete(`${row.dataset.id}:${field}`);
  renderEnvironments();
}

/**
 * 防抖保存环境编辑内容
 * @param {HTMLElement} row 环境表格行元素
 * @returns {void} 无返回值
 */
function scheduleEnvironmentSave(row) {
  const id = row.dataset.id;

  window.clearTimeout(state.saveTimers.get(id));
  state.saveTimers.set(id, window.setTimeout(() => {
    runTask(async () => {
      const patch = getRowPatch(row);
      await api.updateEnvironment(id, patch);
      const environment = getEnvironment(id);

      if (environment) {
        Object.assign(environment, patch);
      }

      showToast('修改已自动保存');
    });
  }, 650));
}

nodes.navItems.forEach((button) => {
  button.addEventListener('click', () => {
    const viewName = button.dataset.view;

    nodes.navItems.forEach((item) => item.classList.toggle('active', item === button));
    nodes.views.forEach((view) => view.classList.toggle('active', view.id === `view-${viewName}`));
  });
});

nodes.createForm.addEventListener('submit', (event) => {
  event.preventDefault();

  runTask(async () => {
    await api.createEnvironment({
      name: nodes.envName.value,
      startUrl: nodes.envUrl.value,
      copyMaster: nodes.copyMaster.checked,
    });

    nodes.createForm.reset();
    nodes.copyMaster.checked = true;
    await loadAndRender();
    showToast('环境已创建');
  });
});

nodes.scanProfilesBtn.addEventListener('click', () => {
  runTask(async () => {
    const scannedEnvironments = await api.scanExistingProfiles();

    await loadAndRender();
    showToast(scannedEnvironments.length > 0 ? `已扫描到 ${scannedEnvironments.length} 个已有 Profile` : '没有发现新的已有 Profile');
  });
});

nodes.selectChromeBtn.addEventListener('click', () => {
  runTask(async () => {
    const selected = await api.selectChromeFile();

    if (selected) {
      nodes.chromePath.value = selected;
      await saveSettings();
    }
  });
});

nodes.selectMasterBtn.addEventListener('click', () => {
  runTask(async () => {
    const selected = await api.selectDirectory('选择母版 Profile 路径', nodes.masterProfilePath.value);

    if (selected) {
      nodes.masterProfilePath.value = selected;
      await saveSettings();
    }
  });
});

nodes.selectStorageBtn.addEventListener('click', () => {
  runTask(async () => {
    const selected = await api.selectDirectory('选择 Profile 保存路径', nodes.profileStoragePath.value);

    if (selected) {
      nodes.profileStoragePath.value = selected;
      await saveSettings();
    }
  });
});

[
  nodes.chromePath,
  nodes.masterProfilePath,
  nodes.profileStoragePath,
  nodes.defaultUrl,
].forEach((input) => {
  input.addEventListener('input', scheduleSettingsSave);
  input.addEventListener('blur', () => {
    window.clearTimeout(state.settingsSaveTimer);

    runTask(async () => {
      await saveSettings(false);
    });
  });
});

[
  nodes.syncCookies,
  nodes.syncSiteStorage,
  nodes.syncExtensions,
  nodes.syncCache,
  nodes.syncSessions,
  nodes.syncHistory,
].forEach((checkbox) => {
  checkbox.addEventListener('change', () => {
    runTask(async () => {
      await saveSettings();
    });
  });
});

nodes.themeToggleBtn.addEventListener('click', () => {
  const nextTheme = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
  applyTheme(nextTheme);
});

nodes.openRepositoryBtn.addEventListener('click', () => {
  runTask(async () => {
    await api.openPath('https://github.com/gorkys/chromeProfileManage');
  });
});

nodes.checkUpdateBtn.addEventListener('click', () => {
  runTask(async () => {
    nodes.checkUpdateBtn.disabled = true;
    nodes.updateStatus.textContent = '正在检查更新...';

    try {
      const latestTag = await fetchLatestReleaseTag();

      if (!latestTag) {
        throw new Error('检查更新失败：未获取到最新版本标签');
      }

      if (compareVersions(latestTag, CURRENT_VERSION) > 0) {
        nodes.updateStatus.innerHTML = `发现新版本 ${escapeHtml(latestTag)}，<button class="link-button inline-link" id="openReleasePageBtn" type="button">打开发布页</button>`;
        return;
      }

      nodes.updateStatus.textContent = `已是最新版本 ${CURRENT_VERSION}`;
    } finally {
      nodes.checkUpdateBtn.disabled = false;
    }
  });
});

nodes.updateStatus.addEventListener('click', (event) => {
  const button = event.target.closest('#openReleasePageBtn');

  if (!button) {
    return;
  }

  runTask(async () => {
    await api.openPath(RELEASE_PAGE_URL);
  });
});

nodes.environmentList.addEventListener('click', (event) => {
  const button = event.target.closest('button[data-action]');

  if (!button) {
    return;
  }

  const { action, id } = button.dataset;
  const environment = getEnvironment(id);

  runTask(async () => {
    if (action === 'launch') {
      await api.launchEnvironment(id);
      showToast('Chrome 已启动');
      return;
    }

    if (action === 'copy-master') {
      if (environment.managed === false) {
        const confirmed = window.confirm(`将把母版 Profile 同步到外部环境「${environment.name}」：\n${environment.profilePath}\n\n该操作会覆盖同名文件，可能影响原 Chrome Profile 数据，确认继续？`);

        if (!confirmed) {
          return;
        }
      }

      await api.copyMaster(id);
      showToast('母版 Profile 已同步');
      return;
    }

    if (action === 'open-profile') {
      await api.openPath(environment.profilePath);
      return;
    }

    if (action === 'delete') {
      const confirmed = isDeletableExternalProfile(environment)
        ? window.confirm(`将删除外部 Profile「${environment.name}」及其文件夹：\n${environment.profilePath}\n\n该操作不可恢复，确认删除？`)
        : environment.managed === false
          ? window.confirm(`将环境「${environment.name}」移出管理列表：\n${environment.profilePath}\n\n该 Profile 不属于可删除 Profile，原文件夹不会删除，确认移出？`)
          : window.confirm(`将删除环境「${environment.name}」及其 profile 文件夹：\n${environment.profilePath}\n\n该操作不可恢复，确认删除？`);

      if (!confirmed) {
        return;
      }

      await api.deleteEnvironment(id);
      await loadAndRender();
      showToast(environment.managed === false && isProtectedExternalProfile(environment) ? '环境已移出管理列表' : '环境和 profile 文件夹已删除');
    }
  });
});

nodes.environmentList.addEventListener('dblclick', (event) => {
  const target = event.target.closest('[data-edit-field]');

  if (!target) {
    return;
  }

  const row = target.closest('tr[data-id]');

  if (row) {
    enterEditMode(row, target.dataset.editField);
  }
});

nodes.environmentList.addEventListener('input', (event) => {
  const input = event.target.closest('input[data-field]');

  if (!input) {
    return;
  }

  const row = input.closest('tr[data-id]');

  if (row) {
    scheduleEnvironmentSave(row);
  }
});

nodes.environmentList.addEventListener('blur', (event) => {
  const input = event.target.closest('input[data-field]');

  if (!input) {
    return;
  }

  const row = input.closest('tr[data-id]');

  if (row) {
    const key = `${row.dataset.id}:${input.dataset.field}`;

    if (state.savingFields.has(key)) {
      return;
    }

    window.clearTimeout(state.saveTimers.get(row.dataset.id));
    runTask(async () => {
      const patch = getRowPatch(row);
      await api.updateEnvironment(row.dataset.id, patch);
      const environment = getEnvironment(row.dataset.id);

      if (environment) {
        Object.assign(environment, patch);
      }

      exitEditMode(row, input.dataset.field);
      showToast('修改已自动保存');
    });
  }
}, true);

nodes.environmentList.addEventListener('keydown', (event) => {
  const input = event.target.closest('input[data-field]');

  if (!input || event.key !== 'Enter') {
    return;
  }

  const row = input.closest('tr[data-id]');

  if (row) {
    const key = `${row.dataset.id}:${input.dataset.field}`;

    window.clearTimeout(state.saveTimers.get(row.dataset.id));
    state.savingFields.add(key);
    runTask(async () => {
      try {
        const patch = getRowPatch(row);
        await api.updateEnvironment(row.dataset.id, patch);
        const environment = getEnvironment(row.dataset.id);

        if (environment) {
          Object.assign(environment, patch);
        }

        exitEditMode(row, input.dataset.field);
        showToast('修改已自动保存');
      } finally {
        state.savingFields.delete(key);
      }
    });
  }
});

applyTheme(localStorage.getItem('theme') || 'light');
loadAndRender();
