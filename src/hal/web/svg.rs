use super::{
    computeSvgLayout, computeSvgSdfTexOfWasm,
    font_brush::{ArcEndpoint, LayoutInfo, SdfInfo},
};
use crate::hal::web::font_brush::SdfInfo2;
use crate::{
    createCircle, createEllipse, createPath, createPolygon, createPolyline, createRect,
    createSegment, createSvgInfo, free, getSvgInfo,
};
use parry2d::bounding_volume::Aabb;
use wasm_bindgen::JsValue;
// pub use pi_sdf::shape::*;
// pub use pi_sdf::utils::SdfInfo2;

#[derive(Debug, Clone)]
pub struct SvgInfo {
    buf: Vec<u8>,
    binding_box: Vec<f32>,
}

impl SvgInfo {
    pub fn new(
        binding_box: &[f32],
        points: Vec<f32>,
        is_area: bool,
        is_reverse: Option<bool>,
    ) -> Self {
        let info = createSvgInfo(binding_box, points, is_area, is_reverse);
        let binding_box = js_sys::Reflect::get(&info, &"binding_box".to_string().into()).unwrap();
        let binding_box = js_sys::Float32Array::from(binding_box).to_vec();

        let buf = js_sys::Reflect::get(&info, &"buf".to_string().into()).unwrap();
        let buf = js_sys::Uint8Array::from(buf).to_vec();
        Self { binding_box, buf }
    }

    pub fn compute_layout(&self, tex_size: usize, pxrange: u32, cur_off: u32) -> LayoutInfo {
        let v = computeSvgLayout(&self.binding_box, tex_size, pxrange, cur_off);

        log::error!("computeLayout: {:?}", v);
        let v = js_sys::Float32Array::from(v).to_vec();
        LayoutInfo {
            plane_bounds: vec![v[0], v[1], v[2], v[3]],
            atlas_bounds: vec![v[4], v[5], v[6], v[7]],
            extents: vec![v[8], v[9], v[10], v[11]],
            distance: v[12],
            tex_size: v[13] as u32,
        }
    }

    pub async fn compute_sdf_tex(
        &self,
        tex_size: usize,
        pxrange: u32,
        is_outer_glow: bool,
        cur_off: u32,
        scale: f32,
    ) -> SdfInfo2 {
        // log:
        let js_value =
            computeSvgSdfTexOfWasm(self.buf.clone(), tex_size, pxrange, is_outer_glow, cur_off, scale).await;
        let bytes = js_sys::Uint8Array::from(js_value).to_vec();
        bitcode::deserialize(&bytes).unwrap()
    }
}

impl Drop for SvgInfo {
    fn drop(&mut self) {
        // free(self.0.clone());
    }
}

pub struct Rect(JsValue);

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self(createRect(x, y, width, height))
    }

    pub fn get_svg_info(&self) -> SvgInfo {
        let info = getSvgInfo(self.0.clone());
        let buf = js_sys::Reflect::get(&info, &"buf".to_string().into()).unwrap();
        let bbox = js_sys::Reflect::get(&info, &"binding_box".to_string().into()).unwrap();
        SvgInfo {
            buf: js_sys::Uint8Array::from(buf).to_vec(),
            binding_box: js_sys::Float32Array::from(bbox).to_vec(),
        }
    }
}

impl Drop for Rect {
    fn drop(&mut self) {
        free(self.0.clone());
    }
}

pub struct Circle(JsValue);

impl Circle {
    pub fn new(cx: f32, cy: f32, radius: f32) -> Result<Self, String> {
        Ok(Self(createCircle(cx, cy, radius)))
    }

    pub fn get_svg_info(&self) -> SvgInfo {
        let info = getSvgInfo(self.0.clone());
        let buf = js_sys::Reflect::get(&info, &"buf".to_string().into()).unwrap();
        let bbox = js_sys::Reflect::get(&info, &"binding_box".to_string().into()).unwrap();
        SvgInfo {
            buf: js_sys::Uint8Array::from(buf).to_vec(),
            binding_box: js_sys::Float32Array::from(bbox).to_vec(),
        }
    }
}

impl Drop for Circle {
    fn drop(&mut self) {
        free(self.0.clone());
    }
}

pub struct Ellipse(JsValue);

impl Ellipse {
    pub fn new(cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        Self(createEllipse(cx, cy, rx, ry))
    }

    pub fn get_svg_info(&self) -> SvgInfo {
        let info = getSvgInfo(self.0.clone());
        let buf = js_sys::Reflect::get(&info, &"buf".to_string().into()).unwrap();
        let bbox = js_sys::Reflect::get(&info, &"binding_box".to_string().into()).unwrap();
        SvgInfo {
            buf: js_sys::Uint8Array::from(buf).to_vec(),
            binding_box: js_sys::Float32Array::from(bbox).to_vec(),
        }
    }
}

impl Drop for Ellipse {
    fn drop(&mut self) {
        free(self.0.clone());
    }
}

pub struct Segment(JsValue);

impl Segment {
    pub fn new(ax: f32, ay: f32, bx: f32, by: f32) -> Self {
        Self(createSegment(ax, ay, bx, by))
    }

    pub fn get_svg_info(&self) -> SvgInfo {
        let info = getSvgInfo(self.0.clone());
        let buf = js_sys::Reflect::get(&info, &"buf".to_string().into()).unwrap();
        let bbox = js_sys::Reflect::get(&info, &"binding_box".to_string().into()).unwrap();
        SvgInfo {
            buf: js_sys::Uint8Array::from(buf).to_vec(),
            binding_box: js_sys::Float32Array::from(bbox).to_vec(),
        }
    }
}

impl Drop for Segment {
    fn drop(&mut self) {
        free(self.0.clone());
    }
}

pub struct Polygon(JsValue);

impl Polygon {
    pub fn new(points: Vec<f32>) -> Self {
        Self(createPolygon(points))
    }

    pub fn get_svg_info(&self) -> SvgInfo {
        let info = getSvgInfo(self.0.clone());
        let buf = js_sys::Reflect::get(&info, &"buf".to_string().into()).unwrap();
        let bbox = js_sys::Reflect::get(&info, &"binding_box".to_string().into()).unwrap();
        SvgInfo {
            buf: js_sys::Uint8Array::from(buf).to_vec(),
            binding_box: js_sys::Float32Array::from(bbox).to_vec(),
        }
    }
}

impl Drop for Polygon {
    fn drop(&mut self) {
        free(self.0.clone());
    }
}

pub struct Polyline(JsValue);

impl Polyline {
    pub fn new(points: Vec<f32>) -> Self {
        Self(createPolyline(points))
    }

    pub fn get_svg_info(&self) -> SvgInfo {
        let info = getSvgInfo(self.0.clone());
        let buf = js_sys::Reflect::get(&info, &"buf".to_string().into()).unwrap();
        let bbox = js_sys::Reflect::get(&info, &"binding_box".to_string().into()).unwrap();
        SvgInfo {
            buf: js_sys::Uint8Array::from(buf).to_vec(),
            binding_box: js_sys::Float32Array::from(bbox).to_vec(),
        }
    }
}

impl Drop for Polyline {
    fn drop(&mut self) {
        free(self.0.clone());
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PathVerb {
    // 绝对点
    MoveTo = 1,
    // 相对点
    MoveToRelative = 2,
    LineTo = 3,
    LineToRelative = 4,
    QuadTo = 5,
    QuadToRelative = 6,
    SmoothQuadTo = 7,
    SmoothQuadToRelative = 8,
    CubicTo = 9,
    CubicToRelative = 10,
    SmoothCubicTo = 11,
    SmoothCubicToRelative = 12,
    HorizontalLineTo = 13,
    HorizontalLineToRelative = 14,
    VerticalLineTo = 15,
    VerticalLineToRelative = 16,
    EllipticalArcTo = 17,
    EllipticalArcToRelative = 18,
    Close = 19,
}

pub struct Path(JsValue);

impl Path {
    pub fn new(verb: Vec<u8>, points: Vec<f32>) -> Self {
        Self(createPath(verb, points))
    }

    pub fn new1(verb: Vec<PathVerb>, points: Vec<f32>) -> Self {
        let verb: Vec<u8> = unsafe { core::mem::transmute(verb) };
        Self(createPath(verb, points))
    }

    pub fn get_svg_info(&self) -> SvgInfo {
        let info = getSvgInfo(self.0.clone());
        let buf = js_sys::Reflect::get(&info, &"buf".to_string().into()).unwrap();
        let bbox = js_sys::Reflect::get(&info, &"binding_box".to_string().into()).unwrap();
        SvgInfo {
            buf: js_sys::Uint8Array::from(buf).to_vec(),
            binding_box: js_sys::Float32Array::from(bbox).to_vec(),
        }
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        free(self.0.clone());
    }
}

// pub fn computer_svg_sdf(info: SvgInfo) -> SdfInfo {
//     let v = computerSvgSdf(info.0.clone());
//     let buf = js_sys::Uint8Array::from(v).to_vec();

//     let sdf_info: SdfInfo = bitcode::deserialize(&buf[..]).unwrap();
//     sdf_info
// }

// pub fn compute_shape_sdf_tex(
//     info: SvgInfo,
//     tex_size: usize,
//     pxrange: u32,
//     is_outer_glow: bool,
//     cur_off: u32,
// ) -> SdfInfo2 {
//     let v = computeShapeSdfTex(info.0.clone(), size, pxrange, is_outer_glow, cur_off);
//     let buf = js_sys::Uint8Array::from(v).to_vec();

//     let sdf_info: SdfInfo2 = bitcode::deserialize(&buf[..]).unwrap();
//     sdf_info
// }

// pub fn createCircle(cx: f32, cy: f32, radius: f32) -> JsValue;
//     pub fn createRect(x: f32, y: f32, width: f32, height: f32) -> JsValue;
//     pub fn createSegment(ax: f32, ay: f32, bx: f32, by: f32) -> JsValue;
//     pub fn createEllipse(cx: f32, cy: f32, rx: f32, ry: f32) -> JsValue;
//     pub fn createPolygon(points: Vec<f32>) -> JsValue;
//     pub fn createPolyline(points: Vec<f32>) -> JsValue;
//     pub fn createPath(verb: Vec<u8>, points: Vec<f32>) -> JsValue;
//     pub fn getSvgInfo(shape: JsValue) -> JsValue;
//     pub fn computerSvgSdf(svg_info: JsValue) -> JsValue;
//     pub fn free(obj: JsValue) -> JsValue;

// pub use pi_sdf::utils::SdfInfo2;
