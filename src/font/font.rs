/// 字体接口定义

use std::{
	hash::Hash, 
	collections::hash_map::Entry, 
};

use derive_deref::{Deref, DerefMut};
use ordered_float::NotNan;
use pi_hash::XHashMap;
use pi_slotmap::{DefaultKey, SlotMap};
use serde::{Serialize, Deserialize};

use pi_atom::Atom;
use smallvec::SmallVec;

use super::{tables::FontTable, sdf_table::FontCfg};

#[derive(Debug, Clone)]
pub struct Size<T> {
	pub width: T,
	pub height: T,
}

#[derive(Debug)]
pub struct Block {
	pub y: f32, 
	pub x: f32, 
	pub width: f32, 
	pub height: f32,
}

pub struct FontImage {
	pub buffer: Vec<u8>,
	pub width: usize,
	pub height: usize,
}

impl FontImage {
	pub fn new(width: usize, height: usize) -> Self {
		let len = width * height * 4;
		let mut buffer = Vec::with_capacity(len);
		unsafe { buffer.set_len(len) }

		Self {
			buffer,
			width,
			height,
		}
	}
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Font {
	pub font_family: SmallVec<[Atom; 1]>,
	pub font_family_string: Atom,
	pub font_size: usize,
	pub font_weight: usize,
	pub stroke: NotNan<f32>,
	pub font_type: FontType,
}

impl Font {
	pub fn new(font_family_string: Atom, font_size: usize, font_weight: usize, stroke: NotNan<f32>) -> Self {
		let font_family = font_family_string.split(",");
		let font_family = font_family.map(|r| {
			Atom::from(r.trim())
		}).collect::<SmallVec<[Atom; 1]>>();
		Self {
			font_family_string,
			font_family,
			
			font_size,
			font_weight,
			stroke,
			font_type: FontType::Bitmap,
		}
	}
}

// #[derive(Debug, Clone, Hash, PartialEq, Eq)]
// pub struct Font {
// 	pub(crate) font: Font,
// 	pub(crate) font_id: SmallVec<[FontId; 1]>,
// }

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut)]
pub struct GlyphId(pub DefaultKey);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut, Default)]
pub struct FontId(pub DefaultKey);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut, Default)]
pub struct FontFamilyId(pub DefaultKey);


#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut)]
pub struct FontFaceId(pub DefaultKey);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontType {
	/// 位图方案
	Bitmap, 
	/// Sdf1 （配置表配置sdf信息的方案）
	Sdf1,
	/// Sdf2（圆弧模拟贝塞尔曲线计算距离的方案）
	Sdf2,
}

impl Default for FontType {
    fn default() -> Self {
        Self::Bitmap
    }
}

pub struct FontMgr {
	pub sheet: GlyphSheet, // 字形表，用于存放字体的字形信息
	pub table: FontTable,
	// 画笔，用于测量、绘制文字，不同平台可能有不同的实现（web平台依赖于canvas的文字功能， app、exe平台通常可以使用freetype； 同时还有sdf字体功能）
	pub font_type: FontType,
}

impl std::ops::Deref for FontMgr {
    type Target = GlyphSheet;

    fn deref(&self) -> &Self::Target {
        &self.sheet
    }
}

impl std::ops::DerefMut for FontMgr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sheet
    }
}

/// 字形表
pub struct GlyphSheet {
	font_names: SlotMap<DefaultKey, Atom>,
	font_names_map: XHashMap<Atom, DefaultKey>,

	fonts_map: XHashMap<Font, FontId>,
	pub fonts: SlotMap<DefaultKey, FontInfo>,

	pub font_familys: SlotMap<DefaultKey, SmallVec<[Atom;1]>>,
	font_family_map: XHashMap<SmallVec<[Atom;1]>, DefaultKey>,

	default_sdf_char: Vec<(Atom, char)>,
	// 默认字体
	default_font: Option<Atom>, 

}

impl GlyphSheet {
	pub fn fonts(&self) -> &SlotMap<DefaultKey, FontInfo> {
		&self.fonts
	}

	// pub fn glyphs(&self) -> &SlotMap<DefaultKey, GlyphIdDesc> {
	// 	&self.glyphs
	// }
}

impl FontMgr {
	pub fn new(width: usize, height: usize, font_type: FontType) -> FontMgr {
		Self { 
			sheet: GlyphSheet {
				font_names: SlotMap::default(),
				font_names_map: XHashMap::default(),

				fonts_map: XHashMap::default(),
				fonts: SlotMap::default(),

				font_familys: SlotMap::default(),
				font_family_map:  XHashMap::default(),
				// glyph_id_map: XHashMap::default(), 
				// glyphs: SlotMap::default(), 
				// base_glyph_id_map: XHashMap::default(),
				// base_glyphs: SlotMap::default(),
				// text_packer: TextPacker::new(width as usize, height as usize),
				// size: Size {width, height},
				default_sdf_char: Vec::default(),
				default_font: None,
			},
			table: FontTable::new(width, height),
			font_type,
		}
	}

	pub fn size(&self) -> Size<usize> {
		self.table.size(self.font_type)
	}

	pub fn set_font_type(&mut self, font_type: FontType) {
		self.font_type = font_type
	}

	pub fn font_type(&self) -> FontType {
		self.font_type
	}
}

impl FontMgr {
	/// 字体id， 为每一种不同的字体描述创建一个唯一分配的id
	pub fn font_id(&mut self, mut f: Font) -> FontId {
		f.font_type = self.font_type;
		let mut font_family = f.font_family.clone();

		// 插入默认字体
		if let Some(d) = &self.default_font {
			let mut has_default = false;
			for f in font_family.iter() {
				if f == d {
					has_default = true;
					break;
				}
			}
			if !has_default {
				font_family.push(d.clone());
			}
		}

		let font_face_ids = font_family.iter().map(|r| {
			self.create_font_face(r)
		}).collect::<SmallVec<[FontFaceId; 1]>>();
		let font_family_id = self.font_family_id(&font_family);
		
		let font_id = self.get_or_insert_font(f.clone(), font_face_ids.clone(), font_family_id);

		let font_info = &mut self.sheet.fonts[font_id.0];

		self.table.check_or_create_face(font_info, self.font_type);

		let (height, max_height) = self.table.height(font_id, font_info, self.font_type);
		font_info.height = height;
		font_info.max_height = max_height;

		font_id
	}

	pub fn font_family_id(&mut self, fonts: &SmallVec<[Atom; 1]>) -> FontFamilyId {
		let GlyphSheet{ font_family_map, font_familys,..} = &mut self.sheet;

		match font_family_map.entry(fonts.clone()) {
			Entry::Occupied(r) => FontFamilyId(*r.get()),
			Entry::Vacant(r) => {
				let id =  font_familys.insert(fonts.clone());
				r.insert(id);
				FontFamilyId(id)
			},
		}
	}

	pub fn font_info(&self, f: FontId) -> &FontInfo {
		&self.sheet.fonts[f.0]
	}

	pub fn font_height(&self, f: FontId, font_size: usize) -> f32 {
		match self.sheet.fonts.get(*f) {
			Some(r) =>  {if r.height < 2.0 {r.height * font_size as f32} else {r.height}},
			None => font_size as f32, // 异常情况，默认返回font_size
		}
	}

	/// 字形id, 纹理中没有更多空间容纳时，返回None
	pub fn glyph_id(&mut self, f: FontId, char: char) -> Option<GlyphId> {
		let font_info = &mut self.sheet.fonts[f.0];
		self.table.glyph_id(f, char, font_info, self.font_type)
	}

	/// 测量宽度
	pub fn measure_width(&mut self, f: FontId, char: char) -> f32 {
		let font_info = match self.sheet.fonts.get_mut(*f) {
			Some(r) => r,
			None => return 0.0,
		};
		self.table.measure_width(f, font_info, char, self.font_type)
	}

	// /// 取到字形信息
	// pub fn glyph(&self, id: GlyphId) -> &Glyph {
	// 	if self.sheet.glyphs.get(*id).is_none() {
	// 		panic!("glyph is not exist, {:?}", id);
	// 	}
	// 	&self.sheet.glyphs[*id].glyph
	// }

	// /// 绘制文字
	// pub fn draw<F: FnMut(Block, FontImage) + Clone + ThreadSync + 'static>(&mut self, update: F) {
	// 	// let (fonts, glyphs) = (&mut self.fonts, &self.glyphs);
	// 	let width = self.size.width;
	// 	let (sheet, brush) = (&mut self.sheet, &mut self.brush);
	// 	let (glyphs, fonts) = (&sheet.glyphs, &mut sheet.fonts);

	// 	let mut sdf_all_draw = Vec::new();
	// 	let mut native_all_draw = Vec::new();
	// 	// let mut sdf_all_draw_slotmap = Vec::new();
	// 	// let mut f = Vec::new();
	// 	// 遍历所有支持的字体，如果其对应的绘制等待列表不为空，则进行绘制
	// 	for (k, font_info) in fonts.iter_mut() {
	// 		let await_info = &font_info.await_info;
	// 		if await_info.wait_list.len() == 0 {
	// 			continue;
	// 		}

	// 		let offset = *font_info.font.stroke/2.0;

	// 		// let g_0 = &glyphs[*await_info.wait_list[0]];
	// 		let mut start_pos;
	// 		let (mut y, mut height, mut font_face_index);

	// 		let (mut start, mut end) = (0, 0.0);
	// 		let mut x_c;

	// 		while start < await_info.wait_list.len() {
	// 			let g = &glyphs[*await_info.wait_list[start]];
	// 			start_pos = (g.glyph.x, g.glyph.y);
	// 			y = g.glyph.y as f32;
	// 			font_face_index = g.font_face_index;
	// 			height = g.glyph.height;

	// 			x_c = Vec::new();

	// 			// 每一批次绘制，只绘制同一行的字符
	// 			for i in start..await_info.wait_list.len() {
	// 				let g = &glyphs[
	// 					*await_info.wait_list[i]
	// 				];

	// 				// y不相同的字符（不在同一行）, 或者fontface不同，在下一批次绘制，因此结束本批次字符的收集
	// 				if g.glyph.y as f32 != y || g.font_face_index != font_face_index {
	// 					break;
	// 				} else if g.glyph.x as f32 - end > 1.0 && x_c.len() > 0 {
	// 					// y相同， 但x间有空位，也在下批次处理
	// 					break;
	// 				}
	// 				// 否则y相同，则加入当前批次
	// 				x_c.push(Await {
	// 					x_pos: g.glyph.x as f32 - start_pos.0 as f32 + offset, // 如果有描边，需要偏移一定位置，否则可能无法容纳描边
	// 					char: g.char,
	// 					width: g.glyph.width as u32,
	// 					height: g.glyph.height as u32,
	// 				});
	// 				end = g.glyph.x as f32 + g.glyph.width;
	// 			}
	// 			start += x_c.len();

	// 			let mut end = end + 1.0;
	// 			if end as usize > width {
	// 				end = width as f32;
	// 			}
				
	// 			let block = DrawBlock {
	// 				chars: x_c,
	// 				font_id: FontId(k),
	// 				font_size: font_info.font.font_size,
	// 				font_stroke:  font_info.font.stroke,
	// 				block: Block {
	// 					x: start_pos.0 as f32,
	// 					y: start_pos.1 as f32,
	// 					width: end - start_pos.0 as f32,
	// 					height,
	// 				},
	// 				font_face_index,
	// 				font_family: font_info.font.font_family.clone(),
	// 			};
	// 			if self.font_type == FontType::Sdf1 {
	// 				sdf_all_draw.push(block);
	// 			} else {
	// 				native_all_draw.push(block);
	// 			}
				
	// 		}

	// 		font_info.await_info.wait_list.clear();
	// 		font_info.await_info.size = Size {width: 0, height: 0};// 似乎没有作用？
	// 	}

	// 	if sdf_all_draw.len() > 0 {
	// 		// 绘制一个批次的字符
	// 		brush.sdf_brush.draw(sdf_all_draw, update.clone());
	// 	}
	// 	if native_all_draw.len() > 0 {
	// 		// 绘制一个批次的字符
	// 		brush.native_brush.draw(native_all_draw, update);
	// 	}
	// }

	// 添加sdf配置
	pub fn add_sdf_cfg(&mut self, font_cfg: FontCfg) {
		let font_face = Atom::from(font_cfg.name.clone());
		let font_face_id = self.create_font_face(&font_face);
		self.table.sdf_table.add_cfg(font_face_id, font_cfg);
	}

	// 添加sdf配置
	pub fn add_sdf_default_char(&mut self, _font_face: Atom, _char: char) {
		// let font_face_id = self.create_font_face(&font_face);
		// let font_family_id = self.font_family_id(Font::new(font_face.clone(), BASE_FONT_SIZE, 500, unsafe{ NotNan::new_unchecked(0.0)}));
		// let glyph_id = self.glyph_id(font_family_id, char).unwrap();
		// self.brush.sdf_brush.add_default_char(font_face_id, glyph_id, font_face.clone(), char);
		// self.default_sdf_char.push((font_face, char));
	}

	pub fn fonts_mut(&mut self) -> &mut SlotMap<DefaultKey, FontInfo> {
		&mut self.sheet.fonts
	}

	/// 清理字形信息
	pub fn clear(&mut self) {
		for  info in self.sheet.fonts.values_mut() {
			info.await_info.size = Size {width: 0, height: 0};
			info.await_info.wait_list.clear();
		}
		// self.sheet.glyph_id_map.clear();
		// self.sheet.glyphs.clear();
		// self.sheet.text_packer.clear();
		self.table.clear();

		// 添加默认字符
		for (font_face, c) in self.default_sdf_char.clone().into_iter() {
			self.add_sdf_default_char(font_face, c);
		}
	}

	// /// 取到纹理
	// fn texture_view(&self) -> &Handle<RenderRes<TextureView>> {
	// 	&self.sheet.texture_view
	// }

	// /// 取到纹理版本
	// fn texture_version(&self) -> usize {
	// 	self.sheet.texture_version.load(Ordering::Relaxed)
	// }

	fn get_or_insert_font(&mut self, f: Font, font_ids: SmallVec<[FontFaceId; 1]>, font_family_id: FontFamilyId) -> FontId {
		match self.sheet.fonts_map.entry(f.clone()) {
			Entry::Occupied(r) => return r.get().clone(),
			Entry::Vacant(r) => {
				let id = self.sheet.fonts.insert(FontInfo {
					font: f,
					height: 0.0,
					max_height: 0.0,
					await_info: AwaitInfo { 
						size: Size {width: 0, height: 0}, 
						wait_list: Vec::new() },
					font_family_id,
					font_ids,
				});
				r.insert(FontId(id)).clone()
			}
		}
	}
	
	

	pub fn create_font_face(&mut self, r: &Atom) -> FontFaceId {
		let GlyphSheet{font_names_map, font_names, default_font, ..} = &mut self.sheet;
		if default_font.is_none() {
			// 默认字体初始化为第一次创建的font_face（也可被外部更改）
			*default_font = Some(r.clone());
		}

		if r.as_str() == "" {
			if let Some(first) = font_names.iter().next() {
				return FontFaceId(first.0); // 第一个添加的字体为默认字体
			}
		}
		match font_names_map.entry(r.clone()) {
			Entry::Occupied(v) => FontFaceId(v.get().clone()),
			Entry::Vacant(v) => {
				let key = font_names.insert(r.clone());
				v.insert(key.clone());
				FontFaceId(key)
			},
		}
	}
}

pub const BASE_FONT_SIZE: usize = 32;

/// sdf没字符的ox，oy为浮点数， 但ox，oy通常比较小， 
/// 在配置中将ox，oy编码为i16，以节省配置空间，减少配置文件下载量，因此将真实的ox，oy乘以本常量再取整，
/// 因此真实的ox = glyph.ox * OFFSET_RANGE, oy = glyph.oy * OFFSET_RANGE
pub const OFFSET_RANGE: f32 = (2 as u32).pow(15) as f32;

// 粗体字的font-weight
pub const BLOD_WEIGHT: usize = 700;

// 粗体字的放大因子
pub const BLOD_FACTOR: f32 = 1.13;

pub struct GlyphIdDesc {
	pub font_id: FontId,
	pub char: char,
	pub glyph: Glyph,
	pub font_face_index: usize,
}

#[derive(Debug)]
pub struct FontInfo {
	pub font: Font,
	pub font_ids: SmallVec<[FontFaceId; 1]>,
	pub height: f32,
	pub max_height: f32,
	pub await_info: AwaitInfo,
	pub font_family_id: FontFamilyId,
}

#[derive(Debug)]
pub struct AwaitInfo {
	pub size: Size<usize>,
	pub wait_list: Vec<GlyphId>,
	// pub top: usize,
}


#[derive(Debug, Default, Clone)]
pub struct Glyph {
	pub x: f32,
    pub y: f32,
	pub ox: f32,
	pub oy: f32,
	pub width: f32,
    pub height: f32,
	pub advance: f32,
}

#[derive(Debug)]
pub struct Await {
	pub x_pos: f32,
	pub char: char,
	pub width: u32,
	pub height: u32,
}

#[derive(Debug)]
pub struct DrawBlock {
	pub chars: Vec<Await>, 
	pub font_id: FontId, 
	pub font_family: SmallVec<[Atom; 1]>,
	pub font_size: usize,
	pub font_stroke: NotNan<f32>,
	pub block: Block,
	pub font_face_index: usize,
}


// msdf 需要修正字形信息
pub fn fix_box(is_sdf: bool, width: f32, weight: usize, sw: f32) -> (f32/*左偏移*/, f32/*宽度*/) {
	if is_sdf {
		let mut w = width - sw;
		if weight >= BLOD_WEIGHT {
			w = w / BLOD_FACTOR;
		}
		((width - w)/2.0,  w)
	} else {
		(0.0, width)
	}
	
}