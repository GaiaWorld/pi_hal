use pi_sdf::{shape::SvgInfo, utils::{LayoutInfo, SdfInfo2}};
use wasm_bindgen::JsValue;
use super::computeSvgSdfTexOfWasm;

// pub fn compute_layout(sdf_info: &SvgInfo, tex_size: usize, pxrange: u32, cur_off: u32) -> LayoutInfo {
//     let sdf_info = match JsValue::from_serde(sdf_info) {
//         Ok(r) => r,
//         Err(_e) => {
//             log::info!("serde sdf_info fail");
//             panic!();
//         }
//     };
//     let v = computeLayout(sdf_info, tex_size, pxrange, cur_off);
//     let plane_bounds = js_sys::Reflect::get(&v, &"plane_bounds".to_string().into()).unwrap();
//     let atlas_bounds = js_sys::Reflect::get(&v, &"atlas_bounds".to_string().into()).unwrap();
//     let extents = js_sys::Reflect::get(&v, &"extents".to_string().into()).unwrap();
//     let distance = js_sys::Reflect::get(&v, &"distance".to_string().into()).unwrap();
//     let tex_size = js_sys::Reflect::get(&v, &"tex_size".to_string().into()).unwrap();

//     LayoutInfo {
//         plane_bounds: js_sys::Float32Array::from(plane_bounds).to_vec(),
//         atlas_bounds: js_sys::Float32Array::from(atlas_bounds).to_vec(),
//         extents: js_sys::Float32Array::from(extents).to_vec(),
//         distance: distance.as_f64().unwrap() as f32,
//         tex_size: tex_size.as_f64().unwrap() as u32,
//     }
// }

pub async fn compute_sdf_tex(
    sdf_info: &SvgInfo,
    tex_size: usize,
    pxrange: u32,
    is_outer_glow: bool,
    cur_off: u32,
    scale: f32,
)->  SdfInfo2{
    let sdf_info = match JsValue::from_serde(sdf_info) {
        Ok(r) => r,
        Err(_e) => {
            log::info!("serde sdf_info fail");
            panic!();
        }
    };
    let js_value = computeSvgSdfTexOfWasm(sdf_info, tex_size, pxrange, is_outer_glow, cur_off).await;
    let bytes = js_sys::Uint8Array::from(js_value).to_vec();
    bitcode::deserialize(&bytes).unwrap()
}