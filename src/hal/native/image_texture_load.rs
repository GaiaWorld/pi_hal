//! 图片纹理加载模块
//!
//! 提供从URL加载普通图片和压缩纹理的功能，支持多种图片格式和GPU纹理格式转换。

use std::io::ErrorKind;

use image::DynamicImage;
pub use image::ImageError;
use ktx::KtxInfo;
use pi_wgpu::{self as wgpu, util::{DeviceExt, TextureDataOrder}};

use crate::texture::{convert_format, depth_or_array_layers, dimension, view_dimension, ImageTexture, ImageTextureDesc, KTX_SUFF};

/// 从URL加载图片纹理
///
/// # 参数
/// - `desc`: 纹理描述信息，包含URL、使用标志等
/// - `device`: WGPU设备实例
/// - `queue`: WGPU命令队列
/// 
/// # 返回值
/// 返回`Result<ImageTexture, ImageError>`: 成功时包含纹理数据，失败时返回图像错误
/// 
/// # 功能
/// 根据文件后缀自动选择加载方式：
/// - `.ktx` 后缀使用压缩纹理加载
/// - 其他后缀使用普通图片加载
pub async fn load_from_url(desc: &ImageTextureDesc, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<ImageTexture, ImageError> {
    if desc.url.ends_with(KTX_SUFF) {
        load_compress_from_url(desc, device, queue).await
    } else {
        load_common_from_url(desc, device, queue).await
    }
}

/// 加载普通图片纹理
///
/// # 参数
/// - `desc`: 纹理描述信息
/// - `device`: WGPU设备实例
/// - `queue`: WGPU命令队列
///
/// # 返回值
/// 返回`Result<ImageTexture, ImageError>`: 成功时包含纹理数据，失败时返回图像错误
///
/// # 处理流程
/// 1. 从URL加载图片数据
/// 2. 根据图片格式转换像素数据
/// 3. 创建GPU纹理资源
/// 4. 将像素数据上传到GPU
/// 5. 返回纹理对象及相关元数据
async fn load_common_from_url(desc: &ImageTextureDesc, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<ImageTexture, ImageError> {
    let image = crate::image::load_from_url(&desc.url).await?;
    let is_opacity = desc.url.ends_with(".png");

    let buffer_temp;
	// let buffer_temp1;
	let (width, height, buffer, format, pre_pixel_size) = match &image {
		DynamicImage::ImageLuma8(image) => (image.width(), image.height(), image.as_raw(), wgpu::TextureFormat::R8Unorm, 1),
		DynamicImage::ImageRgb8(r) => {
			buffer_temp =  image.to_rgba8();
			(r.width(), r.height(), buffer_temp.as_raw(), if desc.srgb { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm }, 4)
		},
		DynamicImage::ImageRgba8(image) => (image.width(), image.height(), image.as_raw(), if desc.srgb  { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm }, 4),
		_ => panic!("不支持的图片格式"),
	};
	let texture_extent = wgpu::Extent3d {
		width,
		height,
		depth_or_array_layers: 1,
	};

	// log::warn!("create_texture==========={:?}, {:?}", key, std::thread::current().id());
	let texture = device.create_texture(&wgpu::TextureDescriptor {
		label: Some("image texture"),
		size: texture_extent,
		mip_level_count: 1,
		sample_count: 1,
		dimension: wgpu::TextureDimension::D2,
		format,
		usage: desc.useage,
		view_formats: &[],
	});

	queue.write_texture(
		texture.as_image_copy(),
		buffer,
		wgpu::ImageDataLayout {
			offset: 0,
			bytes_per_row: Some(width * pre_pixel_size),
			rows_per_image: None,
		},
		texture_extent,
	);

    Ok(ImageTexture {
        texture, is_opacity,
        width, height, format,
        size: pre_pixel_size as usize * width as usize * height as usize,
        view_dimension: wgpu::TextureViewDimension::D2,
    })
}

/// 加载压缩纹理（KTX格式）
///
/// # 参数
/// - `desc`: 纹理描述信息
/// - `device`: WGPU设备实例
/// - `queue`: WGPU命令队列
///
/// # 返回值
/// 返回`Result<ImageTexture, ImageError>`: 成功时包含纹理数据，失败时返回图像错误
///
/// # 处理流程
/// 1. 从URL加载KTX文件数据
/// 2. 解析KTX文件头信息
/// 3. 转换GPU纹理格式
/// 4. 计算纹理尺寸和层级信息
/// 5. 合并多级mipmap数据
/// 6. 创建GPU纹理并上传数据
/// 7. 返回纹理对象及相关元数据
async fn load_compress_from_url(desc: &ImageTextureDesc, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<ImageTexture, ImageError> {
    // 加载ktx
    let buffer = crate::file::load_from_url(&desc.url).await;
    let buffer = match buffer {
        Ok(r) => r,
        Err(e) =>  {
            log::error!("load file fail: {:?}", desc.url.as_str());
            return Err(ImageError::IoError(std::io::Error::new(ErrorKind::InvalidFilename, format!("{:?}", e))));
        },
    };

    let ktx = ktx::Ktx::new(buffer.as_slice());
    let format = convert_format(ktx.gl_internal_format());
    let mip_level_count = ktx.mipmap_levels().max(1);
    let layer_count = ktx.array_elements().max(1);
    let face_count = ktx.faces().max(1);
    

	let texture_extent = wgpu::Extent3d {
		width: ktx.pixel_width(),
		height: ktx.pixel_height(),
		depth_or_array_layers: depth_or_array_layers(layer_count, face_count, ktx.pixel_depth()),
	}.physical_size(format);
	log::warn!("width====={:?}, height==={:?}", texture_extent.width, texture_extent.height);

	// let byte_size = buffer.len();
	// let mut textures = decoder.read_textures();

    let mut data1: Vec<u8>;
    let data: &[u8];
    if mip_level_count == 1 && layer_count == 1 && face_count == 1 {
        data = ktx.texture_level(0)
    } else {
        let levels = ktx.textures();
        let mut list = Vec::with_capacity(mip_level_count as usize);
        let mut len = 0;
        for item in levels {
            list.push(item);
            len += item.len();
        }

        data1 = Vec::with_capacity(len);
        for item in list {
            data1.extend_from_slice(item)
        }
        data = data1.as_slice();
    }

	let texture = (device).create_texture_with_data(queue, &wgpu::TextureDescriptor {
		label: Some("ktx texture"),
		size: texture_extent,
		mip_level_count: mip_level_count, // TODO
		sample_count: 1,
		dimension: dimension(ktx.pixel_height(), ktx.pixel_depth()),
		format,
		usage: desc.useage,
		view_formats: &[],
	}, TextureDataOrder::MipMajor, data);


    Ok(ImageTexture {
        texture, is_opacity: true/*TODO*/,
        width: ktx.pixel_width(), height: ktx.pixel_width(), format,
        size: buffer.len(),
        view_dimension: view_dimension(layer_count, face_count, ktx.pixel_depth()),
    })
}
