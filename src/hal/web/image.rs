use std::io::ErrorKind;

pub use image::{DynamicImage, ImageBuffer, ImageError};
use pi_atom::Atom;

use crate::{loadImageAsCanvas, hasAtom, setAtom};

// path可能是本地路径， 也可能是网络路径，
#[cfg(not(feature="web_local_load"))]
pub async fn load_from_url(path: &Atom) -> Result<DynamicImage, ImageError> {
	// let is_png = if path.ends_with(".png") {
	// 	true
	// } else {
	// 	false
	// };
	
	let id = path.get_hash() as u32;
	if hasAtom(id) == false {
		setAtom(id, path.to_string());
	}

	match loadImageAsCanvas(path.get_hash() as u32).await {
		Ok(r) => {
			let ctx = web_sys::CanvasRenderingContext2d::from(r);
			let canvas = ctx.canvas().unwrap();
			let img_data = match ctx.get_image_data(0.0, 0.0, canvas.width() as f64, canvas.height() as f64) {
				Ok(r) => r,
				Err(e) => return Err(ImageError::IoError(std::io::Error::new(ErrorKind::InvalidFilename, format!("{:?}", e)))),
			};
			// log::warn!("img_data========{:?}, {:?}, {:?}", img_data.width(), img_data.height());
			// if is_png {
				Ok(DynamicImage::ImageRgba8(ImageBuffer::from_raw(img_data.width(), img_data.height(), img_data.data().0).unwrap()))
			// } else {
			// 	Ok(DynamicImage::ImageRgb8(ImageBuffer::from_raw(img_data.width(), img_data.height(), img_data.data().0).unwrap()))
			// }
			
		},
		Err(e) => Err(ImageError::IoError(std::io::Error::new(ErrorKind::InvalidFilename, format!("{:?}", e))))
	}
}

#[cfg(feature="web_local_load")]
pub use super::web_local::{load_from_url, on_load};