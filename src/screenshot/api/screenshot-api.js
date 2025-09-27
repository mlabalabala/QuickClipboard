/**
 * 截屏后端API调用模块
 * 负责与后端Tauri命令的通信
 */

const { invoke } = window.__TAURI__.core;

export class ScreenshotAPI {
    /**
     * 获取CSS像素格式的显示器信息
     */
    static async getMonitors() {
        try {
            return await invoke('get_css_monitors');
        } catch (error) {
            console.error('获取显示器信息失败:', error);
            // 返回默认单显示器配置
            return [{
                x: 0,
                y: 0,
                width: window.innerWidth,
                height: window.innerHeight,
                is_primary: true
            }];
        }
    }

    /**
     * 约束选区或工具栏位置到合适的显示器边界内
     */
    static async constrainBounds(x, y, width, height) {
        try {
            const [constrainedX, constrainedY] = await invoke('constrain_selection_bounds', {
                x, y, width, height
            });
            return { x: constrainedX, y: constrainedY };
        } catch (error) {
            console.error('边界约束失败:', error);
            // 降级到简单边界检查
            return {
                x: Math.max(0, Math.min(x, window.innerWidth - width)),
                y: Math.max(0, Math.min(y, window.innerHeight - height))
            };
        }
    }

    /**
     * 显示截屏窗口
     */
    static async showWindow() {
        return await invoke('show_screenshot_window');
    }

    /**
     * 隐藏截屏窗口
     */
    static async hideWindow() {
        return await invoke('hide_screenshot_window');
    }

}
