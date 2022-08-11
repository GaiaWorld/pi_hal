use pi_slotmap::{SecondaryMap, DefaultKey};
use crate::font::font::{FontId, Font, FontImage, Block, Await, DrawBlock};

pub struct Brush;

impl Brush {
	pub fn new() -> Self {
		Brush
	}

	pub fn check_or_create_face(& mut self, font_id: FontId, font: &Font) {
	
		
	}

	pub fn height(&mut self, font_id: FontId) -> f32 {
		
		0.0
	}

    pub fn width(&mut self, font_id: FontId, char: char) -> f32 {
		0.0
    }

    pub fn draw<F: FnMut(Block, FontImage) + Clone + Send + Sync + 'static>(
		&mut self, 
		draw_list: Vec<DrawBlock>,
		mut update: F) {
		
	}
}


// impl FontImage {
// 	fn init_background(&mut self) {
// 		let mut i = 0;
// 		let len = self.buffer.len();
// 		while i < len{
// 			self.buffer[i] = 255;
// 			self.buffer[i + 1] = 0;
// 			self.buffer[i + 2] = 255;
// 			self.buffer[i + 3] = 255;
// 			i += 4;
// 		}
// 	}
// }

// impl WritePixel for FontImage {
//     fn put_font_pixel(&mut self, x: i32, y: i32, src: Rgba) {
// 		if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
// 			return;
// 		}
// 		// 与[255, 0, 255, 255]颜色混合
// 		let src_a = src.a as f32 /255.0;
// 		let dst_a = 1.0 - src_a;
// 		let offset = 4 * (self.width * y as usize + x as usize);
// 		if offset + 4 < self.buffer.len() {
// 			// 一次性内存写入，TODO bgra
// 			self.buffer[offset] =  (src.b as f32 * src_a + self.buffer[offset] as f32 * dst_a) as u8 ;
// 			self.buffer[offset + 1] = (src.g as f32 * src_a + self.buffer[offset + 1] as f32 * dst_a) as u8;
// 			self.buffer[offset + 2] = (src.r as f32 * src_a + self.buffer[offset + 2] as f32 * dst_a) as u8;

// 			// let b =  (src.b as f32 * src_a + self.buffer[offset] as f32 * dst_a) as u8 ;
// 			// let g = (src.g as f32 * src_a + self.buffer[offset + 1] as f32 * dst_a) as u8;
// 			// let r = (src.r as f32 * src_a + self.buffer[offset + 2] as f32 * dst_a) as u8;
// 			// if( self.buffer[offset + 1] + self.buffer[offset + 2] )<250 {
// 			// 	log::warn!("{}, {}, {}", self.buffer[offset], self.buffer[offset + 1], self.buffer[offset + 2]);
// 			// }
// 		}
//     }

// 	// TODO
//     fn put_shadow_pixel(&mut self, _x: i32, _y: i32, _src: Rgba) {
//     }
// }

