#[cfg(windows)]
use std::time::Duration;

#[cfg(windows)]
#[cfg(windows)]
#[cfg(windows)]
use windows::Win32::System::Memory::{SetProcessWorkingSetSizeEx, SETPROCESSWORKINGSETSIZEEX_FLAGS};
#[cfg(windows)]
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
#[cfg(windows)]
use windows::Win32::System::Threading::GetCurrentProcess;

// Windows: 触发当前进程工作集收缩
#[cfg(windows)]
pub fn trim_working_set() -> Result<(), String> {
    unsafe {
        let handle = GetCurrentProcess();
        // SetProcessWorkingSetSizeEx(hProcess, (SIZE_T)-1, (SIZE_T)-1, 0) 触发收缩
        match SetProcessWorkingSetSizeEx(
            handle,
            usize::MAX,
            usize::MAX,
            SETPROCESSWORKINGSETSIZEEX_FLAGS(0),
        ) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!("SetProcessWorkingSetSizeEx 失败: {}", e)),
        }
    }
}

// Windows: 获取当前进程工作集大小（字节）
#[cfg(windows)]
pub fn get_working_set_bytes() -> Option<u64> {
    unsafe {
        let handle = GetCurrentProcess();
        let mut counters = PROCESS_MEMORY_COUNTERS::default();
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let ok = K32GetProcessMemoryInfo(
            handle,
            &mut counters as *mut _ as *mut _,
            counters.cb,
        );
        if ok.as_bool() {
            Some(counters.WorkingSetSize as u64)
        } else {
            None
        }
    }
}

// Windows: 启动内存收缩调度器
// - 启动后延迟一次收缩
// - 后台每隔一段时间检查一次，超过阈值则收缩
#[cfg(windows)]
pub fn start_memory_trim_scheduler() {
    const STARTUP_DELAY_MS: u64 = 5000; // 启动后 5s 收缩一次
    const CHECK_INTERVAL_MS: u64 = 5000; // 每 30s 检查一次
    const THRESHOLD_BYTES: u64 = 5 * 1024 * 1024; // 超过 32MB 触发收缩

    // 启动后延迟一次
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(STARTUP_DELAY_MS));
        let _ = trim_working_set();
    });

    // 周期检测 + 收缩
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_millis(CHECK_INTERVAL_MS));
        if let Some(ws) = get_working_set_bytes() {
            if ws > THRESHOLD_BYTES {
                let _ = trim_working_set();
            }
        }
    });
}

// ================= 非 Windows 平台的空实现 =================

#[cfg(not(windows))]
pub fn trim_working_set() -> Result<(), String> { Ok(()) }

#[cfg(not(windows))]
pub fn get_working_set_bytes() -> Option<u64> { None }

#[cfg(not(windows))]
pub fn start_memory_trim_scheduler() {} 