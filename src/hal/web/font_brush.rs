use std::mem::transmute;

use crate::createFace;
use crate::{
    ascender, computerSdf, descender,
    font::font::{Await, Block, DrawBlock, Font, FontImage, FontInfo, BASE_FONT_SIZE},
    horizontalAdvance, maxBox, maxBoxNormaliz, measureText,
};
use parry2d::{bounding_volume::Aabb, math::Point};
use pi_share::Share;
use pi_share::ThreadSync;
use pi_slotmap::{DefaultKey, SecondaryMap};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement};

use super::{
    computeLayout, computeNearArcs, computeSdfTex, computeSdfTexOfWasm, debugSize, drawChar,
    drawCharWithStroke, fillBackGround, free, getGlobalMetricsHeight, glyphIndex, loadFontSdf,
    setFont, toOutline, toOutlineOfGlyphIndex, glyphIndexs, horizontalAdvanceOfGlyphIndex
};

/// 字体绘制工具，封装了Canvas绘制上下文
/// 
/// # 字段说明
/// - `fonts`: 字体缓存集合
/// - `canvas`: HTML Canvas元素
/// - `ctx`: Canvas 2D渲染上下文
pub struct Brush {
    fonts: SecondaryMap<DefaultKey, Font>,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
}
impl Brush {
    pub fn new() -> Self {
        let window = window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let canvas = document
            .create_element("canvas")
            .expect("create canvas fail");
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("create canvas fail");
        let ctx = canvas
            .get_context("2d")
            .expect("")
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("create canvas fail");
        Brush {
            fonts: SecondaryMap::default(),
            canvas,
            ctx,
        }
    }

    pub fn check_or_create_face(&mut self, font: &FontInfo) {
        self.fonts.insert(font.font_family_id.0, font.font.clone());
    }

    pub fn base_height(&mut self, font: &FontInfo) -> f32 {
        let font = &mut self.fonts[*font.font_family_id];
        getGlobalMetricsHeight(
            unsafe { transmute(font.font_family_string.str_hash()) },
            BASE_FONT_SIZE as f32,
        ) as f32
    }

    pub fn base_width(
        &mut self,
        font: &FontInfo,
        char: char,
    ) -> (f32, usize /*fontface在数组中的索引*/) {
        let font = match self.fonts.get_mut(*font.font_family_id) {
            Some(r) => r,
            None => return (0.0, 0),
        };
        let ch_code: u32 = unsafe { transmute(char) };
        (
            measureText(
                &self.ctx,
                ch_code,
                BASE_FONT_SIZE as u32,
                unsafe { transmute(font.font_family_string.str_hash()) },
            ),
            0, /*在web上，font face索引并不重要*/
        )
    }

    pub fn draw<F: FnMut(Block, FontImage) + Clone + ThreadSync + 'static>(
        &mut self,
        draw_list: Vec<DrawBlock>,
        mut update: F,
    ) {
        for draw_block in draw_list.into_iter() {
            let font = match self.fonts.get_mut(*draw_block.font_id) {
                Some(r) => r,
                None => return,
            };

            draw_sync(
                draw_block.chars,
                &draw_block.block,
                font,
                1.0,
                &self.canvas,
                &self.ctx,
            );
            let (width, height) = (draw_block.block.width, draw_block.block.height);
            match self
                .ctx
                .get_image_data(0.0, 0.0, width as f64, height as f64)
            {
                Ok(r) => {
                    update(
                        draw_block.block,
                        FontImage {
                            buffer: (*r.data()).clone(),
                            width: width as usize,
                            height: height as usize,
                        },
                    );
                }
                Err(e) => log::error!("get_image_data fail, {:?}", e),
            }
        }
    }
}

fn draw_sync(
    list: Vec<Await>,
    block: &Block,
    font: &Font,
    stroke: f64,
    canvas: &HtmlCanvasElement,
    ctx: &CanvasRenderingContext2d,
) {
    fillBackGround(canvas, ctx, block.width as u32, block.height as u32);
    setFont(
        ctx,
        font.font_weight as u32,
        font.font_size as u32,
        unsafe { transmute(font.font_family_string.str_hash()) },
        stroke as u8,
    );
    if stroke > 0.0 {
        for await_item in list.iter() {
            let ch_code: u32 = unsafe { transmute(await_item.char) };
            let x = (await_item.x_pos + stroke as f32 / 2.0) as u32;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcEndpoint {
    pub(crate) p: [f32; 2],
    pub d: f32,

    // 线段特殊处理，只有一个值
    pub line_key: Option<u64>,

    pub(crate) line_encode: Option<[f32; 4]>, // rgba
}

#[derive(Clone)]
pub struct OutlineInfo {
    js_value: JsValue,
    pub units_per_em: u16,
    pub advance: u16,
    pub bbox: Vec<f32>,
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SdfInfo2 {
    pub tex_info: TexInfo2,
    pub sdf_tex: Vec<u8>,
    pub tex_size: u32,
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct TexInfo2 {
    pub sdf_offset_x: usize,
    pub sdf_offset_y: usize,
    pub advance: f32,
    pub char: char,
    pub plane_min_x: f32,
    pub plane_min_y: f32,
    pub plane_max_x: f32,
    pub plane_max_y: f32,
    pub atlas_min_x: f32,
    pub atlas_min_y: f32,
    pub atlas_max_x: f32,
    pub atlas_max_y: f32,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct LayoutInfo {
    pub plane_bounds: Vec<f32>,
    pub atlas_bounds: Vec<f32>,
    pub extents: Vec<f32>,
    pub distance: f32,
    pub tex_size: u32,
}

impl OutlineInfo {
    pub async fn compute_near_arcs(&self, scale: f32) -> Vec<u8> {
        let buf = computeNearArcs(self.js_value.clone(), scale).await;
        let buf = js_sys::Uint8Array::from(buf).to_vec();
        buf
    }

    pub fn compute_layout(&self, tex_size: usize, pxrange: u32, cur_off: u32) -> LayoutInfo {
        let v = computeLayout(self.js_value.clone(), tex_size, pxrange, cur_off);
        let v = js_sys::Float32Array::from(v).to_vec();
        LayoutInfo {
            plane_bounds: vec![v[0], v[1], v[2], v[3]],
            atlas_bounds: vec![v[4], v[5], v[6], v[7]],
            extents: vec![v[8], v[9], v[10], v[11]],
            distance: v[12],
            tex_size: v[13] as u32,
        }
    }

    pub fn compute_sdf_tex(
        &self,
        result_arcs: Vec<u8>,
        tex_size: usize,
        pxrange: u32,
        is_outer_glow: bool,
        cur_off: u32,
    ) -> SdfInfo2 {
        let js_value = computeSdfTexOfWasm(
            self.js_value.clone(),
            result_arcs,
            tex_size,
            pxrange,
            is_outer_glow,
            cur_off,
        );
        let bytes = js_sys::Uint8Array::from(js_value).to_vec();
        bitcode::deserialize(&bytes).unwrap()
    }
}

impl Drop for OutlineInfo {
    fn drop(&mut self) {
        free(self.js_value.clone());
    }
}

#[derive(Clone)]
pub struct FontFace(JsValue);

impl FontFace {
    pub fn new(font_data: Share<Vec<u8>>) -> Self {
        FontFace(createFace(font_data.as_ref()))
    }

    pub async fn compute_sdf(max_box: Aabb, sink: JsValue) -> Vec<u8> {
        let max_box = vec![
            max_box.mins.x,
            max_box.mins.y,
            max_box.maxs.x,
            max_box.maxs.y,
        ];
        let v = computerSdf(max_box, sink).await;
        let buf = js_sys::Uint8Array::from(v).to_vec();
        buf
    }

    pub async fn compute_sdf_tex(outline: JsValue, size: usize, pxrange: u32) -> Vec<u8> {
        let v = computeSdfTex(outline, size, pxrange).await;
        let buf = js_sys::Uint8Array::from(v).to_vec();
        buf
    }

    /// 水平宽度
    pub fn horizontal_advance(&mut self, char: char) -> f32 {
        return horizontalAdvance(self.0.clone(), char.to_string());
    }

    /// 水平宽度
    pub fn horizontal_advance_of_glyph_index(&mut self, glyph_index: u16) -> f32 {
        return horizontalAdvanceOfGlyphIndex(self.0.clone(), glyph_index);
    }

    pub fn ascender(&self) -> f32 {
        return ascender(self.0.clone());
    }

    pub fn descender(&self) -> f32 {
        return descender(self.0.clone());
    }

    pub fn max_box(&self) -> Vec<f32> {
        let v = maxBox(self.0.clone());

        let arr = js_sys::Float32Array::from(v);
        vec![
            arr.get_index(0),
            arr.get_index(1),
            arr.get_index(2),
            arr.get_index(3),
        ]
    }

    pub fn max_box_normaliz(&self) -> Aabb {
        let v = maxBoxNormaliz(self.0.clone());
        let arr = js_sys::Float32Array::from(v);
        Aabb::new(
            Point::new(arr.get_index(0), arr.get_index(1)),
            Point::new(arr.get_index(2), arr.get_index(3)),
        )
    }

    pub fn to_outline(&self, c: char) -> OutlineInfo {
        let js_value = toOutline(self.0.clone(), c.to_string());
        let bbox = js_sys::Reflect::get(&js_value, &"bbox".to_string().into()).unwrap();
        let units_per_em =
            js_sys::Reflect::get(&js_value, &"units_per_em".to_string().into()).unwrap();
        let advance = js_sys::Reflect::get(&js_value, &"advance".to_string().into()).unwrap();
        OutlineInfo {
            js_value,
            units_per_em: units_per_em.as_f64().unwrap() as u16,
            bbox: js_sys::Float32Array::from(bbox).to_vec(),
            advance: advance.as_f64().unwrap() as u16,
        }
    }

    pub fn to_outline_of_glyph_index(&self, glyph_index: u16) -> OutlineInfo {
        let js_value = toOutlineOfGlyphIndex(self.0.clone(), glyph_index);
        let bbox = js_sys::Reflect::get(&js_value, &"bbox".to_string().into()).unwrap();
        let units_per_em =
            js_sys::Reflect::get(&js_value, &"units_per_em".to_string().into()).unwrap();
        let advance = js_sys::Reflect::get(&js_value, &"advance".to_string().into()).unwrap();
        OutlineInfo {
            js_value,
            units_per_em: units_per_em.as_f64().unwrap() as u16,
            bbox: js_sys::Float32Array::from(bbox).to_vec(),
            advance: advance.as_f64().unwrap() as u16,
        }
    }

    pub fn glyph_index(&self, c: char) -> u16 {
        glyphIndex(self.0.clone(), c.to_string())
    }

    pub fn glyph_indexs(&self, text: &str, script: u32) -> Vec<u16> {
        glyphIndexs(self.0.clone(), text.to_string(), script)
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

pub async fn load_font_sdf() -> Vec<(String, Vec<SdfInfo>)> {
    let data = loadFontSdf().await;
    let data = js_sys::Uint8Array::from(data).to_vec();
    log::error!("sdf data size: {}", data.len());
    bitcode::deserialize(&data[..]).unwrap()
}
pub struct CellInfo;
// pub use pi_sdf::glyphy::blob::{TexInfo, SdfInfo};
// pub use pi_sdf::utils::{SdfInfo2, LayoutInfo};
