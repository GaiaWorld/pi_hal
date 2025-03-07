//! 字体管理模块
//!
//! 本模块提供字体加载、字形管理、字体渲染等功能，支持位图和SDF两种渲染模式。
//!
//! # 主要功能
//! - 字体元数据管理（字体家族、字号、字重等）
//! - 字形纹理打包与缓存
//! - 字体度量信息计算
//! - SDF（有符号距离场）生成与处理
//! - 多平台字体渲染抽象

use std::{
	hash::Hash, 
	collections::hash_map::Entry, 
};

use derive_deref::{Deref, DerefMut};
use ordered_float::NotNan;
use parry2d::bounding_volume::Aabb;
use pi_hash::XHashMap;
use pi_share::Share;
use pi_slotmap::{DefaultKey, SlotMap};
use serde::{Serialize, Deserialize};
use pi_null::Null;

use pi_atom::Atom;
use smallvec::SmallVec;

use super::{sdf_table::{FontCfg, MetricsInfo}, tables::FontTable};

/// 通用尺寸结构体
/// 
/// # 泛型参数
/// - `T`: 数值类型，支持任意数值类型（如usize, f32等）
/// 
/// # 示例
/// ```
/// let size = Size { width: 1024, height: 768 };
/// ```
#[derive(Debug, Clone)]
pub struct Size<T> {
	pub width: T,
	pub height: T,
}

/// 矩形区域描述
/// 
/// 用于表示纹理中的矩形区域，坐标系以左上角为原点
#[derive(Debug, Clone, Copy)]
pub struct Block {
	pub y: f32,    // 矩形顶部Y坐标
	pub x: f32,    // 矩形左侧X坐标 
	pub width: f32, // 矩形宽度
	pub height: f32, // 矩形高度
}

pub struct FontImage {
	pub buffer: Vec<u8>,
	pub width: usize,
	pub height: usize,
}

pub struct ShadowImage {
	pub minimip: Vec<Vec<u8>>,
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
	pub font_type: FontType,
	// pub is_outer_glow: Option<u32>,
	// pub shadow: Option<(NotNan<f32>,  NotNan<f32>)>,
	pub font_weight: usize,
	// pub stroke: NotNan<f32>,
}

impl Font {
	pub fn new(font_family_string: Atom, font_size: usize, font_weight: usize) -> Self {
		let font_family = font_family_string.split(",");
		let font_family = font_family.map(|r| {
			Atom::from(r.trim())
		}).collect::<SmallVec<[Atom; 1]>>();
		Self {
			font_family_string,
			font_family,
			
			font_size,
			font_type: FontType::Bitmap,
			font_weight,
			// stroke,
			// is_outer_glow,
			// shadow
		}
	}
}

// #[derive(Debug, Clone, Hash, PartialEq, Eq)]
// pub struct Font {
// 	pub(crate) font: Font,
// 	pub(crate) font_id: SmallVec<[FontId; 1]>,
// }

/// 字形唯一标识符
/// 
/// 基于SlotMap的键值，用于快速查找字形信息
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut)]
pub struct GlyphId(pub DefaultKey);

/// 字体唯一标识符
/// 
/// 表示已加载的字体实例，包含字体属性哈希和SlotMap键值
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut, Default)]
pub struct FontId(pub DefaultKey);

/// 字体家族标识符
/// 
/// 表示一组字体家族名称（如["Arial", "sans-serif"]）
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut, Default)]
pub struct FontFamilyId(pub DefaultKey);


/// 字体外观标识符
/// 
/// 表示特定字体文件（如"arial.ttf"）的加载实例
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Deref, DerefMut)]
pub struct FontFaceId(pub DefaultKey);

/// 字体渲染类型枚举
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontType {
	// 位图渲染 - 使用传统光栅化方式
	Bitmap, 
	// SDF1模式 - 基于预生成的SDF配置表
	Sdf1,
	// SDF2模式 - 实时计算贝塞尔曲线距离场
	Sdf2,
}

impl Default for FontType {
    fn default() -> Self {
        Self::Bitmap
    }
}

/// 字体管理器
/// 
/// 负责管理所有字体资源，提供统一的字体接口
pub struct FontMgr {
	pub sheet: GlyphSheet,    // 字形表，存储所有字形元数据
	pub table: FontTable,     // 字体表，处理平台相关的字体操作
	pub font_type: FontType,  // 当前激活的渲染模式
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
	pub font_names: SlotMap<DefaultKey, Atom>,
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
	/// 创建新的字体管理器
	/// 
	/// # 参数
	/// - `width`: 纹理图集宽度
	/// - `height`: 纹理图集高度  
	/// - `font_type`: 初始渲染模式
	/// - `device`: wgpu设备共享句柄
	/// - `queue`: wgpu命令队列共享句柄
	pub fn new(width: usize, height: usize, font_type: FontType, device: Share<pi_wgpu::Device>, queue: Share<pi_wgpu::Queue>) -> FontMgr {
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
			table: FontTable::new(width, height, device, queue ),
			font_type,
		}
	}

	/// 获取当前纹理图集尺寸
	pub fn size(&self) -> Size<usize> {
		self.table.size(self.font_type)
	}

	/// 设置当前字体渲染模式
	pub fn set_font_type(&mut self, font_type: FontType) {
		self.font_type = font_type
	}

	/// 获取当前字体渲染模式
	pub fn font_type(&self) -> FontType {
		self.font_type
	}
}

impl FontMgr {
	/// 获取或创建字体ID
	/// 
	/// # 参数
	/// - `f`: 字体描述符
	/// 
	/// # 返回值
	/// 唯一标识该字体配置的FontId
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

		// println!("aaa========={:?}", (font_id, f, font_family_id, &font_face_ids));

		let font_info = &mut self.sheet.fonts[font_id.0];

		self.table.check_or_create_face(font_info, self.font_type);

		let (height, max_height) = self.table.height(font_id, font_info, self.font_type);
		font_info.height = height;
		font_info.max_height = max_height;

		font_id
	}

	/// 获取字体家族ID
	/// 
	/// 如果指定的字体家族不存在，会自动创建新条目
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

	/// 获取字体元信息
	pub fn font_info(&self, f: FontId) -> &FontInfo {
		&self.sheet.fonts[f.0]
	}

	/// 计算字体渲染高度
	/// 
	/// # 参数
	/// - `f`: 字体ID
	/// - `font_size`: 请求的字号（单位：像素）
	pub fn font_height(&self, f: FontId, font_size: usize) -> f32 {
		match self.sheet.fonts.get(*f) {
			Some(r) =>  {if r.height < 2.0 {r.height * font_size as f32} else {r.height}},
			None => font_size as f32, // 异常情况，默认返回font_size
		}
	}

	/// 获取字符的字形ID
	/// 
	/// 如果字形尚未渲染，会自动触发渲染流程
	/// 
	/// # 返回值
	/// Option<GlyphId> - 当纹理图集已满时返回None
	pub fn glyph_id(&mut self, f: FontId, char: char) -> Option<GlyphId> {
		let font_info = &mut self.sheet.fonts[f.0];
		self.table.glyph_id(f, char, font_info, self.font_type)
	}

	/// 为字形添加阴影效果
	/// 
	/// # 参数
	/// - `f`: 字体ID
	/// - `id`: 目标字形ID
	/// - `radius`: 阴影半径（像素）
	/// - `weight`: 阴影强度（0.0-1.0）
	pub fn add_font_shadow(&mut self, f: FontId, id: GlyphId, radius: u32, weight: NotNan<f32>) {
		let font_info = &self.sheet.fonts[f.0];
		self.table.sdf2_table. add_font_shadow(id, font_info, radius, weight);
	}

	/// 添加外发光效果
	/// 
	/// # 参数
	/// - `range`: 发光范围（像素）
	pub fn add_font_outer_glow(&mut self, f: FontId, id: GlyphId, range: u32) {
		let font_info = &self.sheet.fonts[f.0];
		self.table.sdf2_table. add_font_outer_glow(id, font_info, range);
	}

	/// 测量字符宽度
	/// 
	/// # 返回值
	/// 字符的布局宽度（单位：像素）
	pub fn measure_width(&mut self, f: FontId, char: char) -> f32 {
		let font_info = match self.sheet.fonts.get_mut(*f) {
			Some(r) => r,
			None => return 0.0,
		};
		self.table.measure_width(f, font_info, char, self.font_type)
	}

	/// 获取字形度量信息
	pub fn metrics(&self, id: GlyphId) -> Option<&MetricsInfo> {
		let desc = self.table.glyph_id_desc(id, self.font_type);
		let font_info = match self.sheet.fonts.get(desc.font_id.0) {
			Some(r) => r,
			None => return None,
		};
		self.table.metrics(id, font_info, self.font_type)
	}

	/// 获取字体全局度量信息
	pub fn font_metrics(&self, font_id: FontId) -> Option<&MetricsInfo> {
		let font_info = match self.sheet.fonts.get(font_id.0) {
			Some(r) => r,
			None => return None,
		};
		if font_info.font_ids.len() > 0 &&  font_info.font_ids[0].0.is_null(){
			return self.table.fontface_metrics(font_info.font_ids[0], self.font_type)
		}
		
		return None;
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

	/// 添加SDF配置项
	/// 
	/// # 参数
	/// - `font_cfg`: SDF字体配置信息
	pub fn add_sdf_cfg(&mut self, font_cfg: FontCfg) {
		let font_face = Atom::from(font_cfg.name.clone());
		let font_face_id = self.create_font_face(&font_face);
		self.table.sdf_table.add_cfg(font_face_id, font_cfg);
	}

	/// 添加默认SDF字符
	/// 
	/// 用于预生成常用字符的SDF数据
	pub fn add_sdf_default_char(&mut self, _font_face: Atom, _char: char) {
		// let font_face_id = self.create_font_face(&font_face);
		// let font_family_id = self.font_family_id(Font::new(font_face.clone(), BASE_FONT_SIZE, 500, unsafe{ NotNan::new_unchecked(0.0)}));
		// let glyph_id = self.glyph_id(font_family_id, char).unwrap();
		// self.brush.sdf_brush.add_default_char(font_face_id, glyph_id, font_face.clone(), char);
		// self.default_sdf_char.push((font_face, char));
	}

	/// 获取可变的字体信息表
	pub fn fonts_mut(&mut self) -> &mut SlotMap<DefaultKey, FontInfo> {
		&mut self.sheet.fonts
	}

	/// 清理所有缓存数据
	/// 
	/// 重置纹理图集并保留预生成的默认字符
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

	/// 内部方法：获取或插入字体记录
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
	
	

	/// 创建字体外观实例
	/// 
	/// # 参数
	/// - `r`: 字体名称原子（如"Arial"）
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
/// SDF偏移量范围常量，用于坐标编码优化
pub const OFFSET_RANGE: f32 = (2_u32).pow(15) as f32;

/// 粗体字的最小字重阈值
pub const BLOD_WEIGHT: usize = 700;

/// 粗体字缩放因子，用于自动调整字符宽度
pub const BLOD_FACTOR: f32 = 1.13;

/// 字形描述信息
#[derive(Debug)]
pub struct GlyphIdDesc {
	pub font_id: FontId,        // 所属字体ID
	pub char: char,             // Unicode字符
	pub glyph: Glyph,           // 字形度量信息
	pub font_face_index: usize,  // 字体外观索引
}

/// 字体元信息
#[derive(Debug)]
pub struct FontInfo {
	pub font: Font,                // 字体配置
	pub font_ids: SmallVec<[FontFaceId; 1]>, // 字体外观ID列表
	pub height: f32,               // 实际渲染高度
	pub max_height: f32,           // 最大字符高度
	pub await_info: AwaitInfo,      // 待渲染队列信息
	pub font_family_id: FontFamilyId, // 字体家族ID
}

/// 待渲染队列信息
#[derive(Debug)]
pub struct AwaitInfo {
	pub size: Size<usize>,      // 等待区域尺寸
	pub wait_list: Vec<GlyphId>, // 等待渲染的字形ID列表
}


/// 字形度量信息
#[derive(Debug, Default, Clone)]
pub struct Glyph {
	pub plane_min_x: f32,  // 最小X坐标（相对于字体高度的百分比）
	pub plane_min_y: f32,  // 最小Y坐标（相对于字体高度的百分比）
	pub plane_max_x: f32,  // 最大X坐标（相对于字体高度的百分比）
	pub plane_max_y: f32,  // 最大Y坐标（相对于字体高度的百分比）
	pub x: f32,            // 纹理X坐标（像素）
    pub y: f32,            // 纹理Y坐标（像素）
	pub width: f32,        // 纹理宽度（像素）
    pub height: f32,       // 纹理高度（像素）
	pub advance: f32,      // 布局步进宽度（相对于字体高度的百分比）
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
	// pub font_stroke: NotNan<f32>,
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
