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
use smallvec::SmallVec;

use super::{text_pack::TextPacker, brush::FontBrush, sdf_brush::FontCfg};

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
	pub is_sdf: bool,
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
			is_sdf: false,
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
pub struct FontFamilyId(pub(crate) DefaultKey);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut)]
pub struct FontId(pub(crate) DefaultKey);

pub struct FontMgr {
	sheet: GlyphSheet, // 字形表，用于存放字体的字形信息
	// 画笔，用于测量、绘制文字，不同平台可能有不同的实现（web平台依赖于canvas的文字功能， app、exe平台通常可以使用freetype； 同时还有sdf字体功能）
	pub brush: FontBrush,
	use_sdf: bool,
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

	fonts_map: XHashMap<Font, FontFamilyId>,
	fonts: SlotMap<DefaultKey, FontInfo>,
	glyph_id_map: XHashMap<(FontFamilyId, char), GlyphId>,
	glyphs: SlotMap<DefaultKey, GlyphIdDesc>,
	
	base_glyph_id_map: XHashMap<(FontFamilyId, char), BaseCharDesc>,
	// base_glyphs: SlotMap<DefaultKey, BaseCharDesc>,

	text_packer: TextPacker,
	size: Size<usize>,

	default_sdf_char: Vec<(Atom, char)>,

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
				font_names: SlotMap::default(),
				font_names_map: XHashMap::default(),

				fonts_map: XHashMap::default(),
				fonts: SlotMap::default(), 
				glyph_id_map: XHashMap::default(), 
				glyphs: SlotMap::default(), 
				base_glyph_id_map: XHashMap::default(),
				// base_glyphs: SlotMap::default(),
				text_packer: TextPacker::new(width as usize, height as usize),
				size: Size {width, height},
				default_sdf_char: Vec::default(),
			},
			brush: FontBrush::new(),
			use_sdf: false,
		}
	}

	pub fn set_use_sdf(&mut self, use_sdf: bool) {
		self.use_sdf = use_sdf
	}

	pub fn use_sdf(&self) -> bool {
		self.use_sdf
	}
}

impl FontMgr {
	/// 字体id， 为每一种不同的字体描述创建一个唯一分配的id
	pub fn font_family_id(&mut self, mut f: Font) -> FontFamilyId {
		f.is_sdf = self.use_sdf;

		let font_id = f.font_family.iter().map(|r| {
			self.create_font_face(r)
		}).collect::<SmallVec<[FontId; 1]>>();
		
		let id = self.get_or_insert_font(f.clone(), font_id.clone());

		// 每个字体字体描述，都对应一个基础字体
		// 基础字体的font_size为32px，font_weight为500， stroke宽度为0，其余属性与当前传入的字体保持一样
		// 基础字体的左右是，存储文字在32号字体下的字形信息（测量字形速度并不快，该设计为了尽量减少文字的测量次数，不同字号的文字均通过基础文字通过缩放算得，只有基础尺寸的文字，才会真正的去测量）
		let base_font = Font {
			font_size: 32,
			stroke: unsafe { NotNan::new_unchecked(0.0) },
			font_weight: 500,
			font_family: f.font_family,
			font_family_string: f.font_family_string,
			is_sdf: self.use_sdf,
		};
		let base_font_id = self.get_or_insert_font(base_font.clone(), font_id.clone());
		let base_font = &mut self.sheet.fonts[*base_font_id];
		// 基础字体的高度为0.0，证明是新创建的基础字体（之前不存在），则立即获取字体的高度（字体高度是同字体，不同字符共享的，所以可根据字体直接测量得到）
		if base_font.height == 0.0 {
			self.brush.check_or_create_face(base_font_id, &base_font, self.use_sdf);
			let height = self.brush.height(base_font_id, &base_font, self.use_sdf);
			log::debug!("base height = {:?}", height);
			base_font.base_font_id = base_font_id;
			base_font.height = height.0;
			base_font.max_height = height.1;
		}

		let base_h = base_font.height;
		let base_max_h = base_font.max_height;
		let font = &mut self.sheet.fonts[*id];
		if base_font_id != id { 
			// 当前传入字体与基础字体不同， 则通过比例缩放，计算传入字体的高度。
			self.brush.check_or_create_face(id, &font, self.use_sdf);
			font.height = base_h * (f.font_size as f32 / BASE_FONT_SIZE as f32);
			font.max_height = base_max_h * (f.font_size as f32 / BASE_FONT_SIZE as f32);
			log::debug!("font height = {:?}", font);
			font.base_font_id = base_font_id;
		}

		id
	}
	pub fn font_info(&self, f: FontFamilyId) -> &FontInfo {
		&self.sheet.fonts[f.0]
	}

	pub fn font_height(&self, f: FontFamilyId, font_size: usize) -> f32 {
		match self.sheet.fonts.get(*f) {
			Some(r) =>  r.height,
			None => font_size as f32, // 异常情况，默认返回font_size
		}
	}

	/// 字形id, 纹理中没有更多空间容纳时，返回None
	pub fn glyph_id(&mut self, f: FontFamilyId, char: char) -> Option<GlyphId> {
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
					font_face_index: 0,
					glyph: Glyph {
						x: 0.0, 
						y: 0.0, 
						ox: 0.0,
						oy: 0.0,
						width: 0.0, 
						height: 0.0,
						advance: 0.0,},
				}));

				// 放入等待队列, 并统计等待队列的总宽度
				// font.await_info.size.width += size.width.ceil() as usize;
				// font.await_info.size.height += size.height.ceil() as usize;

				(r.insert(id).clone(), font.base_font_id)
			}
		};

		let font = &mut self.sheet.fonts[*f];
		// let ff = font.font.font_family_string.clone();
		let mut max_height = font.max_height;
		let char_texture_size = if self.use_sdf {
			let (glyph_info, index) = match self.brush.sdf_brush.glyph_info(base_font_id, font, char) {
				Some(r) => {
					font.await_info.wait_list.push(id);
					r
				},
				None => {
					if char == '□' {
						if let Some(r) = &self.brush.sdf_brush.default_char {
							return Some(r.5); // 显示默认字符
						} 
						return None;
					} else {
						match self.glyph_id(f, '□') {
							Some(id) => {
								self.sheet.glyph_id_map.insert((f, char), id);
								return Some(id);
							},
							None => {
								self.sheet.glyph_id_map.remove(&(f, char));
								return None;
							},
						};
					}
				},
			};

			let glyph = &mut self.sheet.glyphs[id.0];
			glyph.font_face_index = index;
			glyph.glyph.ox = glyph_info.ox as f32 / OFFSET_RANGE;
			glyph.glyph.oy = glyph_info.oy as f32 / OFFSET_RANGE;
			glyph.glyph.advance = glyph_info.advance as f32;
			max_height = glyph_info.height as f32;
			// sdf的文字纹理， 不需要加上描边宽度， 也不需要间隔, 直接从配置中取到
			Size {
				width: glyph_info.width as f32,
				height: glyph_info.height as f32 ,
			}
		} else {
			font.await_info.wait_list.push(id);

			let (base_w, index) = self.measure_base(base_font_id, char);
			let glyph = &mut self.sheet.glyphs[id.0];
			glyph.font_face_index = index;

			let font = &mut self.sheet.fonts[*f];
			Size {
				width: base_w * (font.font.font_size as f32 / BASE_FONT_SIZE as f32) + *font.font.stroke + 2.0, 
				height: font.height,
			}
		};
		// 在纹理中分配一个位置
		let tex_position = self.sheet.text_packer.alloc(
			char_texture_size.width.ceil() as usize, 
			max_height.ceil() as usize);
		let tex_position = match tex_position {
			Some(r) => r,
			None => return None,
		};
		// log::warn!("char_texture_size====={:?}, {:?}, {:?}, {:?}", &char, &char_texture_size, tex_position, ff);
		let g = &mut self.sheet.glyphs[*id];
		g.glyph.width = char_texture_size.width.round();
		g.glyph.height = char_texture_size.height;
		g.glyph.x = tex_position.x as f32;
		g.glyph.y = tex_position.y as f32;
		Some(id)
	}

	/// 测量宽度
	pub fn measure_width(&mut self, f: FontFamilyId, char: char) -> f32 {
		if let Some(id) = self.sheet.glyph_id_map.get(&(f, char)) {
			return self.glyphs[**id].glyph.width
		}

		let (base_font_id, font_size, stroke) = match self.sheet.fonts.get(*f) {
			Some(r) => (r.base_font_id, r.font.font_size, *r.font.stroke),
			None => return 0.0,
		};
		let base_w = self.measure_base(base_font_id, char).0;
		let ratio = font_size as f32 / BASE_FONT_SIZE as f32;

		ratio * base_w + stroke
	}

	/// 取到字形信息
	pub fn glyph(&self, id: GlyphId) -> &Glyph {
		if self.sheet.glyphs.get(*id).is_none() {
			panic!("glyph is not exist, {:?}", id);
		}
		&self.sheet.glyphs[*id].glyph
	}

	/// 绘制文字
	pub fn draw<F: FnMut(Block, FontImage) + Clone + ThreadSync + 'static>(&mut self, update: F) {
		// let (fonts, glyphs) = (&mut self.fonts, &self.glyphs);
		let width = self.size.width;
		let (sheet, brush) = (&mut self.sheet, &mut self.brush);
		let (glyphs, fonts) = (&sheet.glyphs, &mut sheet.fonts);

		let mut sdf_all_draw = Vec::new();
		let mut native_all_draw = Vec::new();
		// let mut sdf_all_draw_slotmap = Vec::new();
		// let mut f = Vec::new();
		// 遍历所有支持的字体，如果其对应的绘制等待列表不为空，则进行绘制
		for (k, font_info) in fonts.iter_mut() {
			let await_info = &font_info.await_info;
			if await_info.wait_list.len() == 0 {
				continue;
			}

			let offset = *font_info.font.stroke/2.0;

			// let g_0 = &glyphs[*await_info.wait_list[0]];
			let mut start_pos;
			let (mut y, mut height, mut font_face_index);

			let (mut start, mut end) = (0, 0.0);
			let mut x_c;

			while start < await_info.wait_list.len() {
				let g = &glyphs[*await_info.wait_list[start]];
				start_pos = (g.glyph.x, g.glyph.y);
				y = g.glyph.y as f32;
				font_face_index = g.font_face_index;
				height = g.glyph.height;

				x_c = Vec::new();

				// 每一批次绘制，只绘制同一行的字符
				for i in start..await_info.wait_list.len() {
					let g = &glyphs[
						*await_info.wait_list[i]
					];

					// y不相同的字符（不在同一行）, 或者fontface不同，在下一批次绘制，因此结束本批次字符的收集
					if g.glyph.y as f32 != y || g.font_face_index != font_face_index {
						break;
					} else if g.glyph.x as f32 - end > 1.0 && x_c.len() > 0 {
						// y相同， 但x间有空位，也在下批次处理
						break;
					}
					// 否则y相同，则加入当前批次
					x_c.push(Await {
						x_pos: g.glyph.x as f32 - start_pos.0 as f32 + offset, // 如果有描边，需要偏移一定位置，否则可能无法容纳描边
						char: g.char,
						width: g.glyph.width as u32,
						height: g.glyph.height as u32,
					});
					end = g.glyph.x as f32 + g.glyph.width;
				}
				start += x_c.len();

				let mut end = end + 1.0;
				if end as usize > width {
					end = width as f32;
				}
				
				let block = DrawBlock {
					chars: x_c,
					font_id: FontFamilyId(k),
					font_size: font_info.font.font_size,
					font_stroke:  font_info.font.stroke,
					block: Block {
						x: start_pos.0 as f32,
						y: start_pos.1 as f32,
						width: end - start_pos.0 as f32,
						height,
					},
					font_face_index,
					font_family: font_info.font.font_family.clone(),
				};
				if self.use_sdf {
					sdf_all_draw.push(block);
				} else {
					native_all_draw.push(block);
				}
				
			}

			font_info.await_info.wait_list.clear();
			font_info.await_info.size = Size {width: 0, height: 0};// 似乎没有作用？
		}

		if sdf_all_draw.len() > 0 {
			// 绘制一个批次的字符
			brush.sdf_brush.draw(sdf_all_draw, update.clone());
		}
		if native_all_draw.len() > 0 {
			// 绘制一个批次的字符
			brush.native_brush.draw(native_all_draw, update);
		}
	}

	// 添加sdf配置
	pub fn add_sdf_cfg(&mut self, font_cfg: FontCfg) {
		let font_face = Atom::from(font_cfg.name.clone());
		let font_face_id = self.create_font_face(&font_face);
		self.brush.sdf_brush.add_cfg(font_face_id, font_cfg);
	}

	// 添加sdf配置
	pub fn add_sdf_default_char(&mut self, font_face: Atom, char: char) {
		let font_face_id = self.create_font_face(&font_face);
		let font_family_id = self.font_family_id(Font::new(font_face.clone(), BASE_FONT_SIZE, 500, unsafe{ NotNan::new_unchecked(0.0)}));
		let glyph_id = self.glyph_id(font_family_id, char).unwrap();
		self.brush.sdf_brush.add_default_char(font_face_id, glyph_id, font_face.clone(), char);
		self.default_sdf_char.push((font_face, char));
	}

	/// 清理字形信息
	pub fn clear(&mut self) {
		for  info in self.sheet.fonts.values_mut() {
			info.await_info.size = Size {width: 0, height: 0};
			info.await_info.wait_list.clear();
		}
		// self.sheet.fonts_map.clear();
		self.sheet.glyph_id_map.clear();
		self.sheet.glyphs.clear();
		self.sheet.text_packer.clear();

		// 添加默认字符
		for (font_face, c) in self.default_sdf_char.clone().into_iter() {
			self.add_sdf_default_char(font_face, c);
		}


		// glyph_id_map: XHashMap<(FontFamilyId, char), GlyphId>,
		// glyphs: SlotMap<DefaultKey, GlyphIdDesc>,
		
		// base_glyph_id_map: XHashMap<(FontFamilyId, char), GlyphId>,
		// base_glyphs: SlotMap<DefaultKey, BaseCharDesc>,

		// text_packer: TextPacker,
	}

	// /// 取到纹理
	// fn texture_view(&self) -> &Handle<RenderRes<TextureView>> {
	// 	&self.sheet.texture_view
	// }

	// /// 取到纹理版本
	// fn texture_version(&self) -> usize {
	// 	self.sheet.texture_version.load(Ordering::Relaxed)
	// }

	fn get_or_insert_font(&mut self, f: Font, font_ids: SmallVec<[FontId; 1]>) -> FontFamilyId {
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
					base_font_id: FontFamilyId(DefaultKey::null()),
					font_ids,
				});
				r.insert(FontFamilyId(id)).clone()
			}
		}
	}
	
	fn measure_base(&mut self, base_font_id: FontFamilyId, char: char) -> (f32, usize) {
		let font = &self.sheet.fonts[*base_font_id];
		match self.sheet.base_glyph_id_map.entry((base_font_id, char)) {
			Entry::Occupied(r) => {
				let g = r.get();
				(g.width, g.font_face_index)
			},
			Entry::Vacant(r) => {
				let (mut width, index) = self.brush.width(base_font_id, font, char, self.use_sdf);

				r.insert(BaseCharDesc{
					font_id: base_font_id,
					char,
					width,
					font_face_index: index,
				});

				// 如果是ascii字符， 其粗体文字的宽度会适当加宽（浏览器实验所得结果）
				let is_blod = char.is_ascii() && font.font.font_weight >= BLOD_WEIGHT;
				if is_blod {
					width = width * BLOD_FACTOR;
				}

				(width, index)
			}
		}
	}

	fn create_font_face(&mut self, r: &Atom) -> FontId {
		let GlyphSheet{font_names_map, font_names, ..} = &mut self.sheet;
		match font_names_map.entry(r.clone()) {
			Entry::Occupied(v) => FontId(v.get().clone()),
			Entry::Vacant(v) => {
				let key = font_names.insert(r.clone());
				v.insert(key.clone());
				log::debug!("r==============={:?}, {:?}", r, key);
				FontId(key)
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
const BLOD_FACTOR: f32 = 1.13;

pub struct GlyphIdDesc {
	pub font_id: FontFamilyId,
	pub char: char,
	pub glyph: Glyph,
	pub font_face_index: usize,
}

pub struct BaseCharDesc {
	pub font_id: FontFamilyId,
	pub char: char,
	pub width: f32,
	pub font_face_index: usize,
}

#[derive(Debug)]
pub struct FontInfo {
	pub font: Font,
	pub font_ids: SmallVec<[FontId; 1]>,
	pub height: f32,
	pub max_height: f32,
	pub await_info: AwaitInfo,
	pub base_font_id: FontFamilyId,
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
	pub font_id: FontFamilyId, 
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