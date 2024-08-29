
use pi_atom::Atom;
use pi_wgpu::{self as wgpu, AstcBlock, AstcChannel, TextureDimension, TextureViewDimension};

pub trait PiDefaultTextureFormat {
    fn pi_render_default() -> Self;

	fn is_srgb() -> bool;
}

impl PiDefaultTextureFormat for wgpu::TextureFormat {
    fn pi_render_default() -> Self {
		
        if cfg!(target_os = "android") || cfg!(target_arch = "wasm32") {
            // Bgra8UnormSrgb texture missing on some Android devices
            // wgpu::TextureFormat::Rgba8UnormSrgb
			wgpu::TextureFormat::Rgba8Unorm
        } else  {
			// wgpu::TextureFormat::Bgra8UnormSrgb
            wgpu::TextureFormat::Bgra8Unorm
        }
		
    }

	fn is_srgb() -> bool {
		false
        // if cfg!(target_os = "android") || cfg!(target_arch = "wasm32") {
        //     // Bgra8UnormSrgb texture missing on some Android devices
        //     true
        // } else  {
        //     false
        // }

    }
}

/// 图片纹理
pub struct ImageTexture {
    // 纹理
    pub texture: wgpu::Texture,
    // 是否不透明
    pub is_opacity: bool,
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
    pub size: usize,
    pub view_dimension: wgpu::TextureViewDimension,
}

pub fn convert_format(v: u32) -> wgpu::TextureFormat {
	match v {
		// 0x83f0 => wgpu::TextureFormat::Bc1RgbUnorm,// GL_COMPRESSED_RGB_S3TC_DXT1_EXT	0x83f0     GL_COMPRESSED_RGB_S3TC_DXT1_EXT	Bc1RgbUnorm
		0x83f1 => wgpu::TextureFormat::Bc1RgbaUnorm,// GL_COMPRESSED_RGBA_S3TC_DXT1_EXT	0x83f1     GL_COMPRESSED_RGBA_S3TC_DXT1_EXT	Bc1RgbaUnorm
		0x83f2 => wgpu::TextureFormat::Bc2RgbaUnorm,// GL_COMPRESSED_RGBA_S3TC_DXT3_EXT	0x83f2     GL_COMPRESSED_RGBA_S3TC_DXT3_EXT	Bc2RgbaUnorm
		0x83f3 => wgpu::TextureFormat::Bc3RgbaUnorm,// GL_COMPRESSED_RGBA_S3TC_DXT5_EXT	0x83f3     GL_COMPRESSED_RGBA_S3TC_DXT5_EXT	Bc3RgbaUnorm
		0x9274 => wgpu::TextureFormat::Etc2Rgb8Unorm,// GL_COMPRESSED_RGB8_ETC2	0x9274             GL_COMPRESSED_RGB8_ETC2	Etc2Rgb8Unorm
		0x9278 => wgpu::TextureFormat::Etc2Rgba8Unorm,// GL_COMPRESSED_RGBA8_ETC2_EAC	0x9278         GL_COMPRESSED_RGBA8_ETC2_EAC	Etc2Rgba8Unorm

		// 0x8c00 => wgpu::TextureFormat::Bc1RgbaUnorm,// GL_COMPRESSED_RGB_PVRTC_4BPPV1_IMG	0x8c00  GL_COMPRESSED_RGB_PVRTC_4BPPV1_IMG	PvrtcRgb4bppUnorm 
		// 0x8c01 => wgpu::TextureFormat::Bc1RgbaUnorm,// GL_COMPRESSED_RGB_PVRTC_2BPPV1_IMG	0x8c01 GL_COMPRESSED_RGB_PVRTC_2BPPV1_IMG	PvrtcRgb2bppUnorm 
		// 0x8c02 => wgpu::TextureFormat::Bc1RgbaUnorm,// GL_COMPRESSED_RGBA_PVRTC_4BPPV1_IMG	0x8c02 UnormGL_COMPRESSED_RGBA_PVRTC_4BPPV1_IMG	PvrtcRgba4bppUnorm
		// 0x8c03 => wgpu::TextureFormat::Bc1RgbaUnorm,// GL_COMPRESSED_RGBA_PVRTC_2BPPV1_IMG	0x8c03 GL_COMPRESSED_RGBA_PVRTC_2BPPV1_IMG	PvrtcRgba2bppUnorm 

		0x93b0 => wgpu::TextureFormat::Astc { block: AstcBlock::B4x4, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_4x4_KHR	0x93b0     GL_COMPRESSED_RGBA_ASTC_4x4_KHR	Astc4x4Unorm 
		0x93b1 => wgpu::TextureFormat::Astc { block: AstcBlock::B5x4, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_5x4_KHR	0x93b1     GL_COMPRESSED_RGBA_ASTC_5x4_KHR	Astc5x4Unorm 
		0x93b2 => wgpu::TextureFormat::Astc { block: AstcBlock::B5x5, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_5x5_KHR	0x93b2     GL_COMPRESSED_RGBA_ASTC_5x5_KHR	Astc5x5Unorm
		0x93b3 => wgpu::TextureFormat::Astc { block: AstcBlock::B6x5, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_6x5_KHR	0x93b3     GL_COMPRESSED_RGBA_ASTC_6x5_KHR	Astc6x5Unorm 
		0x93b4 => wgpu::TextureFormat::Astc { block: AstcBlock::B6x6, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_6x6_KHR	0x93b4     GL_COMPRESSED_RGBA_ASTC_6x6_KHR	Astc6x6Unorm 
		0x93b5 => wgpu::TextureFormat::Astc { block: AstcBlock::B8x5, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_8x5_KHR	0x93b5     GL_COMPRESSED_RGBA_ASTC_8x5_KHR	Astc8x5Unorm 
		0x93b6 => wgpu::TextureFormat::Astc { block: AstcBlock::B8x6, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_8x6_KHR	0x93b6     GL_COMPRESSED_RGBA_ASTC_8x6_KHR	Astc8x6Unorm 
		0x93b7 => wgpu::TextureFormat::Astc { block: AstcBlock::B8x8, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_8x8_KHR	0x93b7     GL_COMPRESSED_RGBA_ASTC_8x8_KHR	Astc8x8Unorm 
		0x93b8 => wgpu::TextureFormat::Astc { block: AstcBlock::B10x5, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_10x5_KHR	0x93b8     GL_COMPRESSED_RGBA_ASTC_10x5_KHR	Astc10x5Unorm 
		0x93b9 => wgpu::TextureFormat::Astc { block: AstcBlock::B10x6, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_10x6_KHR	0x93b9     GL_COMPRESSED_RGBA_ASTC_10x6_KHR	Astc10x6Unorm 
		0x93ba => wgpu::TextureFormat::Astc { block: AstcBlock::B10x8, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_10x8_KHR	0x93ba GL_COMPRESSED_RGBA_ASTC_10x8_KHR	Astc10x8Unorm  
		0x93bb => wgpu::TextureFormat::Astc { block: AstcBlock::B10x10, channel: AstcChannel::Unorm },//  GL_COMPRESSED_RGBA_ASTC_10x10_KHR	0x93bb     GL_COMPRESSED_RGBA_ASTC_10x10_KHR	Astc10x10Unorm 
		0x93bc => wgpu::TextureFormat::Astc { block: AstcBlock::B12x10, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_12x10_KHR	0x93bc     GL_COMPRESSED_RGBA_ASTC_12x10_KHR	Astc12x10 
		0x93bd => wgpu::TextureFormat::Astc { block: AstcBlock::B12x12, channel: AstcChannel::Unorm },// GL_COMPRESSED_RGBA_ASTC_12x12_KHR	0x93bd     GL_COMPRESSED_RGBA_ASTC_12x12_KHR	Astc12x12Unorm
		_ => panic!("not suport fomat： {}", v),
	}
}

pub fn depth_or_array_layers(layer_count: u32, face_count: u32, depth: u32) -> u32 {
    if layer_count > 1 || face_count > 1 {
        layer_count * face_count
    } else {
        depth
    }
    .max(1)
}

pub fn dimension(height: u32, depth: u32) -> TextureDimension {
    if depth > 1 {
        TextureDimension::D3
    } else if height > 1 {
        TextureDimension::D2
    } else {
        TextureDimension::D1
    }
}

pub fn view_dimension(layer_count: u32, face_count: u32, depth: u32) -> TextureViewDimension {
    if face_count == 6 {
        if layer_count > 1 {
            TextureViewDimension::CubeArray
        } else {
            TextureViewDimension::Cube
        }
    } else if layer_count > 1 {
        TextureViewDimension::D2Array
    } else if depth > 1 {
        TextureViewDimension::D3
    } else {
        TextureViewDimension::D2
    }
}


#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ImageTextureDesc {
    /// 路径
    pub url: Atom,
    /// 是否 sRGB
    pub srgb: bool,
    pub useage: wgpu::TextureUsages,
}

impl ImageTextureDesc {
    pub fn new(url: Atom) -> Self {
        Self {
            url,
            srgb: <wgpu::TextureFormat as PiDefaultTextureFormat>::is_srgb(),
            useage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        }
    }
}

pub(crate) const KTX_SUFF: &'static str = ".ktx";