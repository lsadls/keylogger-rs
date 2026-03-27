// ============================================================================
// Windows API 类型定义模块
// ============================================================================
// 本模块定义了 Windows API 中常用的类型别名和常量
// 
// Windows API 使用特定的数据类型，这些类型在 Rust 中需要对应定义
// 使用类型别名可以让代码更清晰，也更容易理解
// 
// 主要内容：
// 1. 基本类型别名（HHOOK、HWND、WPARAM 等）
// 2. 消息常量（WM_KEYDOWN、WM_SYSKEYDOWN 等）
// 3. 钩子类型常量（WH_KEYBOARD_LL）
// 4. 虚拟键码常量（VK_RETURN、VK_SPACE 等）
// 5. 结构体定义（KBDLLHOOKSTRUCT、MSG）
// ============================================================================

// ============================================================================
// 类型别名定义
// ============================================================================
// 这些类型别名对应 Windows API 中的基本类型

/// 钩子句柄类型
/// 用于标识一个安装的钩子
pub type HHOOK = isize;

/// 窗口句柄类型
/// 用于标识一个窗口
pub type HWND = isize;

/// 消息参数类型（WPARAM）
/// Windows 消息的第一个参数
/// 通常用于传递小整数或句柄
pub type WPARAM = usize;

/// 消息参数类型（LPARAM）
/// Windows 消息的第二个参数
/// 通常用于传递指针或较大的值
pub type LPARAM = isize;

/// 窗口过程返回值类型
/// 窗口过程函数的返回值类型
pub type LRESULT = isize;

/// 双字类型（32位无符号整数）
/// Windows API 中常用的无符号整数类型
pub type DWORD = u32;

/// 布尔类型
/// Windows API 中的布尔类型
/// 非 0 表示 TRUE，0 表示 FALSE
pub type BOOL = i32;

/// 实例句柄类型
/// 用于标识一个模块或实例
pub type HINSTANCE = isize;

// ============================================================================
// 消息常量定义
// ============================================================================

/// 按键按下消息
/// 当用户按下键盘上的任意键时发送
/// 值：0x0100 = 256
pub const WM_KEYDOWN: u32 = 0x0100;

/// 系统按键按下消息
/// 当用户按下 Alt+任意键 时发送
/// 值：0x0104 = 260
pub const WM_SYSKEYDOWN: u32 = 0x0104;

// ============================================================================
// 钩子类型常量定义
// ============================================================================

/// 低级键盘钩子类型
/// 用于 SetWindowsHookExA 函数的第一个参数
/// 值：13
/// 
/// 低级键盘钩子可以捕获所有键盘输入
/// 包括系统快捷键（如 Alt+Tab）
pub const WH_KEYBOARD_LL: i32 = 13;

// ============================================================================
// 虚拟键码常量定义
// ============================================================================
// 虚拟键码是 Windows 为每个按键分配的唯一标识符
// 这些常量用于识别特定的按键

// 注释掉的键码：这些键码在 key_handler.rs 中使用数字直接匹配
// pub const VK_SHIFT: u32 = 0x10;
// pub const VK_CONTROL: u32 = 0x11;
// pub const VK_MENU: u32 = 0x12; // Alt 键

/// Caps Lock 键
/// 大写锁定键
pub const VK_CAPITAL: u32 = 0x14;

/// 左 Windows 键
pub const VK_LWIN: u32 = 0x5B;

/// 右 Windows 键
pub const VK_RWIN: u32 = 0x5C;

/// Enter/Return 键
pub const VK_RETURN: u32 = 0x0D;

/// 退格键
pub const VK_BACK: u32 = 0x08;

/// Tab 键
pub const VK_TAB: u32 = 0x09;

/// 空格键
pub const VK_SPACE: u32 = 0x20;

/// Escape 键
pub const VK_ESCAPE: u32 = 0x1B;

/// 左方向键
pub const VK_LEFT: u32 = 0x25;

/// 上方向键
pub const VK_UP: u32 = 0x26;

/// 右方向键
pub const VK_RIGHT: u32 = 0x27;

/// 下方向键
pub const VK_DOWN: u32 = 0x28;

/// Insert 键
pub const VK_INSERT: u32 = 0x2D;

/// Delete 键
pub const VK_DELETE: u32 = 0x2E;

/// Home 键
pub const VK_HOME: u32 = 0x24;

/// End 键
pub const VK_END: u32 = 0x23;

/// Page Up 键
pub const VK_PRIOR: u32 = 0x21;

/// Page Down 键
pub const VK_NEXT: u32 = 0x22;

/// F1 功能键
pub const VK_F1: u32 = 0x70;

/// F2 功能键
pub const VK_F2: u32 = 0x71;

/// F3 功能键
pub const VK_F3: u32 = 0x72;

/// F4 功能键
pub const VK_F4: u32 = 0x73;

/// F5 功能键
pub const VK_F5: u32 = 0x74;

/// F6 功能键
pub const VK_F6: u32 = 0x75;

/// F7 功能键
pub const VK_F7: u32 = 0x76;

/// F8 功能键
pub const VK_F8: u32 = 0x77;

/// F9 功能键
pub const VK_F9: u32 = 0x78;

/// F10 功能键
pub const VK_F10: u32 = 0x79;

/// F11 功能键
pub const VK_F11: u32 = 0x7A;

/// F12 功能键
pub const VK_F12: u32 = 0x7B;

/// Print Screen 键
pub const VK_SNAPSHOT: u32 = 0x2C;

/// Scroll Lock 键
pub const VK_SCROLL: u32 = 0x91;

/// Pause 键
pub const VK_PAUSE: u32 = 0x13;

/// Num Lock 键
pub const VK_NUMLOCK: u32 = 0x90;

// ============================================================================
// 结构体定义
// ============================================================================

/// 低级键盘钩子结构体
/// 
/// 当安装低级键盘钩子后，每次键盘事件都会传递这个结构体
/// 包含键盘事件的详细信息
/// 
/// 对应 Windows API 中的 KBDLLHOOKSTRUCT 结构体
#[repr(C)]  // 使用 C 语言内存布局
pub struct KBDLLHOOKSTRUCT {
    /// 虚拟键码
    /// Windows 为每个按键分配的唯一标识符
    /// 例如：A=65, B=66, Enter=13
    pub vk_code: u32,
    
    /// 硬件扫描码
    /// 键盘硬件产生的原始扫描码
    /// 通常用于识别具体的物理按键
    pub scan_code: u32,
    
    /// 标志位
    /// 包含各种标志信息：
    /// - LLKHF_EXTENDED (0x01): 扩展键（如右侧 Alt、Ctrl）
    /// - LLKHF_INJECTED (0x10): 人工注入的按键
    /// - LLKHF_ALTDOWN (0x20): Alt 键按下
    /// - LLKHF_UP (0x80): 按键释放
    pub flags: u32,
    
    /// 时间戳
    /// 消息产生的时间（毫秒）
    pub time: u32,
    
    /// 额外信息
    /// 与消息关联的额外信息
    pub dw_extra_info: usize,
}

/// Windows 消息结构体
/// 
/// Windows 消息系统的核心结构体
/// 包含一条消息的所有信息
/// 
/// 对应 Windows API 中的 MSG 结构体
#[repr(C)]
pub struct MSG {
    /// 接收消息的窗口句柄
    pub hwnd: HWND,
    
    /// 消息标识符
    /// 标识消息的类型，如 WM_KEYDOWN、WM_PAINT 等
    pub message: u32,
    
    /// 消息参数1
    /// 具体含义取决于消息类型
    pub w_param: WPARAM,
    
    /// 消息参数2
    /// 具体含义取决于消息类型
    pub l_param: LPARAM,
    
    /// 消息产生的时间
    pub time: u32,
    
    /// 光标位置的 X 坐标
    /// 消息产生时的光标位置
    pub pt_x: i32,
    
    /// 光标位置的 Y 坐标
    pub pt_y: i32,
}
