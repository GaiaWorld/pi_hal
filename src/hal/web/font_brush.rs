use std::mem::transmute;


use parry2d::{bounding_volume::Aabb, math::Point};
use pi_slotmap::{SecondaryMap, DefaultKey};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use crate::{ascender, computerSdf, descender, font::font::{Await, Block, DrawBlock, Font, FontFamilyId, FontImage, FontInfo, BASE_FONT_SIZE}, horizontalAdvance, maxBox, maxBoxNormaliz, measureText};
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement};
use pi_share::ThreadSync;
use crate::createFace;

use super::{fillBackGround, setFont, drawCharWithStroke, drawChar, getGlobalMetricsHeight, toOutline, debugSize, loadFontSdf, free, glyphIndex};

pub struct Brush {
	fonts: SecondaryMap<DefaultKey, Font>,
	canvas: HtmlCanvasElement,
	ctx: CanvasRenderingContext2d,
}
impl Brush {
	pub fn new() -> Self {
		let window = window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let canvas = document.create_element("canvas").expect("create canvas fail");
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>().expect("create canvas fail");
        let ctx = canvas
            .get_context("2d")
            .expect("")
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("create canvas fail");
		Brush {
			fonts: SecondaryMap::default(),
			canvas,
			ctx
		}
	}

	pub fn check_or_create_face(&mut self, font: &FontInfo) {
		self.fonts.insert(font.font_family_id.0, font.font.clone());
	}

	pub fn base_height(&mut self, font: &FontInfo) -> f32 {
		let font = &mut self.fonts[*font.font_family_id];
		getGlobalMetricsHeight(font.font_family_string.str_hash() as u32, BASE_FONT_SIZE as f32) as f32
	}

    pub fn base_width(&mut self, font: &FontInfo, char: char) -> (f32, usize/*fontface在数组中的索引*/) {
		let font = match self.fonts.get_mut(*font.font_family_id) {
			Some(r) => r,
			None => return (0.0, 0),
		};
		let ch_code: u32 = unsafe { transmute(char) };
		(measureText(&self.ctx, ch_code, BASE_FONT_SIZE as u32, font.font_family_string.str_hash() as u32), 0/*在web上，font face索引并不重要*/)
    }

    pub fn draw<F: FnMut(Block, FontImage) + Clone + ThreadSync + 'static>(
		&mut self, 
		draw_list: Vec<DrawBlock>,
		mut update: F) {
		
		for draw_block in draw_list.into_iter() {
			let font = match self.fonts.get_mut(*draw_block.font_id) {
				Some(r) => r,
				None => return ,
			};

			draw_sync(
				draw_block.chars, 
				&draw_block.block,
				font,
				*draw_block.font_stroke as f64,
				&self.canvas,
				&self.ctx
			);
			let (width, height) = (draw_block.block.width, draw_block.block.height);
			match self.ctx.get_image_data(0.0, 0.0, width as f64, height as f64) {
				Ok(r) => {
					update(draw_block.block, FontImage {buffer: (*r.data()).clone(), width: width as usize, height: height as usize});
				},
				Err(e) => log::error!("get_image_data fail, {:?}", e),
			}
		}
	}
}

fn draw_sync(list: Vec<Await>, block: &Block, font: &Font, stroke: f64, canvas: &HtmlCanvasElement, ctx: &CanvasRenderingContext2d) {
	fillBackGround(canvas, ctx, block.width as u32, block.height as u32);
	setFont(
		ctx,
		font.font_weight as u32,
		font.font_size as u32,
		font.font_family_string.str_hash() as u32,
		stroke as u8,
	);
	if stroke > 0.0 {
		for await_item in list.iter() {
			let ch_code: u32 = unsafe { transmute(await_item.char) };
			let x = (await_item.x_pos + stroke as f32/2.0) as u32;
			//fillText 和 strokeText 的顺序对最终效果会有影响， 为了与css text-stroke保持一致， 应该fillText在前
			drawCharWithStroke(ctx, ch_code, x, 0);
		}
	} else {
		for await_item in list.iter() {
			let ch_code: u32 = unsafe { transmute(await_item.char) };
			drawChar(ctx, ch_code, await_item.x_pos as u32, 0);
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TexInfo {
    pub grid_w: f32,
    pub grid_h: f32,

    pub cell_size: f32,

    pub max_offset: usize,
    pub min_sdf: f32,
    pub sdf_step: f32,
    
    pub index_offset_x: usize,
    pub index_offset_y: usize,
    pub data_offset_x: usize,
    pub data_offset_y: usize,
    pub char: char,
    pub extents_min_x: f32,
    pub extents_min_y: f32,
    pub extents_max_x: f32,
    pub extents_max_y: f32, 
    pub binding_box_min_x: f32,
    pub binding_box_min_y: f32,
    pub binding_box_max_x: f32,
    pub binding_box_max_y: f32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SdfInfo {
    pub tex_info: TexInfo,
    pub data_tex: Vec<u8>,
    pub index_tex: Vec<u8>,
    pub sdf_tex1: Vec<u8>,
    pub sdf_tex2: Vec<u8>,
    pub sdf_tex3: Vec<u8>,
    pub sdf_tex4: Vec<u8>,
    pub grid_size: Vec<f32>,
}

#[derive(Clone)]
pub struct FontFace(JsValue);

impl FontFace{
	pub fn new(font_data: Vec<u8>) -> Self{
		FontFace(createFace(font_data))
	}

	pub async fn compute_sdf(max_box: Aabb, sink: JsValue) -> Vec<u8> {
		let max_box = vec![max_box.mins.x, max_box.mins.y, max_box.maxs.x, max_box.maxs.y];
		let v = computerSdf(max_box, sink).await;
		let buf = js_sys::Uint8Array::from(v).to_vec();
		buf
    }

    /// 水平宽度
    pub fn horizontal_advance(&mut self, char: char) -> f32 {
        return horizontalAdvance(self.0.clone(), char.to_string())
    }

    pub fn ascender(&self) -> f32 {
        return ascender(self.0.clone())
    }

    pub fn descender(&self) -> f32 {
		return descender(self.0.clone())
    }

    pub fn max_box(&self) -> Aabb {
		let v= maxBox(self.0.clone());

		let arr = js_sys::Float32Array::from(v);
        Aabb::new(Point::new(arr.get_index(0), arr.get_index(1)), Point::new(arr.get_index(2), arr.get_index(3)))
    }

    pub fn max_box_normaliz(&self) -> Aabb {
		let v= maxBoxNormaliz(self.0.clone());
		let arr = js_sys::Float32Array::from(v);
        Aabb::new(Point::new(arr.get_index(0), arr.get_index(1)), Point::new(arr.get_index(2), arr.get_index(3)))
    }

	pub fn to_outline(&self, c: char) -> JsValue {
		toOutline(self.0.clone(), c.to_string())
    }

	pub fn glyph_index(&self, c: char) -> u16 {
		glyphIndex(self.0.clone(), c.to_string())
    }

	pub fn debug_size(&self) -> usize {
		debugSize(self.0.clone())
    }
}

impl Drop for FontFace {
    fn drop(&mut self) {
        free(self.0.clone());
    }
}

pub async fn load_font_sdf() -> Vec<(String, Vec<SdfInfo>)>{
	let data = loadFontSdf().await;
	let data = js_sys::Uint8Array::from(data).to_vec();
	log::error!("sdf data size: {}", data.len());
	bincode::deserialize(&data[..]).unwrap()
}
