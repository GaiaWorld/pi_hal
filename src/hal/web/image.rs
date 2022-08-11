use std::{
    io::Error,
    ops::{Deref, DerefMut},
};

pub use image::{DynamicImage, ImageError};
use pi_assets::{
    asset::{Asset, Handle},
    mgr::{AssetMgr, LoadResult},
};
use pi_async::rt::{AsyncRuntime, AsyncValue};
use pi_atom::Atom;
use pi_share::Share;
use std::{collections::HashMap, sync::Arc};

use crate::runtime::MULTI_MEDIA_RUNTIME;
use parking_lot::{Mutex, RwLock};

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
	Err(LoadError::Other("".to_string()))
}

pub enum LoadError {
    IoError(Error),
    Other(String),
}

pub fn from_path(path: &str) -> Result<(Vec<u8>, u32, u32), image::ImageError> {
    todo!()
}

pub fn from_memory(buf: &[u8]) -> Result<(Vec<u8>, u32, u32), image::ImageError> {
    todo!()
}

pub fn init_image_cb(cb: Arc<dyn Fn(String) + Send + Sync>) {
    
}

pub async fn from_path_or_url(path: &str) -> DynamicImage {
    todo!()
}

pub fn on_load(path: &str, image: image::DynamicImage) {

}
