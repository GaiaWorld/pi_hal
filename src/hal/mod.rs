/// 非wasm32版本，使用本地实现
#[cfg(all(not(target_arch="wasm32"), not(feature="empty")))]
mod native;
#[cfg(all(not(target_arch="wasm32"), not(feature="empty")))]
pub use native::*;

// wasm32版本，使用web实现
#[cfg(all(target_arch="wasm32", not(feature="empty")))]
mod web;
#[cfg(all(target_arch="wasm32", not(feature="empty")))]
pub use web::*;

// 也可用feature指定为空实现
#[cfg(feature="empty")]
mod empty;
#[cfg(feature="empty")]
pub use empty::*;