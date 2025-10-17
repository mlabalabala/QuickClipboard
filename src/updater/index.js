/**
 * 更新器模块入口
 */

import { Updater } from './updater-core.js';

// 创建并导出单例
const updater = new Updater();

export default updater;
export { updater };

