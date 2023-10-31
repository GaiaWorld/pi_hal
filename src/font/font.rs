/// 字体接口定义

use std::{
	hash::Hash, 
	collections::hash_map::Entry, 
};

use derive_deref::{Deref, DerefMut};
use ordered_float::NotNan;
use pi_hash::XHashMap;
use pi_share::ThreadSync;
use pi_slotmap::{DefaultKey, SlotMap};
use serde::{Serialize, Deserialize};
use pi_null::Null;

use pi_atom::Atom;

use super::text_pack::TextPacker;
use crate::font_brush::Brush;

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
	sheet: GlyphSheet, // 字形表，用于存放字体的字形信息
	
	brush: Brush,// 画笔，用于测量、绘制文字，不同平台可能有不同的实现
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
	fonts_map: XHashMap<Font, FontId>,
	fonts: SlotMap<DefaultKey, FontInfo>,
	glyph_id_map: XHashMap<(FontId, char), GlyphId>,
	glyphs: SlotMap<DefaultKey, GlyphIdDesc>,
	
	base_glyph_id_map: XHashMap<(FontId, char), GlyphId>,
	base_glyphs: SlotMap<DefaultKey, BaseCharDesc>,

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
				base_glyph_id_map: XHashMap::default(),
				base_glyphs: SlotMap::default(),
				text_packer: TextPacker::new(width as usize, height as usize),
				size: Size {width, height}
			},
			brush: Brush::new(),
		}
	}
}

impl FontMgr {
	/// 字体id， 为每一种不同的字体描述创建一个唯一分配的id
	pub fn font_id(&mut self, f: Font) -> FontId {
		let id = self.get_or_insert_font(f.clone());

		// 每个字体字体描述，都对应一个基础字体
		// 基础字体的font_size为32px，font_weight为500， stroke宽度为0，其余属性与当前传入的字体保持一样
		// 基础字体的左右是，存储文字在32号字体下的字形信息（测量字形速度并不快，该设计为了尽量减少文字的测量次数，不同字号的文字均通过基础文字通过缩放算得，只有基础尺寸的文字，才会真正的去测量）
		let base_font = Font {
			font_size: 32,
			stroke: unsafe { NotNan::new_unchecked(0.0) },
			font_weight: 500,
			font_family: f.font_family,
		};
		let base_font_id = self.get_or_insert_font(base_font.clone());
		let base_font = &mut self.sheet.fonts[*base_font_id];
		// 基础字体的高度为0.0，证明是新创建的基础字体（之前不存在），则立即获取字体的高度（字体高度是同字体，不同字符共享的，所以可根据字体直接测量得到）
		if base_font.height == 0.0 {
			self.brush.check_or_create_face(base_font_id, &base_font.font);
			let height = self.brush.height(base_font_id);
			base_font.base_font_id = base_font_id;
			base_font.height = height;
		}

		let base_h = base_font.height;
		let font = &mut self.sheet.fonts[*id];
		if base_font_id != id { 
			// 当前传入字体与基础字体不同， 则通过比例缩放，计算传入字体的高度。
			self.brush.check_or_create_face(id, &font.font);
			font.height = (base_h * (f.font_size as f32 /BASE_FONT_SIZE as f32)).ceil();
			font.base_font_id = base_font_id;
		}

		id
	}

	pub fn font_height(&self, f: FontId, font_size: usize) -> f32 {
		match self.sheet.fonts.get(*f) {
			Some(r) =>  r.height,
			None => font_size as f32, // 异常情况，默认返回font_size
		}
	}

	/// 字形id, 纹理中没有更多空间容纳时，返回None
	pub fn glyph_id(&mut self, f: FontId, char: char) -> Option<GlyphId> {
		let (id, base_font_id) = match self.sheet.glyph_id_map.entry((f, char)) {
			Entry::Occupied(r) => {
				let id = r.get().clone();
				return Some(id);
			},
			Entry::Vacant(r) => {
				let font = &mut self.sheet.fonts[*f];
				// 分配GlyphId
				let id = GlyphId(self.sheet.glyphs.insert(GlyphIdDesc{
					font_id: f,
					char,
					glyph: Glyph {
						x: 0, 
						y: 0, 
						width: 0.0, 
						height: 0.0},
				}));

				// 放入等待队列, 并统计等待队列的总宽度
				// font.await_info.size.width += size.width.ceil() as usize;
				// font.await_info.size.height += size.height.ceil() as usize;
				font.await_info.wait_list.push(id);
				(r.insert(id).clone(), font.base_font_id)
			}
		};
		let base_w = self.measure_base(base_font_id, char);

		let font = &mut self.sheet.fonts[*f];
		let size = Size {
			width: base_w * (font.font.font_size as f32 /BASE_FONT_SIZE as f32) + *font.font.stroke + 2.0, 
			height: font.height,
		};

		// 在纹理中分配一个位置
		let tex_position = self.sheet.text_packer.alloc(
			size.width.ceil() as usize, 
			size.height.ceil() as usize);
		let tex_position = match tex_position {
			Some(r) => r,
			None => return None,
		};
		let g = &mut self.sheet.glyphs[*id];
		g.glyph.width = size.width.round();
		g.glyph.height = size.height;
		g.glyph.x = tex_position.x;
		g.glyph.y = tex_position.y;
		Some(id)
	}

	/// 测量宽度
	pub fn measure_width(&mut self, f: FontId, char: char) -> f32 {
		if let Some(id) = self.sheet.glyph_id_map.get(&(f, char)) {
			return self.glyphs[**id].glyph.width
		}

		let (base_font_id, font_size, stroke) = match self.sheet.fonts.get(*f) {
			Some(r) => (r.base_font_id, r.font.font_size, *r.font.stroke),
			None => return 0.0,
		};
		let base_w = self.measure_base(base_font_id, char);
		let ratio = font_size as f32 /BASE_FONT_SIZE as f32;

		ratio * base_w + stroke
	}

	/// 取到字形信息
	pub fn glyph(&self, id: GlyphId) -> &Glyph {
		&self.sheet.glyphs[*id].glyph
	}

	/// 绘制文字
	pub fn draw<F: FnMut(Block, FontImage) + Clone + ThreadSync + 'static>(&mut self, update: F) {
		// let (fonts, glyphs) = (&mut self.fonts, &self.glyphs);
		let width = self.size.width;
		let (sheet, brush) = (&mut self.sheet, &mut self.brush);
		let (glyphs, fonts) = (&sheet.glyphs, &mut sheet.fonts);

		let mut all_draw = Vec::new();
		// 遍历所有支持的字体，如果其对应的绘制等待列表不为空，则进行绘制
		for (k, font_info) in fonts.iter_mut() {
			let await_info = &font_info.await_info;
			if await_info.wait_list.len() == 0 {
				continue;
			}

			let offset = *font_info.font.stroke/2.0;

			// let g_0 = &glyphs[*await_info.wait_list[0]];
			let mut start_pos;
			let (mut y, mut height);

			let (mut start, mut end) = (0, 0.0);
			let mut x_c;
			while start < await_info.wait_list.len() {
				let g = &glyphs[*await_info.wait_list[start]];
				start_pos = (g.glyph.x, g.glyph.y);
				y = g.glyph.y as f32;
				height = g.glyph.height;
				x_c = Vec::new();

				// 每一批次绘制，只绘制同一行的字符
				for i in start..await_info.wait_list.len() {
					let g = &glyphs[
						*await_info.wait_list[i]
					];

					// y不相同的字符（不在同一行），在下一批次绘制，因此结束本批次字符的收集
					if g.glyph.y as f32 != y {
						break;
					} else if g.glyph.x as f32 - end > 1.0 && x_c.len() > 0 {
						// y相同， 但x间有空位，也在下批次处理
						break;
					}
					// 否则y相同，则加入当前批次
					x_c.push(Await {
						x_pos: g.glyph.x as f32 - start_pos.0 as f32 + offset, // 如果有描边，需要偏移一定位置，否则可能无法容纳描边
						char: g.char,
					});
					end = g.glyph.x as f32 + g.glyph.width;
				}
				start += x_c.len();

				let mut end = end + 1.0;
				if end as usize > width {
					end = width as f32;
				}
				all_draw.push(DrawBlock {
					chars: x_c,
					font_id: FontId(k),
					font_size: font_info.font.font_size,
					font_stroke:  font_info.font.stroke,
					block: Block {
						x: start_pos.0 as f32,
						y: start_pos.1 as f32,
						width: end - start_pos.0 as f32,
						height,
					},
				});
			}

			font_info.await_info.wait_list.clear();
			font_info.await_info.size = Size {width: 0, height: 0};// 似乎没有作用？
		}

		if all_draw.len() > 0 {
			// 绘制一个批次的字符
			brush.draw(all_draw, update);
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

	fn get_or_insert_font(&mut self, f: Font) -> FontId {
		match self.sheet.fonts_map.entry(f.clone()) {
			Entry::Occupied(r) => return r.get().clone(),
			Entry::Vacant(r) => {
				let id = self.sheet.fonts.insert(FontInfo {
					font: f,
					height: 0.0,
					await_info: AwaitInfo { 
						size: Size {width: 0, height: 0}, 
						wait_list: Vec::new() },
					base_font_id: FontId(DefaultKey::null()),
				});
				r.insert(FontId(id)).clone()
			}
		}
	}
	
	fn measure_base(&mut self, base_font_id: FontId, char: char) -> f32 {
		match self.sheet.base_glyph_id_map.entry((base_font_id, char)) {
			Entry::Occupied(r) => {
				let g = &self.sheet.base_glyphs[**r.get()];
				g.width
			},
			Entry::Vacant(r) => {
				let width = self.brush.width(base_font_id, char);

				// 分配GlyphId
				let id = GlyphId(self.sheet.base_glyphs.insert(BaseCharDesc{
					font_id: base_font_id,
					char,
					width,
				}));
				r.insert(id);
				width
			}
		}
	}
}

pub const BASE_FONT_SIZE: usize = 32;

pub struct GlyphIdDesc {
	pub font_id: FontId,
	pub char: char,
	pub glyph: Glyph,
}

pub struct BaseCharDesc {
	pub font_id: FontId,
	pub char: char,
	pub width: f32,
}

#[derive(Debug)]
pub struct FontInfo {
	pub font: Font,
	pub height: f32,
	pub await_info: AwaitInfo,
	pub base_font_id: FontId,
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

#[derive(Debug)]
pub struct Await {
	pub x_pos: f32,
	pub char: char,
}

#[derive(Debug)]
pub struct DrawBlock {
	pub chars: Vec<Await>, 
	pub font_id: FontId, 
	pub font_size: usize,
	pub font_stroke: NotNan<f32>,
	pub block: Block,
}


