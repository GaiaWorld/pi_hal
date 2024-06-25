use std::{
    hash::{DefaultHasher, Hash, Hasher}, io::Error, ops::{Deref, DerefMut}
};

pub use image::{DynamicImage, ImageError};
use pi_assets::{
    asset::{Asset, Size, Handle},
    mgr::{AssetMgr, LoadResult},
};
use pi_async_rt::rt::AsyncRuntime;
use pi_atom::Atom;
use pi_share::Share;

use crate::{create_async_value, Arg};

use super::runtime::MULTI_MEDIA_RUNTIME;

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
            // DynamicImage::ImageBgr8(image) => image.width() * image.height() * 3,
            // DynamicImage::ImageBgra8(image) => image.width() * image.height() * 4,
            DynamicImage::ImageLuma16(image) => image.width() * image.height() * 2,
            DynamicImage::ImageLumaA16(image) => image.width() * image.height() * 4,

            DynamicImage::ImageRgb16(image) => image.width() * image.height() * 2 * 3,
            DynamicImage::ImageRgba16(image) => image.width() * image.height() * 2 * 4,
            DynamicImage::ImageRgb32F(image) => image.width() * image.height() * 4 * 3,
            DynamicImage::ImageRgba32F(image) => image.width() * image.height() * 4 * 4,
            _ => todo!(),
            // _ => todo!(),
        };
        Self {
            value,
            size: size as usize,
        }
    }
}

impl Asset for ImageRes {
    type Key = Atom;
}

impl Size for ImageRes {
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
            Err(e) => Err(LoadError::IoError(e)),
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
            })
            .unwrap();
            wait.wait_result().await.unwrap();
            match AssetMgr::get(mgr, k) {
                Some(r) => Ok(r),
                None => Err(LoadError::Other("load fail".to_string())),
            }
        }
    }
}

pub enum LoadError {
    IoError(Error),
    Other(String),
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

pub async fn from_path_or_url(path: &str) -> DynamicImage {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let v = create_async_value("file", "", hasher.finish(), vec![Arg::String(path.to_string())]);
    image::load_from_memory(&v.await.unwrap()).unwrap()
}

pub async fn load_from_url(path: &Atom) -> Result<DynamicImage, ImageError> {
	let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let v = create_async_value("file", "", hasher.finish(), vec![Arg::String(path.to_string())]);
	// 此处需要放在多线程运行时中解码(当前运行时可能不是一个多线程运行时)
	let wait = MULTI_MEDIA_RUNTIME.wait::<Result<DynamicImage, ImageError>>();
	wait.spawn(MULTI_MEDIA_RUNTIME.clone(), None, async move {
        let r = match v.await {
            Ok(r) => r,
            Err(e) => {
                return Ok(Err(ImageError::IoError(std::io::Error::other(e))));
            }
        };
		Ok(image::load_from_memory(&r))
	})
	.unwrap();
	wait.wait_result().await.unwrap()
}
