//! 本地平台实现模块
//!
//! 提供与原生操作系统交互的底层实现，包括：
//! - 文件系统操作
//! - 异步资源加载
//! - 本地存储管理
//! - 图形API交互
//!
//! ## 模块结构
//! | 模块                | 功能描述                   |
//! |---------------------|--------------------------|
//! | compressed_texture  | 压缩纹理处理（DDS/KTX/PVR）|
//! | font_brush          | 字体渲染和排版引擎         |
//! | image               | 图像解码和处理            |
//! | runtime             | 异步运行时集成            |
//! | stroe               | 本地持久化存储管理         |
//! | svg                 | SVG矢量图形处理           |
//!
//! ## 特性要求
//! - 压缩纹理需要启用对应特性
//! - 异步运行时依赖tokio/async-std

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use parking_lot::Mutex;
// use parry2d::shape::Shape;
use pi_async_rt::rt::AsyncValue;
use pi_share::Share;

/// 压缩纹理处理模块
/// 
/// 支持格式：
/// - DDS
/// - KTX
/// - PVR
/// 
/// # 注意
/// 需要启用对应特性来支持特定格式
pub mod compressed_texture;

/// 文件系统操作模块
pub mod file;

/// 字体渲染模块
/// 
/// 提供：
/// - 字体加载与解析
/// - 文本排版布局
/// - GPU加速渲染
pub mod font_brush;

/// 图像处理模块
pub mod image;

/// 异步运行时集成模块
pub mod runtime;

/// 本地存储管理模块
pub mod stroe;

/// SVG矢量图形处理模块
pub mod svg;

/// 纹理加载模块
pub mod image_texture_load;

// /// SDF字体处理模块
// pub mod sdf2_info;

/// 异步操作参数枚举
#[derive(Debug)]
pub enum Arg {
    /// 数值参数
    Number(u64),
    /// 字符串参数
    String(String),
    /// 二进制数据参数
    Buffer(Vec<u8>),
    /// 空参数
    None
}

lazy_static! {
    /// 全局加载回调注册器
    pub static ref LOAD_CB: RwLock<Option<Arc<dyn Fn(String, String, String, Vec<Arg>) + Send + Sync>>> = RwLock::new(None);
    
    /// 异步加载任务映射表
    pub static ref LOAD_MAP: Mutex<HashMap<u64, Vec<AsyncValue<Result<Share<Vec<u8>>, String>>>>> =
        Mutex::new(HashMap::new());
}

/// 初始化加载回调函数
/// 
/// # 参数
/// - `cb`: 实现加载逻辑的回调函数
pub fn init_load_cb(cb: Arc<dyn Fn(String, String, String, Vec<Arg>) + Send + Sync>) {
    *LOAD_CB.write().unwrap() = Some(cb);
}

/// 资源加载完成回调
/// 
/// # 参数
/// - `hash`: 资源唯一标识
/// - `data`: 加载结果（成功包含数据，失败包含错误信息）
pub fn on_load(hash: u64, data: Result<Share<Vec<u8>>, String>) {
    let mut v = LOAD_MAP.lock().remove(&hash).unwrap();
    v.drain(..).for_each(|v| {
        v.set(data.clone());
    });
}

/// 创建异步值句柄
/// 
/// # 参数
/// - `modules`: 模块名称
/// - `func`: 函数名称
/// - `hash`: 资源唯一标识
/// - `args`: 调用参数
/// 
/// # 返回值
/// 返回异步值句柄，可用于等待加载结果
pub fn create_async_value(modules: &str, func: &str, hash: u64, args: Vec<Arg>) -> AsyncValue<Result<Share<Vec<u8>>, String>> {
    let mut is_first = false;
    let r = {
        let mut lock = LOAD_MAP.lock();
        let v = if let Some(vec) = lock.get_mut(&hash) {
            let v = AsyncValue::new();
            vec.push(v.clone());
            v
        } else {
            let v = AsyncValue::new();
            lock.insert(hash, vec![v.clone()]);
            is_first = true;
            v
        };

        v
    };

    if is_first{
        if let Some(cb) = LOAD_CB.read().unwrap().as_ref() {
            cb(modules.to_string(), func.to_string(), hash.to_string(), args);
        }
    }
    r
}
