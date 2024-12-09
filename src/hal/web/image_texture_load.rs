//! 图片类型的纹理加载
use std::io::ErrorKind;
pub use image::{ImageError, error::{DecodingError, ImageFormatHint}};
use pi_atom::Atom;
use pi_wgpu::Texture;
use pi_wgpu as wgpu;
use pi_wgpu::{ImageCopyExternalImage, ExternalImageSource, PredefinedColorSpace};
// use pi_wgpu::TextureDataOrder;
// use pi_wgpu::util::DeviceExt;
use crate::{loadKtx, loadImage, hasAtom, setAtom};
use crate::texture::{convert_format, ImageTexture, PiDefaultTextureFormat, KTX_SUFF, view_dimension, depth_or_array_layers, dimension, ImageTextureDesc};


// 用一个url图片纹理
pub async fn load_from_url(desc: &ImageTextureDesc, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<ImageTexture, ImageError> {
    if desc.url.ends_with(KTX_SUFF) {
        load_compress_from_url(desc, device, queue).await
    } else {
        load_common_from_url(desc, device, queue).await
    }
}

// 加载普通《图片纹理》
async fn load_common_from_url(desc: &ImageTextureDesc, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<ImageTexture, ImageError> {
    let id = desc.url.str_hash() as u32;
	if hasAtom(id) == false {
		setAtom(id, desc.url.to_string());
	}

	let is_opacity = desc.url.ends_with(".png");

	let format = if is_opacity || desc.url.ends_with(".jpg") { // 注意， 这里加载出来的.jpg也必须是rgba四通道
	    if desc.srgb { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm }
	} else {
		panic!("unimplemented load, {:?}", desc.url.as_str());
	};

	let image = match loadImage(desc.url.str_hash() as u32).await {
		Ok(r) => web_sys::HtmlImageElement::from(r),
		Err(e) => return Err(ImageError::IoError(std::io::Error::new(ErrorKind::InvalidFilename, format!("{:?}", e))))
	};
	let (width, height) = (image.width(), image.height());

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
	// todo!();

	// queue.copy_external_image_to_texture(
	// 	&ImageCopyExternalImage{
	// 		source: ExternalImageSource::HTMLImageElement(image),
	// 		origin: wgpu::Origin2d::ZERO,
	// 		flip_y: false,
	// 	},
	// 	texture.as_image_copy().to_tagged(PredefinedColorSpace::DisplayP3, false),
	// 	texture_extent,
	// );

    Ok(ImageTexture {
        texture, is_opacity,
		width, height, format,
		size: 4 * width as usize * height as usize,
		view_dimension: wgpu::TextureViewDimension::D2,
    })
}

// 加载压缩纹理《图片纹理》
async fn load_compress_from_url(desc: &ImageTextureDesc, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<ImageTexture, ImageError> {
    // 加载ktx
    let id = desc.url.str_hash() as u32;
	if hasAtom(id) == false {
		setAtom(id, desc.url.to_string());
	}
	match loadKtx(desc.url.str_hash() as u32).await {
		Ok(r) => {
			let r: js_sys::Array = r.into(); // [width, height, depth, format, minmap_count, layer_count, face_count, [Data]]
			let width = r.get(0).as_f64().unwrap() as u32;
			let height = r.get(1).as_f64().unwrap() as u32;
			let depth = (r.get(2).as_f64().unwrap() as u32).max(1);
			let format = r.get(3).as_f64().unwrap() as u32;
			let mipmap_count = (r.get(4).as_f64().unwrap() as u32).max(1);
			let layer_count = (r.get(5).as_f64().unwrap() as u32).max(1);
			let face_count = (r.get(6).as_f64().unwrap() as u32).max(1);
			let data: js_sys::Array = r.get(7).into();

			let mut buffers: Vec<js_sys::Object> = Vec::with_capacity(data.length() as usize);
			let mut len = 0;
			for i in 0..data.length() {
				let buffer: js_sys::Uint8Array = data.get(i).into();
				len += buffer.byte_length() as usize;
				buffers.push(data.get(i).into());
			}

			let format = convert_format(format);

			let texture_extent = wgpu::Extent3d {
				width,
				height,
				depth_or_array_layers: depth_or_array_layers(layer_count, face_count, depth),
			}.physical_size(format);
			

			log::debug!("create_texture_from_ktx, width====={:?}, height==={:?}", texture_extent.width, texture_extent.height);
			todo!();

			// let texture = device.create_compress_texture_with_data_jsdata(queue, &wgpu::TextureDescriptor {
			// 	label: Some("ktx texture"),
			// 	size: texture_extent,
			// 	mip_level_count: mipmap_count,
			// 	sample_count: 1,
			// 	dimension: dimension(height, depth),
			// 	format,
			// 	usage: desc.useage,
			// 	view_formats: &[],
			// }, TextureDataOrder::MipMajor, buffers.as_slice());

		
		
			// return Ok(ImageTexture {
			// 	texture, is_opacity: true/*TODO*/,
			// 	width, height, format,
			// 	size: len,
			// 	view_dimension: view_dimension(layer_count, face_count, depth),
			// })
		},
		Err(e) => return Err(ImageError::IoError(std::io::Error::new(ErrorKind::InvalidFilename, format!("{:?}", e))))
	}
}
