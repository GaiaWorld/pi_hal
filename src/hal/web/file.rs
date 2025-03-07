use std::io::ErrorKind;
use pi_atom::Atom;
use wasm_bindgen::JsValue;
use pi_share::Share;
use crate::{loadFile, hasAtom, setAtom};
use std::mem::transmute;

/// 文件加载错误类型，封装了底层JS错误值
#[derive(Debug, Clone)]
pub struct FileLoadErr(JsValue);

/// 从指定URL异步加载文件数据
///
/// # 参数
/// * `path` - 文件路径，可以是本地路径或网络路径
///
/// # 返回值
/// 返回`Result`包含共享的字节数据或加载错误
///
/// # 特性说明
/// 当未启用`web_local_load`特性时使用此实现
#[cfg(not(feature="web_local_load"))]
pub async fn load_from_url(path: &Atom) -> Result<Share<Vec<u8>>, FileLoadErr> {
	let id = unsafe {transmute::<_, f64>(path.str_hash())};
	if hasAtom(id) == false {
		setAtom(id, path.to_string());
	}
	match loadFile(id).await {
		Ok(r) => {
			Ok(Share::new(js_sys::Uint8Array::from(r).to_vec()))
		},
		Err(e) => Err(FileLoadErr(e))
	}
}


#[cfg(feature="web_local_load")]
pub use super::web_local::load_file_from_url as load_from_url;
