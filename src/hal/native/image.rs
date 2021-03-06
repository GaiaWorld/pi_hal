use std::{ops::{Deref, DerefMut}, io::Error};

pub use image::{DynamicImage, ImageError};
use pi_assets::{asset::{Asset, Handle}, mgr::{AssetMgr, LoadResult}};
use pi_async::rt::AsyncRuntime;
use pi_atom::Atom;
use pi_share::Share;

use crate::runtime::MULTI_MEDIA_RUNTIME;

pub struct ImageRes {
	value: DynamicImage,
	size: usize,
}

impl Deref for ImageRes {
    type Target = DynamicImage;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for ImageRes {

    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl ImageRes {
	pub fn new(value: DynamicImage) -> Self {
		let size = match &value {
			DynamicImage::ImageLuma8(image) => image.width() * image.height() * 4,
			DynamicImage::ImageLumaA8(image) => image.width() * image.height() * 2,
			DynamicImage::ImageRgb8(image) => image.width() * image.height() * 3,
			DynamicImage::ImageRgba8(image) => image.width() * image.height() * 4,
			DynamicImage::ImageBgr8(image) => image.width() * image.height() * 3,
			DynamicImage::ImageBgra8(image) => image.width() * image.height() * 4,

			DynamicImage::ImageLuma16(image) => image.width() * image.height() * 2,
			DynamicImage::ImageLumaA16(image) => image.width() * image.height() * 4,

			DynamicImage::ImageRgb16(image) => image.width() * image.height() * 2 * 3,
			DynamicImage::ImageRgba16(image) => image.width() * image.height() * 2 * 4,
		};
		Self {
			value,
			size: size as usize,
		}
	}
}

impl Asset for ImageRes {
    type Key = Atom;

    fn size(&self) -> usize {
        self.size
    }
}

/// 从本地路径加载图片
pub async fn load_from_path(
	mgr: &Share<AssetMgr<ImageRes>>,
	k: &Atom,
) -> Result<Handle<ImageRes>, LoadError> {
	match AssetMgr::load(mgr, &k) {
		LoadResult::Ok(r) => Ok(r),
		LoadResult::Wait(f) => match f.await {
			Ok(r) => Ok(r),
			Err(e) => Err(LoadError::IoError(e))
		},
		LoadResult::Receiver(recv) => {
			let k1 = k.clone();
			let wait = MULTI_MEDIA_RUNTIME.wait();
			wait.spawn(MULTI_MEDIA_RUNTIME.clone(), None, async move {
				let image = match image::open(k1.as_str()) {
					Ok(r) => r,
					Err(e) => {
						log::error!("load image fail, {:?}", e);
						return Ok(());
					}
				};
				
				if let Err(e) = recv.receive(k1, Ok(ImageRes::new(image))).await {
					log::error!("load image fail, {:?}", e);
				}
				Ok(())
			}).unwrap();
			wait.wait_result().await.unwrap();
			match AssetMgr::get(mgr, k) {
				Some(r) => Ok(r),
				None => Err(LoadError::Other("load fail".to_string()))
			}
		}
	}
}

pub enum LoadError {
	IoError(Error),
	Other(String),
}

pub async fn from_path_or_url(path: &str) -> Result<DynamicImage, image::ImageError> {
	// pat可能是本地路径， 也可能是网络路径，
	// 网络路径TODO
    image::open(path)
}

pub fn from_path(path: &str) -> Result<(Vec<u8>, u32, u32), image::ImageError> {
    let dynamic_image = image::open(path)?;
    let image_buffer = dynamic_image.into_rgba8();
    let (width, height) = image_buffer.dimensions();
    Ok((image_buffer.into_raw(), width, height))
}

pub fn from_memory(buf: &[u8]) -> Result<(Vec<u8>, u32, u32), image::ImageError> {
    let dynamic_image = image::load_from_memory(buf)?;
    let image_buffer = dynamic_image.into_rgba8();
    let (width, height) = image_buffer.dimensions();
    Ok((image_buffer.into_raw(), width, height))
}
