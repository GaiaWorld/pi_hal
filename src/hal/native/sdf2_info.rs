use pi_sdf::{shape::SvgInfo, utils::{LayoutInfo, SdfInfo2}};

// pub fn compute_layout(sdf_info: &SvgInfo, tex_size: usize, pxrange: u32, cur_off: u32) -> LayoutInfo {
//     sdf_info.compute_layout(tex_size, pxrange, cur_off)
// }

pub async fn compute_sdf_tex(
    sdf_info: &SvgInfo,
    tex_size: usize,
    pxrange: u32,
    is_outer_glow: bool,
    cur_off: u32,
    scale: f32,
)->  SdfInfo2 {
    sdf_info.compute_sdf_tex(tex_size, pxrange, is_outer_glow, cur_off, scale)
}