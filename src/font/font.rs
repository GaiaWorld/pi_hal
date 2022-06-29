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

use super::{text_pack::TextPacker};
use crate::font_brush::Brush;

#[derive(Debug, Clone)]
pub struct Size<T> {
	pub width: T,
	pub height: T,
}

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
	pub font_family: Atom,
	pub font_size: usize,
	pub font_weight: usize,
	pub stroke: NotNan<f32>,
}

impl Font {
	pub fn new(font_family: Atom, font_size: usize, font_weight: usize, stroke: NotNan<f32>) -> Self {
		Self {
			font_family,
			font_size,
			font_weight,
			stroke,
		}
	}
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut)]
pub struct GlyphId(pub DefaultKey);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut)]
pub struct FontId(DefaultKey);

pub struct FontMgr {
	sheet: GlyphSheet,
	brush: Brush,
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

pub struct GlyphSheet {
	fonts_map: XHashMap<Font, FontId>,
	fonts: SlotMap<DefaultKey, FontInfo>,
	glyph_id_map: XHashMap<(FontId, char), GlyphId>,
	glyphs: SlotMap<DefaultKey, GlyphIdDesc>,

	text_packer: TextPacker,
	size: Size<usize>,
}

impl GlyphSheet {
	pub fn fonts(&self) -> &SlotMap<DefaultKey, FontInfo> {
		&self.fonts
	}

	pub fn glyphs(&self) -> &SlotMap<DefaultKey, GlyphIdDesc> {
		&self.glyphs
	}
	pub fn size(&self) -> Size<usize> {
		self.size.clone()
	}
}

impl FontMgr {
	pub fn new(width: usize, height: usize) -> FontMgr {
		Self { 
			sheet: GlyphSheet {
				fonts_map: XHashMap::default(), 
				fonts: SlotMap::default(), 
				glyph_id_map: XHashMap::default(), 
				glyphs: SlotMap::default(), 
				text_packer: TextPacker::new(width as usize, height as usize),
				size: Size {width, height}
			},
			brush: Brush::new(),
		}
	}
}

impl FontMgr {
	/// 字体id
	pub fn font_id(&mut self, f: Font) -> FontId {
		match self.sheet.fonts_map.entry(f.clone()) {
			Entry::Occupied(r) => r.get().clone(),
			Entry::Vacant(r) => {
				let id = self.sheet.fonts.insert(FontInfo {
					font: f,
					height: 0.0,
					await_info: AwaitInfo { 
						size: Size {width: 0, height: 0}, 
						wait_list: Vec::new() },
				});
				let id = r.insert(FontId(id)).clone();
				let height = self.brush.height(id, &self.sheet.fonts[*id].font);
				self.sheet.fonts[*id].height = height;
				id
			}
		}
	}

	pub fn font_height(&self, f: FontId, font_size: usize) -> f32 {
		match self.sheet.fonts.get(*f) {
			Some(r) => r.height * (font_size as f32 / BASE_FONT_SIZE as f32),
			None => font_size as f32, // 异常情况，默认返回font_size
		}
	}

	/// 字形id, 纹理中没有更多空间容纳时，返回None
	pub fn glyph_id(&mut self, f: FontId, char: char) -> Option<GlyphId> {
		match self.sheet.glyph_id_map.entry((f, char)) {
			Entry::Occupied(r) => Some(r.get().clone()),
			Entry::Vacant(r) => {
				let font = &mut self.sheet.fonts[*f];

				let width = self.brush.width(f, &font.font, char);
				let size = Size {
					width: width, 
					height: font.height};

				// 在纹理中分配一个位置
				let tex_position = self.sheet.text_packer.alloc(
					size.width.ceil() as usize, 
					size.height.ceil() as usize);
				let tex_position = match tex_position {
					Some(r) => r,
					None => return None,
				};

				// 分配GlyphId
				let id = GlyphId(self.sheet.glyphs.insert(GlyphIdDesc{
					font_id: f,
					char,
					glyph: Glyph {
						x: tex_position.x, 
						y: tex_position.y, 
						width: size.width, 
						height: size.height},
				}));

				// 放入等待队列, 并统计等待队列的总宽度
				// font.await_info.size.width += size.width.ceil() as usize;
				// font.await_info.size.height += size.height.ceil() as usize;
				font.await_info.wait_list.push(id);
				
				Some(r.insert(id).clone())
			}
		}
	}

	/// 测量宽度
	pub fn measure_width(&mut self, f: FontId, char: char) -> f32 {
		match self.sheet.glyph_id_map.entry((f, char)) {
			Entry::Occupied(r) => {
				let id = r.get();
				self.sheet.glyphs[**id].glyph.width
			},
			Entry::Vacant(_r) => {
				let font = &mut self.sheet.fonts[*f];
				self.brush.width(f, &font.font, char)
			}
		}
	}

	/// 取到字形信息
	pub fn glyph(&self, id: GlyphId) -> &Glyph {
		&self.sheet.glyphs[*id].glyph
	}

	/// 绘制文字
	pub fn draw<F: FnMut(Block, FontImage)>(&mut self, update: F) {
		self.brush.draw(&self.sheet, update);
		// 清理等待列表
		for (_k, font_info) in self.fonts.iter_mut() {
			font_info.await_info.wait_list.clear();
			font_info.await_info.size = Size {width: 0, height: 0};// 似乎没有作用？
		}
	}

	/// 清理字形信息
	pub fn clear(&mut self) {
		self.sheet.fonts.clear();
		self.sheet.fonts_map.clear();
		self.sheet.glyph_id_map.clear();
		self.sheet.glyphs.clear();
		self.sheet.text_packer.clear();
	}

	// /// 取到纹理
	// fn texture_view(&self) -> &Handle<RenderRes<TextureView>> {
	// 	&self.sheet.texture_view
	// }

	// /// 取到纹理版本
	// fn texture_version(&self) -> usize {
	// 	self.sheet.texture_version.load(Ordering::Relaxed)
	// }
}

pub const BASE_FONT_SIZE: usize = 32;

pub struct GlyphIdDesc {
	pub font_id: FontId,
	pub char: char,
	pub glyph: Glyph,
}

#[derive(Debug)]
pub struct FontInfo {
	pub font: Font,
	pub height: f32,
	pub await_info: AwaitInfo,
}

#[derive(Debug)]
pub struct AwaitInfo {
	pub size: Size<usize>,
	pub wait_list: Vec<GlyphId>,
	// pub top: usize,
}


#[derive(Debug, Default, Clone)]
pub struct Glyph {
	pub x: usize,
    pub y: usize,
	pub width: f32,
    pub height: f32,
}