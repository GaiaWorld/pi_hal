//! 位图文字实现
//! 与平台相关， 在web平台依赖于canvas来测量、绘制文字

use std::collections::hash_map::Entry;

use pi_hash::XHashMap;
use pi_share::ThreadSync;
use pi_slotmap::{SlotMap, DefaultKey};
use crate::font_brush::Brush;

use super::{text_pack::TextPacker, font::{Size, FontId, GlyphId, GlyphIdDesc, Glyph, FontInfo, BLOD_WEIGHT, BLOD_FACTOR, BASE_FONT_SIZE, FontFamilyId, Block, FontImage, Await, DrawBlock}};

pub struct BitmapTable {
	glyph_id_map: XHashMap<(FontId, char), GlyphId>,
	glyphs: SlotMap<DefaultKey, GlyphIdDesc>,
	// base_glyphs: SlotMap<DefaultKey, BaseCharDesc>,
	base_glyph_id_map: XHashMap<(FontFamilyId, char), BaseCharDesc>,

	pub(crate) text_packer: TextPacker,

	pub brush: Brush,
}

impl BitmapTable{
	pub fn new(width: usize, height: usize) -> Self {
		Self { 
			glyph_id_map: XHashMap::default(),
			glyphs: SlotMap::default(),
			// base_glyphs: SlotMap<DefaultKey, BaseCharDesc>,

			text_packer: TextPacker::new(width, height),
			base_glyph_id_map: XHashMap::default(),
			brush: Brush::new(),
		}
	}
	pub fn glyph_id(&mut self, f: FontId, font_info: &mut FontInfo, char: char) -> Option<GlyphId> {
		let id = match self.glyph_id_map.entry((f, char)) {
			Entry::Occupied(r) => {
				let id = r.get().clone();
				return Some(id);
			},
			Entry::Vacant(r) => {
				// 分配GlyphId
				let id = GlyphId(self.glyphs.insert(GlyphIdDesc{
					font_id: f,
					char,
					font_face_index: 0,
					glyph: Glyph {
						x: 0.0, 
						y: 0.0, 
						plane_min_x: 0.0,
						plane_min_y: 0.0,
						plane_max_x: 0.0,
						plane_max_y: 0.0,
						width: 0.0, 
						height: 0.0,
						advance: 0.0,},
				}));

				r.insert(id).clone()
			}
		};

		// let ff = font.font.font_family_string.clone();
		let max_height = font_info.max_height;
		let char_texture_size: Size<f32> = {
			font_info.await_info.wait_list.push(id);

			let (base_w, index) = self.measure_base(font_info, char);
			let glyph = &mut self.glyphs[id.0];
			glyph.font_face_index = index;

			Size {
				width: base_w * (font_info.font.font_size as f32 / BASE_FONT_SIZE as f32) + *font_info.font.stroke + 2.0, 
				height: font_info.height,
			}
		};

		// 放入等待队列, 并统计等待队列的总宽度
		font_info.await_info.size.width += char_texture_size.width.ceil() as usize;
		font_info.await_info.size.height += char_texture_size.height.ceil() as usize;

		// 在纹理中分配一个位置
		let tex_position = self.text_packer.alloc(
			char_texture_size.width.ceil() as usize, 
			max_height.ceil() as usize);
		let tex_position = match tex_position {
			Some(r) => r,
			None => return None,
		};
		// log::warn!("char_texture_size====={:?}, {:?}, {:?}, {:?}", &char, &char_texture_size, tex_position, ff);
		let g = &mut self.glyphs[*id];
		g.glyph.width = char_texture_size.width.round();
		g.glyph.height = char_texture_size.height;
		g.glyph.x = tex_position.x as f32;
		g.glyph.y = tex_position.y as f32;

		log::warn!("g1============={:?}, {:?}", id, char);

		Some(id)
	}
	
	/// 取到字形信息
	pub fn glyph(&self, id: GlyphId) -> &Glyph {
		if self.glyphs.get(*id).is_none() {
			panic!("glyph is not exist, {:?}", id);
		}
		&self.glyphs[*id].glyph
	}

	/// 绘制文字
	pub fn draw<F: FnMut(Block, FontImage) + Clone + ThreadSync + 'static>(&mut self, fonts: &mut SlotMap<DefaultKey, FontInfo>, update: F) {
		// let (fonts, glyphs) = (&mut self.fonts, &self.glyphs);
		let width = self.text_packer.width;
		// let (sheet, brush) = (&mut self.sheet, &mut self.brush);
		// let (glyphs, fonts) = (&sheet.glyphs, &mut sheet.fonts);

		// let mut sdf_all_draw = Vec::new();
		let mut native_all_draw = Vec::new();
		// let mut sdf_all_draw_slotmap = Vec::new();
		// let mut f = Vec::new();
		// 遍历所有支持的字体，如果其对应的绘制等待列表不为空，则进行绘制
		for (k, font_info) in fonts.iter_mut() {
			let await_info = &font_info.await_info;
			if await_info.wait_list.len() == 0 {
				continue;
			}

			log::warn!("await_info.wait_list========={:?}", &await_info.wait_list);

			let offset = *font_info.font.stroke/2.0;

			// let g_0 = &glyphs[*await_info.wait_list[0]];
			let mut start_pos;
			let (mut y, mut height, mut font_face_index);

			let (mut start, mut end) = (0, 0.0);
			let mut x_c;

			while start < await_info.wait_list.len() {
				let g = &self.glyphs[*await_info.wait_list[start]];
				log::warn!("g============={:?}, {:?}", &g.glyph, g.char);
				start_pos = (g.glyph.x, g.glyph.y);
				y = g.glyph.y as f32;
				font_face_index = g.font_face_index;
				height = g.glyph.height;

				x_c = Vec::new();

				// 每一批次绘制，只绘制同一行的字符
				for i in start..await_info.wait_list.len() {
					let g = &self.glyphs[
						*await_info.wait_list[i]
					];
					log::warn!("g============={:?}, {:?}", &g.glyph, g.char);

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
					font_id: FontId(k),
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
				native_all_draw.push(block);
				
			}

			font_info.await_info.wait_list.clear();
			font_info.await_info.size = Size {width: 0, height: 0};// 似乎没有作用？
		}

		if native_all_draw.len() > 0 {
			// 绘制一个批次的字符
			self.brush.draw(native_all_draw, update);
		}
	}


	fn measure_base(&mut self, font_info: &FontInfo, char: char) -> (f32, usize) {
		match self.base_glyph_id_map.entry((font_info.font_family_id, char)) {
			Entry::Occupied(r) => {
				let g = r.get();
				(g.width, g.font_face_index)
			},
			Entry::Vacant(r) => {
				let (mut width, index) = self.brush.base_width(font_info, char);

				r.insert(BaseCharDesc{
					font_family_id: font_info.font_family_id,
					char,
					width,
					font_face_index: index,
				});

				// 如果是ascii字符， 其粗体文字的宽度会适当加宽（浏览器实验所得结果）
				let is_blod = char.is_ascii() && font_info.font.font_weight >= BLOD_WEIGHT;
				if is_blod {
					width = width * BLOD_FACTOR;
				}

				(width, index)
			}
		}
	}
}

pub struct BaseCharDesc {
	pub font_family_id: FontFamilyId,
	pub char: char,
	pub width: f32,
	pub font_face_index: usize,
}