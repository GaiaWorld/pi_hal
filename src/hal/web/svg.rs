use super::{
    computeLayout, computeSvgSdfTexOfWasm, font_brush::{ArcEndpoint, LayoutInfo, SdfInfo, SdfInfo2}
};
use crate::{
     createCircle, createEllipse, createPath,
    createPolygon, createPolyline, createRect, createSegment, createSvgInfo, free, getSvgInfo,
};
use parry2d::bounding_volume::Aabb;
use wasm_bindgen::JsValue;

pub struct SvgInfo(JsValue);

impl SvgInfo {
    pub fn new(binding_box: Aabb, arc_endpoints: Vec<ArcEndpoint>) -> Self {
        let arc_endpoints = bitcode::serialize(&arc_endpoints).unwrap();
        let binding_box = vec![
            binding_box.mins.x,
            binding_box.mins.y,
            binding_box.maxs.x,
            binding_box.maxs.x,
        ];
        Self(createSvgInfo(binding_box, arc_endpoints))
    }

    pub fn compute_layout(&self, tex_size: usize, pxrange: u32, cur_off: u32) -> LayoutInfo {
        let v = computeLayout(self.0.clone(), tex_size, pxrange, cur_off);
        let plane_bounds = js_sys::Reflect::get(&v, &"plane_bounds".to_string().into()).unwrap();
        let atlas_bounds = js_sys::Reflect::get(&v, &"atlas_bounds".to_string().into()).unwrap();
        let extents = js_sys::Reflect::get(&v, &"extents".to_string().into()).unwrap();
        let distance = js_sys::Reflect::get(&v, &"distance".to_string().into()).unwrap();
        let tex_size = js_sys::Reflect::get(&v, &"tex_size".to_string().into()).unwrap();

        LayoutInfo {
            plane_bounds: js_sys::Float32Array::from(plane_bounds).to_vec(),
            atlas_bounds: js_sys::Float32Array::from(atlas_bounds).to_vec(),
            extents: js_sys::Float32Array::from(extents).to_vec(),
            distance: distance.as_f64().unwrap() as f32,
            tex_size: tex_size.as_f64().unwrap() as u32,
        }
    }

    pub async fn compute_sdf_tex(
        &self,
        tex_size: usize,
        pxrange: u32,
        is_outer_glow: bool,
        cur_off: u32,
        scale: f32,
    )-> SdfInfo2{
        let js_value = computeSvgSdfTexOfWasm(self.0.clone(), tex_size, pxrange, is_outer_glow, cur_off).await;
        let bytes = js_sys::Uint8Array::from(js_value).to_vec();
        bitcode::deserialize(&bytes).unwrap()
    }
}

impl Drop for SvgInfo {
    fn drop(&mut self) {
        free(self.0.clone());
    }
}

pub struct Rect(JsValue);

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self(createRect(x, y, width, height))
    }

    pub fn get_svg_info(&self) -> SvgInfo {
        SvgInfo(getSvgInfo(self.0.clone()))
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
        SvgInfo(getSvgInfo(self.0.clone()))
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
        SvgInfo(getSvgInfo(self.0.clone()))
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
        SvgInfo(getSvgInfo(self.0.clone()))
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
        SvgInfo(getSvgInfo(self.0.clone()))
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
        SvgInfo(getSvgInfo(self.0.clone()))
    }
}

impl Drop for Polyline {
    fn drop(&mut self) {
        free(self.0.clone());
    }
}

pub struct Path(JsValue);

impl Path {
    pub fn new(verb: Vec<u8>, points: Vec<f32>) -> Self {
        Self(createPath(verb, points))
    }

    pub fn get_svg_info(&self) -> SvgInfo {
        SvgInfo(getSvgInfo(self.0.clone()))
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
