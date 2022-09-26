use std::io::ErrorKind;

pub use image::{DynamicImage, ImageError};
use pi_atom::Atom;

use crate::loadFile;

// path可能是本地路径， 也可能是网络路径，
pub async fn load_from_url(path: &Atom) -> Result<DynamicImage, ImageError> {
	match loadFile(path.get_hash() as u32).await {
		Ok(r) => image::load_from_memory(js_sys::Uint8Array::from(r).to_vec().as_slice()),
		Err(e) => Err(ImageError::IoError(std::io::Error::new(ErrorKind::InvalidFilename, format!("{:?}", e))))
	}
}
