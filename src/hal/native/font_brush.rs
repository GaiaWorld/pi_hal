
use std::hash::{DefaultHasher, Hash, Hasher};

use pi_atom::Atom;
use pi_hash::XHashMap;
use pi_share::ThreadSync;
use pi_slotmap::{SecondaryMap, DefaultKey};
use font_kit::{font::Face, util::{ WritePixel, Rgba}};
use smallvec::SmallVec;

use crate::{create_async_value, font::font::{Await, Block, DrawBlock, FontImage, FontInfo}};

/// 字体渲染笔刷
///
/// 负责管理字体资源并进行文本渲染，主要功能包括：
/// - 字体缓存管理
/// - 字体度量计算
/// - 异步文本渲染
///
/// ## 字段说明
/// - `faces`: 字体实例缓存（按字体ID索引）
/// - `base_faces`: 基础字体实例集合
/// - `base_faces_map`: 字体名称到基础字体的映射
/// - `default_family`: 默认字体族名称
pub struct Brush {
	faces: SecondaryMap<DefaultKey, (SmallVec<[Option<usize>; 1]>, usize, f32) >,
	base_faces: Vec<(Face, usize, f32)>,
	base_faces_map: XHashMap<Atom, usize>,
	default_family: Atom,
}

impl Brush {
	pub fn new() -> Self {
		Brush {
			faces: SecondaryMap::default(),
			base_faces: Vec::new(),
			base_faces_map: XHashMap::default(),
			default_family: Atom::from("default"),
		}
	}

	pub fn check_or_create_face(& mut self, font: &FontInfo) {
		if self.faces.get_mut(*font.font_family_id).is_some() {
			return;
		}
		let mut faces = SmallVec::new();
		// Face::from_family_name("default", font.font_size as u32).unwrap();
		for font_family in font.font.font_family.iter().chain([self.default_family.clone()].iter()) {
			// let time = pi_time::Instant::now();
			match self.base_faces_map.entry(font_family.clone()) {
				std::collections::hash_map::Entry::Occupied(r) => {
					faces.push(Some(*r.get()))
				},
				std::collections::hash_map::Entry::Vacant(_) => {
					let index = match Face::from_family_name(font_family, 32) {
						Ok(face) => {
							self.base_faces.push((face, 32, 0.0));
							let index = self.base_faces.len() - 1;
							self.base_faces_map.insert(font_family.clone(), index);
							Some(index)
						},
						Err(_) => None
					};
					faces.push(index);
				},
			}
			// log::warn!("font face======font_id={:?},{:?}, {:?}, {:?}", font_id, font_family, font, pi_time::Instant::now() - time);
		}
		self.faces.insert(*font.font_family_id, (faces, 32, 0.0));
		// log::trace!("check_or_create_face!!!========{:?}, {:p}, {:?}", *font_id, &self.faces[*font_id], &self.faces[*font_id]);
	}

	/// 计算基础字体高度
	///
	/// # 参数
	/// - `font`: 字体配置信息
	///
	/// # 返回值
	/// 返回字体的总高度（上行高 - 下行高）
	///
	/// # Panics
	/// 当字体不存在时触发panic
	pub fn base_height(&mut self, font: &FontInfo) -> f32 {
		let faces = &mut self.faces[*font.font_family_id];
		// log::trace!("height!!!========{:?}, {:p}, {:?}", *font_id, face, face);
		// face.set_pixel_sizes(font.font_size as u32);
		for face in faces.0.iter() {
			if let Some(face) = face {
				let face = &mut self.base_faces[*face];
				if faces.1 != face.1 {
					face.1 = faces.1;
					face.0.set_pixel_sizes(faces.1 as u32);
				}
				let metrics = face.0.get_global_metrics();
				return metrics.ascender as f32 - metrics.descender as f32
			}
		}
		panic!("font is not exist, font_family={:?}, and default font is none", &font.font.font_family);
		// let metrics = faces.get_global_metrics();
		// metrics.ascender as f32 - metrics.descender as f32
	}

    /// 计算字符宽度
    ///
    /// # 参数
    /// - `font`: 字体配置信息
    /// - `char`: 需要测量的字符
    ///
    /// # 返回值
    /// 元组包含：
    /// - f32: 字符的水平推进宽度
    /// - usize: 使用的字体实例在数组中的索引
    ///
    /// # Panics
    /// 当字体不存在时触发panic
    pub fn base_width(&mut self, font: &FontInfo, char: char) -> (f32, usize) {
		let faces = &mut self.faces[*font.font_family_id];
	
		for (index,face) in faces.0.iter().enumerate() {
			if let Some(face) = face {
				let face = &mut self.base_faces[*face];
				if faces.1 != face.1 {
					face.1 = faces.1;
					face.0.set_pixel_sizes(faces.1 as u32);
				}

				if let Ok(metrics) = face.0.get_metrics(char) {
					return (metrics.hori_advance as f32, index)
				}
			}
		}

		panic!("font is not exist, font_family={:?}, and default font is none", &font.font.font_family);
    }

    /// 执行文本绘制操作
    ///
    /// # 参数
    /// - `draw_list`: 需要绘制的文本块列表
    /// - `update`: 绘制完成后的回调函数
    ///
    /// # 注意
    /// 当前实现为同步绘制，未来计划改为异步操作
    pub fn draw<F: FnMut(Block, FontImage) + Clone + ThreadSync + 'static>(
		&mut self, 
		draw_list: Vec<DrawBlock>,
		mut update: F) {
			todo!();
		// // 修改为异步，TODO
		// for draw_block in draw_list.into_iter() {
		// 	let faces = match self.faces.get_mut(*draw_block.font_id) {
		// 		Some(r) => r,
		// 		None => return ,
		// 	};
		// 	let face = faces.0[draw_block.font_face_index].as_mut().unwrap();
		// 	// 绘制
		// 	// face.set_pixel_sizes(draw_block.font_size as u32);
		// 	// face.set_stroker_width(*draw_block.font_stroke as f64);

		// 	let face = &mut self.base_faces[*face];
		// 	if face.1 != draw_block.font_size {
		// 		face.1 = draw_block.font_size;
		// 		face.0.set_pixel_sizes(draw_block.font_size as u32);
		// 	}

		// 	if face.2 != *draw_block.font_stroke {
		// 		face.2 = *draw_block.font_stroke;
		// 		face.0.set_stroker_width(*draw_block.font_stroke as f64);
		// 	}

		// 	let (block, image) = draw_sync(
		// 		draw_block.chars, 
		// 		draw_block.block,
		// 		&mut face.0,
		// 		*draw_block.font_stroke as f64
		// 	);

		// 	update(block, image);
		// }
	}
}

// 同步绘制（异步： TODO）
fn draw_sync(list: Vec<Await>, block: Block, face: &mut Face, stroke: f64) -> (Block, FontImage) {
	let mut image = FontImage::new(block.width as usize, block.height as usize);
	image.init_background();
	
	for await_item in list.iter() {
		face.fill_char(
			await_item.char, 
			await_item.x_pos as i32, 
			0, 
			Rgba { r: 0, g: 255, b: 0, a: 255}, 
			None, 
			0, 
			0, 
			0, 
			&mut image).unwrap();
		if stroke > 0.0 {
			face.stroker_char(
				await_item.char, 
				await_item.x_pos as i32, 
				0, 
				Rgba { r: 255, g: 0, b: 0, a: 255}, 
				None, 
				0, 
				0, 
				0, 
				&mut image).unwrap();
		}
	}
	(block, image)
}

impl FontImage {
	fn init_background(&mut self) {
		let mut i = 0;
		let len = self.buffer.len();
		while i < len{
			self.buffer[i] = 255;
			self.buffer[i + 1] = 0;
			self.buffer[i + 2] = 255;
			self.buffer[i + 3] = 255;
			i += 4;
		}
	}
}

impl WritePixel for FontImage {
    fn put_font_pixel(&mut self, x: i32, y: i32, src: Rgba) {
		if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
			return;
		}
		// 与[255, 0, 255, 255]颜色混合
		let src_a = src.a as f32 /255.0;
		let dst_a = 1.0 - src_a;
		let offset = 4 * (self.width * y as usize + x as usize);
		if offset + 4 < self.buffer.len() {
			// 一次性内存写入，TODO bgra
			self.buffer[offset] =  (src.r as f32 * src_a + self.buffer[offset] as f32 * dst_a) as u8 ;
			self.buffer[offset + 1] = (src.g as f32 * src_a + self.buffer[offset + 1] as f32 * dst_a) as u8;
			self.buffer[offset + 2] = (src.b as f32 * src_a + self.buffer[offset + 2] as f32 * dst_a) as u8;

			// let b =  (src.b as f32 * src_a + self.buffer[offset] as f32 * dst_a) as u8 ;
			// let g = (src.g as f32 * src_a + self.buffer[offset + 1] as f32 * dst_a) as u8;
			// let r = (src.r as f32 * src_a + self.buffer[offset + 2] as f32 * dst_a) as u8;
			// if( self.buffer[offset + 1] + self.buffer[offset + 2] )<250 {
			// 	log::warn!("{}, {}, {}", self.buffer[offset], self.buffer[offset + 1], self.buffer[offset + 2]);
			// }
		}
    }

	// TODO
    fn put_shadow_pixel(&mut self, _x: i32, _y: i32, _src: Rgba) {
    }
}

pub async fn load_font_sdf() -> Vec<(String, Vec<SdfInfo>)>{
	let mut hasher = DefaultHasher::new();
    "load_font_sdf".hash(&mut hasher);
    let v = create_async_value("file", "load_font_sdf", hasher.finish(), vec![]);
	let buffer = v.await.unwrap();
	bitcode::deserialize(&buffer[..]).unwrap()
}

pub use pi_sdf::font::FontFace;
pub use pi_sdf::glyphy::blob::{TexInfo, SdfInfo};
// pub use pi_sdf::utils::GlyphInfo;
pub use pi_sdf::utils::{CellInfo, SdfInfo2, LayoutInfo, OutlineInfo};
