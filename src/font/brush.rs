

use crate::font_brush::Brush;

use super::{sdf_brush::SdfBrush, font::{FontFamilyId, FontInfo}};

pub struct FontBrush {
	// 本地画笔，用于测量、绘制文字，不同平台可能有不同的实现（web平台依赖于canvas的文字功能， app、exe平台通常可以使用freetype）
	pub(crate) native_brush: Brush,
	// sdf画笔
	pub sdf_brush: SdfBrush,
}

impl FontBrush {
	pub fn new() -> Self {
		Self {
			native_brush: Brush::new(),
			sdf_brush: SdfBrush::default(),
		}
	}
	pub fn check_or_create_face(& mut self, font_id: FontFamilyId, font: &FontInfo, is_sdf: bool) {
		if !is_sdf {
			self.native_brush.check_or_create_face(font_id, font);
		}
	}

	pub fn height(&mut self, font_id: FontFamilyId, font: &FontInfo, is_sdf: bool) -> (f32, f32) {
		if is_sdf {
			self.sdf_brush.height(font_id, font)
		} else {
			let r = self.native_brush.height(font_id, font);
			// max_height, todo
			(r, r)
		}
	}

	pub fn width(&mut self, font_id: FontFamilyId, font: &FontInfo,  char: char, is_sdf: bool) -> (f32, usize){
		if is_sdf {
			self.sdf_brush.width(font_id, font, char)
		} else {
			self.native_brush.width(font_id, font, char)
		}
	}
}