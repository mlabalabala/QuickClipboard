/**
 * 数据管理模块
 */
import { invoke } from '@tauri-apps/api/core';
import { confirm } from '@tauri-apps/plugin-dialog';
import { showNotification } from '../../../js/notificationManager.js';

export class DataManager {
    /**
     * 初始化数据管理功能
     */
    init() {
        this.bindExport();
        this.bindImport();
        this.bindClearHistory();
        this.bindResetAll();
        this.bindStorageManagement();
        this.loadStorageInfo();
    }

    /**
     * 绑定导出数据
     */
    bindExport() {
        const button = document.getElementById('export-all-data');
        if (button) {
            button.addEventListener('click', () => this.handleExportData());
        }
    }

    /**
     * 绑定导入数据
     */
    bindImport() {
        const button = document.getElementById('import-data');
        if (button) {
            button.addEventListener('click', () => this.handleImportData());
        }
    }

    /**
     * 绑定清空历史
     */
    bindClearHistory() {
        const button = document.getElementById('clear-clipboard-history');
        if (button) {
            button.addEventListener('click', () => this.handleClearHistory());
        }
    }

    /**
     * 绑定重置所有数据
     */
    bindResetAll() {
        const button = document.getElementById('reset-all-data');
        if (button) {
            button.addEventListener('click', () => this.handleResetAll());
        }
    }

    /**
     * 绑定存储位置管理
     */
    bindStorageManagement() {
        const openBtn = document.getElementById('open-storage-folder');
        const changeBtn = document.getElementById('change-storage-location');
        const resetBtn = document.getElementById('reset-storage-location');

        if (openBtn) {
            openBtn.addEventListener('click', () => this.handleOpenStorage());
        }
        if (changeBtn) {
            changeBtn.addEventListener('click', () => this.handleChangeStorage());
        }
        if (resetBtn) {
            resetBtn.addEventListener('click', () => this.handleResetStorage());
        }
    }

    /**
     * 处理导出数据
     */
    async handleExportData() {
        try {
            const { save } = await import('@tauri-apps/plugin-dialog');
            const filePath = await save({
                title: '导出全部数据',
                defaultPath: `quickclipboard_backup_${new Date().toISOString().slice(0, 10)}.zip`,
                filters: [{ name: 'ZIP文件', extensions: ['zip'] }]
            });

            if (!filePath) return;

            showNotification('正在导出全部数据，请稍候...', 'info');
            await invoke('export_data', { exportPath: filePath, options: {} });
            showNotification('全部数据导出成功！', 'success');
        } catch (error) {
            console.error('导出数据失败:', error);
            showNotification(`导出数据失败: ${error}`, 'error');
        }
    }

    /**
     * 处理导入数据
     */
    async handleImportData() {
        try {
            const importModeRadios = document.querySelectorAll('input[name="import-mode"]');
            let importMode = 'replace';
            for (const radio of importModeRadios) {
                if (radio.checked) {
                    importMode = radio.value;
                    break;
                }
            }

            const { open } = await import('@tauri-apps/plugin-dialog');
            const filePath = await open({
                title: '选择要导入的数据文件',
                filters: [{ name: 'ZIP文件', extensions: ['zip'] }]
            });

            if (!filePath) return;

            const confirmed = await confirm(
                importMode === 'replace' 
                    ? '导入将替换所有现有数据，此操作不可撤销。是否继续？'
                    : '导入将与现有数据合并。是否继续？',
                { title: '确认导入', kind: 'warning' }
            );

            if (!confirmed) return;

            showNotification('正在导入数据，请稍候...', 'info');
            await invoke('import_data', {
                importPath: filePath,
                options: { mode: importMode === 'replace' ? 'Replace' : 'Merge' }
            });

            showNotification('数据导入成功！', 'success');
            await invoke('refresh_all_windows');
        } catch (error) {
            console.error('导入数据失败:', error);
            showNotification(`导入数据失败: ${error}`, 'error');
        }
    }

    /**
     * 处理清空历史
     */
    async handleClearHistory() {
        const confirmed = await confirm(
            '确定要清空所有剪贴板历史吗？此操作不可撤销。',
            { title: '确认清空历史', kind: 'warning' }
        );

        if (!confirmed) return;

        try {
            showNotification('正在清空剪贴板历史...', 'info');
            await invoke('clear_clipboard_history_dm');
            showNotification('剪贴板历史已清空！', 'success');
            await invoke('refresh_all_windows');
        } catch (error) {
            console.error('清空剪贴板历史失败:', error);
            showNotification(`清空剪贴板历史失败: ${error}`, 'error');
        }
    }

    /**
     * 处理重置所有数据
     */
    async handleResetAll() {
        const firstConfirmed = await confirm(
            '确定要重置所有数据吗？这将删除所有剪贴板历史、常用文本、分组和设置。此操作不可撤销！',
            { title: '确认重置数据', kind: 'warning' }
        );

        if (!firstConfirmed) return;

        const finalConfirmed = await confirm(
            '最后确认：这将完全重置应用到初始状态，所有数据都将丢失。确定继续吗？',
            { title: '最终确认', kind: 'error' }
        );

        if (!finalConfirmed) return;

        try {
            showNotification('正在重置所有数据...', 'info');
            await invoke('reset_all_data');
            showNotification('所有数据已重置！', 'success');
            await invoke('refresh_all_windows');
        } catch (error) {
            console.error('重置所有数据失败:', error);
            showNotification(`重置所有数据失败: ${error}`, 'error');
        }
    }

    /**
     * 处理打开存储文件夹
     */
    async handleOpenStorage() {
        try {
            await invoke('open_storage_folder');
        } catch (error) {
            console.error('打开存储文件夹失败:', error);
            showNotification(`打开存储文件夹失败: ${error}`, 'error');
        }
    }

    /**
     * 处理更改存储位置
     */
    async handleChangeStorage() {
        try {
            const { open } = await import('@tauri-apps/plugin-dialog');
            const selectedPath = await open({
                title: '选择新的数据存储位置',
                directory: true,
                multiple: false
            });

            if (!selectedPath) return;

            const confirmed = await confirm(
                '更改存储位置将迁移所有现有数据到新位置，此过程可能需要一些时间。确定继续吗？',
                { title: '确认更改存储位置', type: 'warning' }
            );

            if (!confirmed) return;

            showNotification('正在迁移数据到新位置，请稍候...', 'info');
            await invoke('set_custom_storage_location', { newPath: selectedPath });
            await this.loadStorageInfo();
            showNotification('存储位置更改成功！', 'success');
        } catch (error) {
            console.error('更改存储位置失败:', error);
            showNotification(`更改存储位置失败: ${error}`, 'error');
        }
    }

    /**
     * 处理重置存储位置
     */
    async handleResetStorage() {
        try {
            const confirmed = await confirm(
                '重置存储位置将把数据迁移回默认的AppData目录。确定继续吗？',
                { title: '确认重置存储位置', type: 'warning' }
            );

            if (!confirmed) return;

            showNotification('正在重置存储位置，请稍候...', 'info');
            await invoke('reset_to_default_storage_location');
            await this.loadStorageInfo();
            showNotification('存储位置已重置为默认位置！', 'success');
        } catch (error) {
            console.error('重置存储位置失败:', error);
            showNotification(`重置存储位置失败: ${error}`, 'error');
        }
    }

    /**
     * 加载存储信息
     */
    async loadStorageInfo() {
        try {
            const storageInfo = await invoke('get_storage_info');
            const currentPathElement = document.getElementById('current-storage-path');
            if (currentPathElement) {
                currentPathElement.textContent = storageInfo.current_path;
                currentPathElement.title = storageInfo.current_path;
            }
        } catch (error) {
            console.error('获取存储信息失败:', error);
            const currentPathElement = document.getElementById('current-storage-path');
            if (currentPathElement) {
                currentPathElement.textContent = '获取存储位置失败';
            }
        }
    }
}
