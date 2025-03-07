

use pi_share::Share;
use pi_wgpu as wgpu;
use super::{bitmap_table::BitmapTable, font::{FontFaceId, FontId, FontInfo, FontType, GlyphId, GlyphIdDesc, Size, BASE_FONT_SIZE}, sdf2_table::Sdf2Table, sdf_table::{MetricsInfo, SdfTable}};

/// 字体表管理器，负责管理不同字体渲染方式的存储和查询
pub struct FontTable {
	/// 位图字体表，用于处理基于位图的字体渲染
	/// - Web平台依赖Canvas文字功能
	/// - 本地平台(app/exe)通常使用freetype实现
	pub bitmap_table: BitmapTable,
	
	/// SDF1字体表，用于基于有符号距离场的一代字体渲染
	pub sdf_table: SdfTable,
	
	/// SDF2字体表，用于改进版的有符号距离场字体渲染
	pub sdf2_table: Sdf2Table,
}

impl FontTable {
	/// 创建新的字体表实例
	/// 
	/// # 参数
	/// - `width`: 纹理图集初始宽度
	/// - `height`: 纹理图集初始高度  
	/// - `device`: wgpu图形设备共享实例
	/// - `queue`: wgpu命令队列共享实例
	pub fn new(width: usize, height: usize, device: Share<wgpu::Device>, queue: Share<wgpu::Queue>) -> Self {
		Self {
			bitmap_table: BitmapTable::new(width, height),
			sdf_table: SdfTable::new(width, height),
			sdf2_table: Sdf2Table::new(width, height, device, queue),
		}
	}

	/// 获取指定字体类型的纹理图集尺寸
	/// 
	/// # 参数
	/// - `font_type`: 字体渲染类型枚举
	/// 
	/// # 返回值
	/// 返回包含宽度和高度的Size结构体
	pub fn size(&self, font_type: FontType) -> Size<usize> {
		match font_type {
			FontType::Bitmap => Size { width: self.bitmap_table.text_packer.width, height: self.bitmap_table.text_packer.height },
			FontType::Sdf1 => Size { width: self.sdf_table.text_packer.width, height: self.sdf_table.text_packer.height },
			FontType::Sdf2 =>  Size { width: self.sdf2_table.index_packer.width, height: self.sdf2_table.index_packer.height },
		}
	}

	/// 检查并创建对应的字体face对象
	/// 
	/// # 参数
	/// - `font`: 字体信息引用
	/// - `font_type`: 字体渲染类型枚举
	/// 
	/// # 注意
	/// 当前仅对位图字体类型有效
	pub fn check_or_create_face(& mut self, font: &FontInfo, font_type: FontType) {
		if font_type == FontType::Bitmap {
			self.bitmap_table.brush.check_or_create_face(font);
		}
	}

	/// 计算字体的垂直度量信息
	/// 
	/// # 参数
	/// - `font_id`: 字体ID
	/// - `font`: 字体信息引用
	/// - `font_type`: 字体渲染类型枚举
	/// 
	/// # 返回值
	/// 返回元组：(实际高度, 行高)
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
	
	/// 测量指定字符的显示宽度
	/// 
	/// # 参数
	/// - `f`: 字体ID
	/// - `font`: 可变字体信息引用
	/// - `char`: 需要测量的字符
	/// - `font_type`: 字体渲染类型枚举
	/// 
	/// # 返回值
	/// 返回字符宽度（像素）
	/// 
	/// # 注意
	/// 位图字体类型的实现待完善
	pub fn measure_width(&mut self, f: FontId, font: &mut FontInfo,  char: char, font_type: FontType) -> f32 {
		if font_type == FontType::Sdf1 {
			self.sdf_table.width(font, char).0
		} else if font_type == FontType::Sdf2 {
			self.sdf2_table.width(f, font, char).0
		} else {
			// let base_w = self.bitmap_table.brush.base_width( font, char);
			// let ratio = font.font.font_size as f32 / BASE_FONT_SIZE as f32;
			// let r = ratio * base_w.0 + *font.font.stroke;

			// log::warn!("width======={:?}, {:?}, {:?}, {:?}", base_w, char, r, font);
			// r
			todo!()
		}
	}

	/// 获取指定字形的度量信息
	/// 
	/// # 参数
	/// - `id`: 字形ID
	/// - `font`: 字体信息引用
	/// - `font_type`: 字体渲染类型枚举
	/// 
	/// # 返回值
	/// 返回Option包装的MetricsInfo引用
	pub fn metrics(&self, id: GlyphId, font: &FontInfo, font_type: FontType) -> Option<&MetricsInfo> {
		if font_type == FontType::Sdf1 {
			todo!()
		} else if font_type == FontType::Sdf2 {
			self.sdf2_table.metrics(id, font)
		} else {
			todo!()
		}
	}

	/// 获取字体face的全局度量信息
	/// 
	/// # 参数
	/// - `face_id`: 字体face ID
	/// - `font_type`: 字体渲染类型枚举
	/// 
	/// # 返回值
	/// 返回Option包装的MetricsInfo引用
	pub fn fontface_metrics(&self, face_id: FontFaceId, font_type: FontType) -> Option<&MetricsInfo> {
        if font_type == FontType::Sdf1 {
			todo!()
		} else if font_type == FontType::Sdf2 {
			self.sdf2_table.fontface_metrics(face_id)
		} else {
			todo!()
		}
	}

	/// 获取字形ID的详细描述
	/// 
	/// # 参数
	/// - `glyph_id`: 字形ID
	/// - `font_type`: 字体渲染类型枚举
	/// 
	/// # 返回值
	/// 返回GlyphIdDesc的引用
	pub fn glyph_id_desc(&self, glyph_id: GlyphId, font_type: FontType) -> &GlyphIdDesc {
		if font_type == FontType::Sdf1 {
			todo!()
		} else if font_type == FontType::Sdf2 {
			self.sdf2_table.glyph_id_desc(glyph_id)
		} else {
			todo!()
		}
    }

	/// 获取字符对应的字形ID
	/// 
	/// # 参数
	/// - `f`: 字体ID
	/// - `char`: 目标字符
	/// - `font_info`: 可变字体信息引用
	/// - `font_type`: 字体渲染类型枚举
	/// 
	/// # 返回值
	/// 返回Option包装的GlyphId
	pub fn glyph_id(&mut self, f: FontId, char: char, font_info: &mut FontInfo, font_type: FontType) -> Option<GlyphId> {
		match font_type {
			FontType::Bitmap => {
				self.bitmap_table.glyph_id(f, font_info, char)
			},
			FontType::Sdf1 => {
				self.sdf_table.glyph_id(f, font_info, char)
			},
			FontType::Sdf2 => self.sdf2_table.glyph_id(f, font_info, char),
		}
	}

	/// 清空所有字体表内容
	/// 
	/// # 注意
	/// 当前实现待完善，需要具体清理逻辑
	pub fn clear(&mut self) {
		// TODO: 实现具体的清理逻辑
	}
}
