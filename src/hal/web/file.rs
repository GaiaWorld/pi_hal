use std::io::ErrorKind;
use pi_atom::Atom;
use wasm_bindgen::JsValue;
use pi_share::Share;
use crate::{loadFile, hasAtom, setAtom};

#[derive(Debug, Clone)]
pub struct FileLoadErr(JsValue);

// path可能是本地路径， 也可能是网络路径，
#[cfg(not(feature="web_local_load"))]
pub async fn load_from_url(path: &Atom) -> Result<Share<Vec<u8>>, FileLoadErr> {
	let id = path.str_hash() as u32;
	if hasAtom(id) == false {
		setAtom(id, path.to_string());
	}
	match loadFile(path.str_hash() as u32).await {
		Ok(r) => {
			Ok(Share::new(js_sys::Uint8Array::from(r).to_vec()))
		},
		Err(e) => Err(FileLoadErr(e))
	}
}


#[cfg(feature="web_local_load")]
pub use super::web_local::load_file_from_url as load_from_url;
