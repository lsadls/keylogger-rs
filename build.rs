fn main() {
    // 设置 Windows 子系统，不显示控制台窗口
    println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
    // 设置入口点为 mainCRTStartup 而不是 WinMainCRTStartup
    println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
}
