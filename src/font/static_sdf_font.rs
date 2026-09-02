use pi_hash::XHashMap;
use pi_slotmap::{DefaultKey, SecondaryMap, SlotMap};

use super::{
    font::{FontFaceId, FontId, FontInfo, Glyph, GlyphId, GlyphIdDesc},
    sdf_table::{FontCfg, MetricsInfo},
    text_pack::TextPacker,
};
use crate::font_brush::SdfInfo2;
#[cfg(target_arch = "wasm32")]
use crate::font_brush::TexInfo2;
#[cfg(not(target_arch = "wasm32"))]
use pi_sdf::utils::TexInfo2;

pub fn glyph_id(
    static_fonts: &SecondaryMap<DefaultKey, FontCfg>,
    metrics: &SecondaryMap<DefaultKey, MetricsInfo>,
    glyph_id_map: &mut XHashMap<(FontFaceId, u32), GlyphId>,
    glyphs: &mut SlotMap<DefaultKey, GlyphIdDesc>,
    index_packer: &mut TextPacker,
    static_glyph_faces: &mut XHashMap<GlyphId, FontFaceId>,
    font_id: FontId,
    font_info: &mut FontInfo,
    font_face_id: FontFaceId,
    char: char,
) -> Option<GlyphId> {
    let static_font = match static_fonts.get(font_face_id.0) {
        Some(value) => value,
        None => {
            log::warn!("static SDF font missing: char={}, face={:?}", char, font_face_id);
            return None;
        }
    };
    let glyph_info = match static_font.glyphs.get(&char) {
        Some(value) => value.clone(),
        None => {
            log::warn!("static SDF glyph missing from config: font={}, char={}, codepoint={}", static_font.name, char, char as u32);
            return None;
        }
    };
    let metrics = match metrics.get(font_face_id.0) {
        Some(value) => value.clone(),
        None => {
            log::warn!("static SDF metrics missing: font={}, char={}", static_font.name, char);
            return None;
        }
    };
    match glyph_id_map.entry((font_face_id, char as u32)) {
        std::collections::hash_map::Entry::Occupied(entry) => {
            Some(*entry.get())
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            let tex_size = usize::from(glyph_info.width.max(glyph_info.height));
            let offset = match index_packer.alloc(tex_size, tex_size) {
                Some(value) => value,
                None => {
                    log::warn!("static SDF atlas allocation failed: font={}, char={}, size={}x{}", static_font.name, char, tex_size, tex_size);
                    return None;
                }
            };
            let font_size = metrics.font_size.max(1.0);
            let plane_min_x = glyph_info.ox as f32 / super::font::OFFSET_RANGE;
            let top_offset = glyph_info.oy as f32 / super::font::OFFSET_RANGE;
            let plane_max_y = metrics.ascender - top_offset;
            let plane_min_y = plane_max_y - glyph_info.height as f32 / font_size;
            let glyph = Glyph {
                plane_min_x,
                plane_min_y,
                plane_max_x: plane_min_x + glyph_info.width as f32 / font_size,
                plane_max_y,
                x: offset.x as f32,
                y: offset.y as f32,
                width: glyph_info.width as f32,
                height: glyph_info.height as f32,
                advance: glyph_info.advance as f32 / font_size,
            };
            let glyph_id = GlyphId(glyphs.insert(GlyphIdDesc {
                font_id,
                char,
                glyph_index: char as u32,
                font_face_index: font_info.font_ids.iter().position(|value| *value == font_face_id).unwrap_or(0),
                glyph,
            }));
            if !char.is_whitespace() {
                font_info.await_info.wait_list.push(glyph_id);
            }
            entry.insert(glyph_id);
            static_glyph_faces.insert(glyph_id, font_face_id);
            Some(glyph_id)
        }
    }
}

pub fn sdf_info(glyph: Glyph, char: char, data: Vec<u8>) -> SdfInfo2 {
    let size = glyph.width.max(glyph.height) as usize;
    let width = glyph.width as usize;
    let height = glyph.height as usize;
    let mut sdf_tex = vec![0; size * size];
    if data.len() < width * height {
        log::warn!("static SDF glyph data too short: char={}, expected={}, actual={}", char, width * height, data.len());
    }
    for row in 0..height {
        let source_start = row * width;
        let source_end = source_start + width;
        if source_end > data.len() {
            break;
        }
        let target_start = row * size;
        sdf_tex[target_start..target_start + width].copy_from_slice(&data[source_start..source_end]);
    }
    SdfInfo2 {
        tex_info: TexInfo2 {
            char,
            advance: glyph.advance,
            plane_min_x: glyph.plane_min_x,
            plane_min_y: glyph.plane_min_y,
            plane_max_x: glyph.plane_max_x,
            plane_max_y: glyph.plane_max_y,
            atlas_min_x: 0.0,
            atlas_min_y: 0.0,
            atlas_max_x: glyph.width,
            atlas_max_y: glyph.height,
            ..Default::default()
        },
        sdf_tex,
        tex_size: size as u32,
    }
}
