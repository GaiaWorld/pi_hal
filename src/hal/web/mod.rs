use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d};
use js_sys::Function;

pub mod image;
pub mod font_brush;
pub mod runtime;

#[wasm_bindgen(module = "/js/utils.js")]
extern "C" {
    // #[wasm_bindgen]
    fn fillBackGround(canvas: &HtmlCanvasElement, ctx: &CanvasRenderingContext2d, x: u32, y: u32);
    // #[wasm_bindgen]
    fn setFont(ctx: &CanvasRenderingContext2d, weight: u32, fontSize: u32, font: u32, strokeWidth: u8);
    // #[wasm_bindgen]
    fn drawCharWithStroke(ctx: &CanvasRenderingContext2d, ch_code: u32, x: u32, y: u32);
	fn getGlobalMetricsHeight(name: u32, size: f32) -> f32;
    // #[wasm_bindgen]
    fn drawChar(ctx: &CanvasRenderingContext2d, ch_code: u32, x: u32, y: u32);
    // #[wasm_bindgen]
    pub fn measureText(ctx: &CanvasRenderingContext2d, ch: u32, font_size: u32, name: u32) -> f32;
    // #[wasm_bindgen]
    pub fn loadImage(image_name: u32, callback: &Function);
	#[wasm_bindgen(catch)]
	pub async fn loadFile(image_name: u32) -> Result<JsValue, JsValue>;
	// 加载图片作文canvas
	#[wasm_bindgen(catch)]
	pub async fn loadImageAsCanvas(image_name: u32) -> Result<JsValue, JsValue>;
    // #[wasm_bindgen]
    pub fn useVao() -> bool;
}