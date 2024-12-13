use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d};
use js_sys::Function;

pub mod image;
pub mod font_brush;
pub mod runtime;
pub mod file;
pub mod stroe;
pub mod svg;
#[cfg(feature="web_local_load")]
pub mod web_local;
pub mod image_texture_load;
pub mod sdf2_info;

#[wasm_bindgen(module = "/js/utils.js")]
extern "C" {
    // #[wasm_bindgen]
    fn fillBackGround(canvas: &HtmlCanvasElement, ctx: &CanvasRenderingContext2d, x: u32, y: u32);
    // #[wasm_bindgen]
    fn setFont(ctx: &CanvasRenderingContext2d, weight: u32, fontSize: u32, font: f64, strokeWidth: u8);
    // #[wasm_bindgen]
    fn drawCharWithStroke(ctx: &CanvasRenderingContext2d, ch_code: u32, x: u32, y: u32);
	fn getGlobalMetricsHeight(name: f64, size: f32) -> f32;
    // #[wasm_bindgen]
    fn drawChar(ctx: &CanvasRenderingContext2d, ch_code: u32, x: u32, y: u32);
    // #[wasm_bindgen]
    pub fn measureText(ctx: &CanvasRenderingContext2d, ch: u32, font_size: u32, name: f64) -> f32;
    // #[wasm_bindgen]
	#[wasm_bindgen(catch)]
	pub async fn loadFile(image_name: f64) -> Result<JsValue, JsValue>;
	// 加载图片作文canvas
	#[wasm_bindgen(catch)]
	pub async fn loadImageAsCanvas(image_name: f64) -> Result<JsValue, JsValue>;
    // 加载ktx文件
    #[wasm_bindgen(catch)]
	pub async fn loadKtx(image_name: f64) -> Result<JsValue, JsValue>;
    // 加载图片作文canvas
	#[wasm_bindgen(catch)]
	pub async fn loadImage(image_name: f64) -> Result<JsValue, JsValue>;
    // #[wasm_bindgen]
    pub fn useVao() -> bool;
    pub fn hasAtom(key: f64) -> bool;
    pub fn setAtom(key: f64, v: String);
    pub fn int(key: f64, v: String);

    pub async fn initLocalStore();
    /**
     * 从indexDb读数据
     */
    // tslint:disable-next-line:no-reserved-keywords
    #[wasm_bindgen(catch)]
    pub async fn get (key: String) -> Result<JsValue, JsValue>;
    
    /**
     * 往indexDb写数据
     */
    pub async fn write (key: String, data: Vec<u8>);
    
    /**
     * 从indexDb删除数据
     */
    pub async fn deleteKey(key: String);

    pub fn createFace(data: &[u8]) -> JsValue;
    pub async fn computerSdf(max_box: Vec<f32>, outline: JsValue) -> JsValue;
    pub async fn computeSdfTex(outline: JsValue, size: usize, pxrange: u32) -> JsValue;
    pub fn horizontalAdvance(face: JsValue, text: String) -> f32;
    pub fn ascender(face: JsValue) -> f32;
    pub fn descender(face: JsValue) -> f32;
    pub fn maxBox(face: JsValue)-> JsValue;
    pub fn maxBoxNormaliz(face: JsValue)-> JsValue;
    pub fn toOutline(face: JsValue, text: String) ->JsValue;
    // pub fn toOutline3(face: JsValue, text: String) ->JsValue;
    pub fn glyphIndex(face: JsValue, text: String) -> u16;
    pub fn debugSize(face: JsValue) -> usize;

    pub fn createCircle(cx: f32, cy: f32, radius: f32) -> JsValue;
    pub fn createRect(x: f32, y: f32, width: f32, height: f32) -> JsValue;
    pub fn createSegment(ax: f32, ay: f32, bx: f32, by: f32) -> JsValue;
    pub fn createEllipse(cx: f32, cy: f32, rx: f32, ry: f32) -> JsValue;
    pub fn createPolygon(points: Vec<f32>) -> JsValue;
    pub fn createPolyline(points: Vec<f32>) -> JsValue;
    pub fn createPath(verb: Vec<u8>, points: Vec<f32>) -> JsValue;
    pub fn getSvgInfo(shape: JsValue) -> JsValue;
    // pub fn computerSvgSdf(svg_info: JsValue) -> JsValue;
    // pub fn computeShapeSdfTex(info: JsValue, size: usize, pxrange: u32, is_outer_glow: bool, cur_off: u32) -> JsValue;
    // pub fn computeArcsSdfTex(info: JsValue, size: usize, pxrange: u32) -> JsValue;
    pub async fn computeNearArcs(info: JsValue, scale: f32) -> JsValue;
    pub fn createSvgInfo(bbox: &[f32], arc_endpoints: Vec<f32>, is_area: bool, is_reverse: Option<bool>,) -> JsValue;
    pub fn free(obj: JsValue) -> JsValue;
    pub async fn loadFontSdf() -> JsValue;

    pub fn computeLayout(info: JsValue, size: usize, pxrange: u32,  cur_off: u32) -> JsValue;
    pub fn computeSvgLayout(bbox: &[f32], size: usize, pxrange: u32,  cur_off: u32) -> JsValue;
    pub fn computeSdfTexOfWasm(info: JsValue, result_arcs: Vec<u8>,tex_size: usize,pxrange: u32, is_outer_glow: bool, cur_off: u32, ) ->JsValue;
    pub async fn computeSvgSdfTexOfWasm(info: Vec<u8>, tex_size: usize,pxrange: u32, is_outer_glow: bool, cur_off: u32, scale: f32) -> JsValue;
    
}

#[cfg(feature="web_local_load")]
pub use web_local::{init_load_cb, on_load};
