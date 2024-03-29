// 此版本，因为频繁的创建fontface很费， 暂不使用
use pi_atom::Atom;
use pi_share::ThreadSync;
use pi_slotmap::{SecondaryMap, DefaultKey};
use font_kit::{font::Face, util::{ WritePixel, Rgba}};
use smallvec::SmallVec;

use crate::font::font::{FontFamilyId, FontImage, Block, Await, DrawBlock, FontInfo};

pub struct Brush {
	faces: SecondaryMap<DefaultKey, SmallVec<[Option<Face>; 1]> >,
	default_family: Atom,
}

impl Brush {
	pub fn new() -> Self {
		Brush {
			faces: SecondaryMap::default(),
			default_family: Atom::from("default"),
		}
	}

	pub fn check_or_create_face(& mut self, font_id: FontFamilyId, font: &FontInfo) {
		if self.faces.get_mut(*font_id).is_some() {
			return;
		}
		let mut faces = SmallVec::new();
		// Face::from_family_name("default", font.font_size as u32).unwrap();
		for font_family in font.font.font_family.iter().chain([self.default_family.clone()].iter()) {
			let time = pi_time::Instant::now();
			faces.push(match Face::from_family_name(font_family, font.font.font_size as u32) {
				Ok(mut face) => {
					if *font.font.stroke > 0.0 {
						face.set_stroker_width(*font.font.stroke as f64);
					}
					Some(face)
				},
				Err(_) => None
			});
			log::warn!("font face======font_id={:?},{:?}, {:?}, {:?}", font_id, font_family, font, pi_time::Instant::now() - time);
		}
		self.faces.insert(*font_id, faces);
		// log::trace!("check_or_create_face!!!========{:?}, {:p}, {:?}", *font_id, &self.faces[*font_id], &self.faces[*font_id]);
	}

	pub fn height(&mut self, font_id: FontFamilyId, font: &FontInfo) -> f32 {
		let faces = &mut self.faces[*font_id];
		// log::trace!("height!!!========{:?}, {:p}, {:?}", *font_id, face, face);
		// face.set_pixel_sizes(font.font_size as u32);
		for face in faces.iter() {
			if let Some(face) = face{
				let metrics = face.get_global_metrics();
				return metrics.ascender as f32 - metrics.descender as f32
			}
		}
		panic!("font is not exist, font_family={:?}, and default font is none", &font.font.font_family);
		// let metrics = faces.get_global_metrics();
		// metrics.ascender as f32 - metrics.descender as f32
	}

    pub fn width(&mut self, font_id: FontFamilyId, font: &FontInfo, char: char) -> (f32, usize/*fontface在数组中的索引*/) {
		let faces = &mut self.faces[*font_id];
	
		for (index,face) in faces.iter().enumerate() {
			if let Some(face) = face {
				if let Ok(metrics) = face.get_metrics(char) {
					return (metrics.hori_advance as f32, index)
				}
			}
		}

		panic!("font is not exist, font_family={:?}, and default font is none", &font.font.font_family);
    }

    pub fn draw<F: FnMut(Block, FontImage) + Clone + ThreadSync + 'static>(
		&mut self, 
		draw_list: Vec<DrawBlock>,
		mut update: F) {
		// 修改为异步，TODO
		for draw_block in draw_list.into_iter() {
			let faces = match self.faces.get_mut(*draw_block.font_id) {
				Some(r) => r,
				None => return ,
			};
			let face = faces[draw_block.font_face_index].as_mut().unwrap();
			// 绘制
			// face.set_pixel_sizes(draw_block.font_size as u32);
			// face.set_stroker_width(*draw_block.font_stroke as f64);

			let (block, image) = draw_sync(
				draw_block.chars, 
				draw_block.block,
				face,
				*draw_block.font_stroke as f64
			);

			update(block, image);
		}
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

