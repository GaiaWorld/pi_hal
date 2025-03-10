/// 本地平台实现模块
/// 
/// 当编译目标不是wasm32架构且未启用empty特性时生效，包含：
/// - 本地文件系统操作
/// - 原生图形API接口
/// - 平台相关的硬件抽象实现
/// 
/// # 示例
/// ```
/// use pi_hal::*; // 根据编译条件自动导入native或web实现
/// ```
#[cfg(all(not(target_arch="wasm32"), not(feature="empty")))]
mod native;
#[cfg(all(not(target_arch="wasm32"), not(feature="empty")))]
pub use native::*;

/// Web平台实现模块
/// 
/// 当编译目标为wasm32架构且未启用empty特性时生效，包含：
/// - WebGL/WebGPU图形接口
/// - 浏览器存储访问
/// - Web Worker通信
/// - 基于Fetch API的网络请求
/// 
/// # 注意
/// 需要配合`wasm-bindgen`使用，适用于浏览器环境
#[cfg(all(target_arch="wasm32", not(feature="empty")))]
mod web;
#[cfg(all(target_arch="wasm32", not(feature="empty")))]
pub use web::*;

/// 空实现模块
/// 
/// 当启用empty特性时生效，提供：
/// - 空操作的mock实现
/// - 用于测试的虚拟接口
/// - 不依赖具体平台的桩实现
/// 
/// # 使用场景
/// - 单元测试
/// - 文档示例
/// - 无硬件依赖的轻量级应用
#[cfg(feature="empty")]
mod empty;
#[cfg(feature="empty")]
pub use empty::*;
