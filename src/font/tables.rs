

use super::{sdf_table::SdfTable, font::{FontId, FontInfo, FontType, GlyphId, BASE_FONT_SIZE, Size}, bitmap_table::BitmapTable, sdf2_table::Sdf2Table};

pub struct FontTable {
	// 本地画笔，用于测量、绘制文字，不同平台可能有不同的实现（web平台依赖于canvas的文字功能， app、exe平台通常可以使用freetype）
	pub bitmap_table: BitmapTable,
	// sdf字体表
	pub sdf_table: SdfTable,

	// sdf2字体表
	pub sdf2_table: Sdf2Table,
}

impl FontTable {
	/// 创建字体表
	pub fn new(width: usize, height: usize) -> Self {
		Self {
			bitmap_table: BitmapTable::new(width, height),
			sdf_table: SdfTable::new(width, height),
			sdf2_table: Sdf2Table::new(width, height),
		}
	}

	pub fn size(&self, font_type: FontType) -> Size<usize> {
		match font_type {
			FontType::Bitmap => Size { width: self.bitmap_table.text_packer.width, height: self.bitmap_table.text_packer.height },
			FontType::Sdf1 => Size { width: self.sdf_table.text_packer.width, height: self.sdf_table.text_packer.height },
			FontType::Sdf2 =>  Size { width: self.sdf2_table.index_packer.width, height: self.sdf2_table.index_packer.height },
		}
	}

	pub fn check_or_create_face(& mut self, font: &FontInfo, font_type: FontType) {
		if font_type == FontType::Bitmap {
			self.bitmap_table.brush.check_or_create_face(font);
		}
	}

	pub fn height(&mut self, font_id: FontId, font: &FontInfo, font_type: FontType) -> (f32, f32) {
		if font_type == FontType::Sdf1 {
			self.sdf_table.height(font_id, font)
		} else if font_type == FontType::Sdf2 {
			self.sdf2_table.height(font)
		} else {
			let mut r = self.bitmap_table.brush.base_height(font);
			log::warn!("height======={:?}, {:?}", r, font);
			// max_height, todo
			r = font.font.font_size as f32 / BASE_FONT_SIZE as f32 * r;
			(r, r)
		}
	}

	/// 测量宽度
	pub fn measure_width(&mut self, f: FontId, font: &mut FontInfo,  char: char, font_type: FontType) -> f32 {
		if font_type == FontType::Sdf1 {
			self.sdf_table.width(font, char).0
		} else if font_type == FontType::Sdf2 {
			self.sdf2_table.width(f, font, char).0
		} else {
			let base_w = self.bitmap_table.brush.base_width( font, char);
			let ratio = font.font.font_size as f32 / BASE_FONT_SIZE as f32;
			let r = ratio * base_w.0 + *font.font.stroke;

			log::warn!("width======={:?}, {:?}, {:?}, {:?}", base_w, char, r, font);
			r
		}
	}

	pub fn glyph_id(&mut self, f: FontId, char: char, font_info: &mut FontInfo, font_type: FontType) -> Option<GlyphId> {
		match font_type {
			FontType::Bitmap => {
				self.bitmap_table.glyph_id(f, font_info, char)
			},
			FontType::Sdf1 => {
				self.sdf_table.glyph_id(f, font_info, char)
			},
			FontType::Sdf2 => Some(self.sdf2_table.glyph_id(f, font_info, char)),
		}
	}

	pub fn clear(&mut self) {

	}
}