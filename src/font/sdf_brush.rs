use std::{sync::{Arc, Mutex, OnceLock}, cell::OnceCell};

use pi_async_rt::prelude::AsyncValue;
use pi_atom::Atom;
use pi_hash::XHashMap;
use pi_share::{ThreadSync, ShareMutex, Share};
use pi_slotmap::{SecondaryMap, DefaultKey, SlotMap};
use serde::{Serialize, Deserialize};

use super::font::{FontFamilyId, Block, FontImage, DrawBlock, FontInfo, FontId, GlyphId};

use crate::runtime::MULTI_MEDIA_RUNTIME;
use pi_async_rt::prelude::AsyncRuntime;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontCfg {
	pub name: String,
	pub metrics: MetricsInfo,
	pub glyphs: XHashMap<char, GlyphInfo>, // msdf才会有，字符纹理宽度
}

#[derive(Default, Debug)]
pub struct SdfBrush {
	fonts_glyph: SecondaryMap<DefaultKey, FontCfg>, // DefaultKey为FontId
	pub(crate) default_char: Option<(MetricsInfo, GlyphInfo, Atom, char, FontId, GlyphId)>,
}

impl SdfBrush {

	// 添加sdf配置
	pub fn add_cfg(&mut self, font_id: FontId, font_cfg: FontCfg) {
		self.fonts_glyph.insert(font_id.0, font_cfg);
	}

	// 添加默认字符
	// 早调用此方法之前，保证改字体的配置已经就绪
	pub fn add_default_char(&mut self, font_id: FontId, glyph_id: GlyphId, name: Atom, char: char) {
		if let Some(r) = self.fonts_glyph.get(font_id.0) {
			if let Some(glyph_info) = r.glyphs.get(&char) {
				self.default_char = Some((r.metrics.clone(), glyph_info.clone(), name, char, font_id, glyph_id));
			}
		}

		log::info!("add default char fail, char or font is not exist, char={:?}, font={:?}", char, font_id);
	}

	pub fn height(&mut self, _font_id: FontFamilyId, font: &FontInfo) -> (f32, f32/*max_height*/) {
		let mut ret = (0.0, 0.0);
		for font_id in font.font_ids.iter() {
			if let Some(r) = self.fonts_glyph.get(font_id.0) {
				if ret.0 != 0.0 {
					ret.0 = (r.metrics.ascender - r.metrics.descender) * (font.font.font_size as f32 / r.metrics.font_size);
				}
				
				ret.1 = r.metrics.max_height.max(ret.1);
			};
		}
		if ret.0 != 0.0 {
			return ret;
		}

		match &self.default_char {
			Some(r) => {
				return ((r.0.ascender - r.0.descender) * (font.font.font_size as f32 / r.0.font_size), r.0.max_height)
			},
			None => return (font.font.font_size as f32, font.font.font_size as f32),
		};
	}

	pub fn width(&mut self, font_id: FontFamilyId, font: &FontInfo, char: char) -> (f32, usize) {
		let (info, metrics, index) = match self.info(font_id, font, char) {
			Some(r) => r,
			None => match self.info(font_id, font, '□') {
				Some(r) => r,
				None => match &self.default_char {
					Some(r) => (&r.1, &r.0, font.font.font_family.len()),
					None => return (font.font.font_size as f32 / 2.0, 0),
				},
			},
		};

		(info.advance as f32 * (font.font.font_size as f32 / metrics.font_size), index)
	}

	
	pub fn glyph_info(&mut self, font_id: FontFamilyId, font: &FontInfo, char: char) -> Option<(&GlyphInfo, usize)> {
		let info = self.info(font_id, font, char);
		info.as_ref().map(|r| (r.0, r.2))
	}

	pub fn draw<F: FnMut(Block, FontImage) + Clone + ThreadSync + 'static>(
		&mut self, 
		mut draw_list: Vec<DrawBlock>,
		mut update: F) {

		// 修改为异步，TODO
		// for draw_block in draw_list.into_iter() {
		// 	let mut update = update.clone();
		// 	let font_family = if draw_block.font_face_index == draw_block.font_family.len() {
		// 		self.default_char.as_ref().unwrap().2.clone()
		// 	} else {
		// 		draw_block.font_family[draw_block.font_face_index].clone()
		// 	};
		let mut chars_familys: Vec<(Atom, Vec<char>)> = Vec::with_capacity(draw_list.len());
		for draw_block in draw_list.iter() {
			let font_family = if draw_block.font_face_index == draw_block.font_family.len() {
				self.default_char.as_ref().unwrap().2.clone()
			} else {
				draw_block.font_family[draw_block.font_face_index].clone()
			};

			// 记录字体的长度
			let chars = match chars_familys.last_mut() {
				Some(r) if r.0 == font_family => &mut r.1,
				_ => {
					chars_familys.push((font_family, Vec::with_capacity(draw_block.chars.len())));
					let last_index = chars_familys.len() - 1;
					&mut chars_familys[last_index].1
				},
			};

			// 添加待加载的字符
			for await_char in draw_block.chars.iter() {
				chars.push(await_char.char);
			}
		}

		let async_value = AsyncValue::new();
		let mut r = Vec::with_capacity(chars_familys.len());
		for _i in 0..chars_familys.len() {
			r.push(None);
		}
		

		let result = Share::new(ShareMutex::new((0, r)));
		let len = chars_familys.len();
		for (index, chars) in chars_familys.into_iter().enumerate() {
			let async_value1 = async_value.clone();
			let result1 = result.clone();
			MULTI_MEDIA_RUNTIME.spawn(async move {
				let v = create_async_value(&chars.0, &chars.1);
				let buffers: Vec<Vec<u8>> = v.await;
				let mut lock = result1.lock();
				log::debug!("load========={:?}, {:?}", lock.0, len);
				lock.1[index] = Some(buffers);
				lock.0 += 1;
				if lock.0 == len {
					async_value1.set(true);
				}
			}).unwrap();
		}
		MULTI_MEDIA_RUNTIME.spawn(async move {
			log::debug!("load1=========");
			async_value.await;
			let mut lock = result.lock();
			let r = &mut lock.1;
			log::debug!("load2========={:?}", r.len());
	
			while let Some(item) = r.pop() {
				let mut buffers = match item {
					Some(r) => r,
					None => unreachable!(),
				};
				
				while buffers.len() > 0 {
					let draw_block = draw_list.pop().unwrap();
					if buffers.len() < draw_block.chars.len() {
						unreachable!()
					}
					let pack: usize = 1;
					let ww = draw_block.block.width as usize;
					let hh = draw_block.block.height as usize;
					let mut buffer = Vec::with_capacity(ww * hh * pack);
					unsafe{ buffer.set_len(ww * hh * pack) };

					let mut index = draw_block.chars.len();
					while index > 0  {
						index -= 1;
						let bin = buffers.pop().unwrap();
						let await_char = &draw_block.chars[index];
						// let glyph = cfg.index.get(chars[i]);
						// let info_index = i * 3;

						let o = await_char.x_pos as usize;
						let width = await_char.width as usize;
						let height = await_char.height as usize;

						let min_width = width.min(ww);
						let min_height = height.min(hh);
						for i in 0..min_height { // i代表行数
							for j in 0..min_width { // 遍历一行中的每一列（j表列数）
								// 拷贝每个像素
								let y_line = i;
								let src_offset = (j + i * width) * pack;
								let dst_offset = (o + j + ww * y_line) * pack;
								// console.log(src_offset, dst_offset);
								buffer[dst_offset] = bin[src_offset];
								// buffer[dst_offset + 1] = bin[src_offset + 1];
								// buffer[dst_offset + 2] = bin[src_offset + 2];
								// buffer[dst_offset + 3] = bin[src_offset + 3];
								// console.log(i, j, dst_offset, src_offset);
							}
						}
					}
					let img = FontImage {
						buffer,
						width: draw_block.block.width as usize,
						height: draw_block.block.height as usize,
					};
					
					update(draw_block.block, img);
				}
			}
		}).unwrap();

		// let mut count = AtomicUsize::new(chars_familys.len());
		

		// MULTI_MEDIA_RUNTIME.spawn(async move {
		// 	let result = ShareMutex::new(Vec::new());
		// 	let mut update = update.clone();
			
		// 	// 修改为异步，TODO
		// 	for draw_block in draw_list.into_iter() {
		// 		let mut chars = Vec::with_capacity(draw_block.chars.len());
		// 		for await_char in draw_block.chars.iter() {
		// 			chars.push(await_char.char);
		// 		}
				
		// 		let v = create_async_value(&font_family, &chars);
		// 		let buffers: Vec<Vec<u8>> = v.await;

		// 		if buffers.len() != chars.len() {
		// 			log::error!("load sdf char error, {:?}", &chars);
		// 			return;
		// 		}

		// 		// let pack = 4;
		// 		let pack: usize = 1;
		// 		let ww = draw_block.block.width as usize;
		// 		let hh = draw_block.block.height as usize;
		// 		let mut buffer = Vec::with_capacity(ww * hh * pack);
		// 		unsafe{ buffer.set_len(ww * hh * pack) };
		// 		//
		// 		for index in 0..chars.len(){
		// 			let bin = &buffers[index];
		// 			let await_char = &draw_block.chars[index];
		// 			// let glyph = cfg.index.get(chars[i]);
		// 			// let info_index = i * 3;

		// 			let o = await_char.x_pos as usize;
		// 			let width = await_char.width as usize;
		// 			let height = await_char.height as usize;

		// 			let min_width = width.min(ww);
		// 			let min_height = height.min(hh);
		// 			// console.log("xxx===============", x);
		// 			for i in 0..min_height { // i代表行数
		// 				for j in 0..min_width { // 遍历一行中的每一列（j表列数）
		// 					// 拷贝每个像素
		// 					let y_line = i;
		// 					let src_offset = (j + i * width) * pack;
		// 					let dst_offset = (o + j + ww * y_line) * pack;
		// 					// console.log(src_offset, dst_offset);
		// 					buffer[dst_offset] = bin[src_offset];
		// 					// buffer[dst_offset + 1] = bin[src_offset + 1];
		// 					// buffer[dst_offset + 2] = bin[src_offset + 2];
		// 					// buffer[dst_offset + 3] = bin[src_offset + 3];
		// 					// console.log(i, j, dst_offset, src_offset);
		// 				}
		// 			}
		// 		}
		// 		let img = FontImage {
		// 			buffer,
		// 			width: draw_block.block.width as usize,
		// 			height: draw_block.block.height as usize,
		// 		};
				
		// 		update(draw_block.block, img);
		// 	}
		// }).unwrap();
		

			
			// let faces = match self.faces.get_mut(*draw_block.font_id) {
			// 	Some(r) => r,
			// 	None => return ,
			// };
			// let face = faces[draw_block.font_face_index].as_mut().unwrap();
			// // 绘制
			// // face.set_pixel_sizes(draw_block.font_size as u32);
			// // face.set_stroker_width(*draw_block.font_stroke as f64);

			// let (block, image) = draw_sync(
			// 	draw_block.chars, 
			// 	draw_block.block,
			// 	face,
			// 	*draw_block.font_stroke as f64
			// );

		// }
	}

	pub fn metrics_info(&self, font_id: &FontId) -> &MetricsInfo {
		if let Some(r) = self.fonts_glyph.get(font_id.0) {
			return  &r.metrics;
		};
		if let Some(r) = &self.default_char {
			return &r.0;
		}
		panic!("");
	}

	fn info(&mut self, _font_id: FontFamilyId, font: &FontInfo, char: char) -> Option<(&GlyphInfo, &MetricsInfo, usize)> {
		for (index, font_id) in font.font_ids.iter().enumerate() {
			
			if let Some(r) = self.fonts_glyph.get(font_id.0) {
				if let Some(glyph_info) = r.glyphs.get(&char) {
					return  Some((glyph_info, &r.metrics, index));
				}
			};
		}
		None

		// for (index, font_id) in font.font_ids.iter().enumerate() {
		// 	if let Some(r) = self.fonts_glyph.get(font_id.0) {
		// 		if let Some(glyph_info) = r.glyphs.get(&'□') {
		// 			return  (glyph_info, &r.metrics, index);
		// 		}
		// 	};
		// }

		// match &self.default_char {
		// 	Some(r) => return (&r.1, &r.0, font.font_ids.len()),
		// 	None => panic!("字符不存在，font_family={:?}, char={:?}, and default_char is none", font.font.font_family, char),
		// };
	}
}

/// Sdf文字自身的字形信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlyphInfo {
    pub ox: i16, //文字可见区域左上角相对于文字外边框的左上角在水平轴上的距离 百分比(实际百分比应该除以256，之所以这样，是为了压缩数据尺寸)
    pub oy: i16, //文字可见区域左上角相对于文字外边框的左上角在垂直轴上的距离 百分比(实际百分比应该除以256，之所以这样，是为了压缩数据尺寸)
    pub width: u8,
    pub height: u8, 
    pub advance: u8, // advancePx
}

/// 字体的全局信息，对该字体的所有文字生效
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MetricsInfo {
	pub font_size: f32,             // 文字尺寸
	pub line_height: f32,           // 默认行高
	pub max_height: f32,	        // 所有字形，最大高度（用于在纹理中分配行高）
	pub ascender: f32,              // 升线 （单位： font_size的百分比）
	pub descender: f32,             // 降线 （单位： font_size的百分比）
	pub underline_y: f32,           // 下划线的位置 （暂未使用）
	pub underline_thickness: f32,   // 
	pub distance_range: f32,        // msdf才会用到（0~1范围内的sdf所跨过的像素数量）
}

#[derive(Debug)]
pub struct Await {
	pub x_pos: f32,
	pub char: char,
	pub font_face_index: usize, 
}
pub struct OnceCellWrap(pub OnceCell<ShareCb>);
unsafe impl Sync for OnceCellWrap {}

pub struct OnceLockWrap(pub OnceLock<ShareCb>);

#[cfg(not(target_arch="wasm32"))]
static LOAD_CB_SDF: OnceLockWrap = OnceLockWrap(OnceLock::new());

#[cfg(target_arch="wasm32")]
static LOAD_CB_SDF: OnceCellWrap = OnceCellWrap(OnceCell::new());
// pub static SDF_LOADER: OnceCell<Box<dyn FnMut()>> = OnceCellWrap(OnceCell::new());
lazy_static! {
	
	pub static ref LOAD_MAP_SDF: Mutex<SlotMap<DefaultKey, AsyncValue<Vec<Vec<u8>>>>> =
		Mutex::new(SlotMap::new());
}

#[cfg(target_arch="wasm32")]
pub trait Cb: Fn(DefaultKey, usize, &[char]) {}
#[cfg(target_arch="wasm32")]
impl<T: Fn(DefaultKey, usize, &[char])> Cb for T {}
#[cfg(target_arch="wasm32")]
pub type ShareCb = std::rc::Rc<dyn Cb>;

#[cfg(not(target_arch="wasm32"))]
pub trait Cb: Fn(DefaultKey, usize, &[char])  + Send + Sync {}
#[cfg(not(target_arch="wasm32"))]
impl<T: Fn(DefaultKey, usize, &[char]) + Send + Sync > Cb for T {}
#[cfg(not(target_arch="wasm32"))]
pub type ShareCb = Arc<dyn Cb>;




pub fn init_load_cb(cb: ShareCb) {
    match LOAD_CB_SDF.0.set(cb) {
		Ok(r) => r,
		Err(_e) => panic!("LOAD_CB_SDF.set")
	};
}

pub fn on_load(key: DefaultKey, data: Vec<Vec<u8>>) {
    let v = LOAD_MAP_SDF.lock().unwrap().remove(key).unwrap();
	v.set(data);
}

pub fn create_async_value(font: &Atom, chars: &[char]) -> AsyncValue<Vec<Vec<u8>>> {
    let mut lock = LOAD_MAP_SDF.lock().unwrap();
	let r = AsyncValue::new();
	let k = lock.insert(r.clone());
	if let Some(cb) = LOAD_CB_SDF.0.get() {
		cb(k, font.get_hash(), chars);
	} else {
	}
	r
}


// #[derive(Debug, Serialize, Deserialize)]
// pub struct FontCfg {
//     pub name: String,
//     pub metrics: MetricsInfo,
//     pub glyphs: XHashMap<char, GlyphInfo>,
// }

// // 字符的sdf值
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct CharSdf {
// 	pub unicode: u32,        // 字符的unicode编码
//     pub buffer: Vec<u8>,  // 字符的sdf buffer
// }