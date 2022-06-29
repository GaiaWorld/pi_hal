
use pi_slotmap::{SecondaryMap, DefaultKey};
use font_kit::{font::Face, util::{ WritePixel, Rgba}};

use crate::font::font::{FontId, Font, GlyphSheet, FontImage, Block};

// use pi_render::rhi::RenderQueue;
// use wgpu::{ImageDataLayout, ImageCopyTexture, Origin3d, TextureAspect, Extent3d};

// use super::font::{FontBrush, Font, FontId, GlyphSheet};

pub struct Brush {
	faces: SecondaryMap<DefaultKey, Face>,
	// queue: RenderQueue,
}

impl Brush {
	pub fn new() -> Self {
		Brush {
			faces: SecondaryMap::default(),
		}
	}

	fn check_or_create_face(& mut self, font_id: FontId, font: &Font) {
		if self.faces.get_mut(*font_id).is_some() {
			return;
		}
		let mut face = match Face::from_family_name(&font.font_family) {
			Ok(r) => r,
			Err(_) => Face::from_family_name("default").unwrap()
		};
		face.set_pixel_sizes(font.font_size as u32);
		self.faces.insert(*font_id, face);
		
	}
}

impl Brush {
	pub fn height(&mut self, font_id: FontId, font: &Font) -> f32 {
		self.check_or_create_face(font_id, font);
		let face = &mut self.faces[*font_id];
		face.set_pixel_sizes(font.font_size as u32);
		face.get_global_metrics().height as f32
	}

    pub fn width(&mut self, font_id: FontId, font: &Font, char: char) -> f32 {
        self.check_or_create_face(font_id, font);
		let face = &mut self.faces[*font_id];
		if face.get_size() != font.font_size as u32 {
			face.set_pixel_sizes(font.font_size as u32);
		}

		let metrics = face.get_metrics(char).unwrap();
		metrics.hori_advance as f32
    }

    pub fn draw<F: FnMut(Block, FontImage)>(&mut self, sheet: &GlyphSheet, mut update: F) {
        let fonts = sheet.fonts();
		let glyphs = sheet.glyphs();

		for (k, font_info) in fonts.iter() {
			let await_info = &font_info.await_info;
			if await_info.wait_list.len() == 0 {
				continue;
			}
			let face = match self.faces.get_mut(k) {
				Some(r) => r,
				None => continue,
			};
			let g_0 = &glyphs[*await_info.wait_list[0]];
			let mut start_pos = (g_0.glyph.x, g_0.glyph.y);

			let (mut start, mut pos) = (0, 0.0);
			let (mut y, mut height) = (g_0.glyph.y as f32, g_0.glyph.height);
			let mut x_c = Vec::new();
			while start < await_info.wait_list.len() {
				for i in start..await_info.wait_list.len() {
					let g = &glyphs[
						*await_info.wait_list[i]
					];
					let y1 = g.glyph.y as f32;

					if y1 != y {
						y = y1;
						height = g.glyph.height;
						start_pos = (g.glyph.x, g.glyph.y);
						break;
					}
					x_c.push(Await {
						x_pos: pos,
						char: g.char,
					});
					pos += g.glyph.width;
				}
				start += x_c.len();

				// 绘制
				face.set_pixel_sizes(font_info.font.font_size as u32);
				face.set_stroker_width(*font_info.font.stroke as f64);
				let (block, image) = draw_sync(
					x_c, 
					Block {
						x: start_pos.0 as f32,
						y: start_pos.1 as f32,
						width: pos.ceil(),
						height: height,
					},
					face
				);
				update(block, image);
				// self.queue.write_texture(
				// 	ImageCopyTexture {
				// 		texture: sheet.texture(),
				// 		mip_level: 0,
				// 		origin: Origin3d {
				// 			x: block.x as u32,
				// 			y: block.y as u32,
				// 			z: 0
				// 		},
				// 		aspect: TextureAspect::All
				// 	}, 
				// 	image.buffer.as_slice(),
				// 	ImageDataLayout {
				// 		offset: 0,
				// 		bytes_per_row: NonZeroU32::new(image.width as u32 * 4), // 32 * 4
				// 		rows_per_image: None,
				// 	},
				// 	Extent3d {
				// 		width: image.width as u32,
				// 		height: image.height as u32,
				// 		depth_or_array_layers: 1,
				// 	});
				x_c = Vec::new();
				// size_new.width = ((block.x + block.width).ceil() as usize).max(size_new.width);
				// size_new.height = ((block.y + block.height).ceil() as usize).max(size_new.height);
			}
		}

		// if is_change {
		// 	sheet.update_version();
		// 	if size.width != size_new.width {
		// 		sheet.update_width(size_new.width);
		// 	}
		// 	if size.height != size_new.height {
		// 		sheet.update_height(size_new.height);
		// 	}
		// }
	}
}

// 同步绘制（异步： TODO）
fn draw_sync(list: Vec<Await>, block: Block, face: &mut Face) -> (Block, FontImage) {
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
		// 描边
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
		// 与[255, 0, 255, 255]颜色混合
		let src_a = src.a as f32 /255.0;
		let dst_a = 1.0 - src_a;
		let offset = 4 * (self.width * y as usize + x as usize);
		if offset + 4 < self.buffer.len() {
			// 一次性内存写入，TODO
			self.buffer[offset] =  (src.r as f32 * src_a + self.buffer[offset] as f32 * dst_a) as u8 ;
			self.buffer[offset + 1] = (src.g as f32 * src_a + self.buffer[offset + 1] as f32 * dst_a) as u8;
			self.buffer[offset + 2] = (src.b as f32 * src_a + self.buffer[offset + 2] as f32 * dst_a) as u8;
			if( self.buffer[offset] + self.buffer[offset + 1] )<254 {
				log::info!("{}, {}, {}", self.buffer[offset], self.buffer[offset + 1], self.buffer[offset + 2]);
			}
		}
    }

	// TODO
    fn put_shadow_pixel(&mut self, _x: i32, _y: i32, _src: Rgba) {
    }
}

pub struct Await {
	x_pos: f32,
	char: char,
}
