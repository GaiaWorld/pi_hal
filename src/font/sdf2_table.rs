use crate::font_brush::CellInfo;
use crate::font_brush::SdfAabb;
use crate::font_brush::SdfArc;
use ordered_float::NotNan;
use parry2d::{bounding_volume::Aabb, math::Point};
use pi_async_rt::prelude::AsyncValueNonBlocking as AsyncValue;
use pi_atom::Atom;
use pi_hash::XHashMap;
use pi_null::Null;
use pi_sdf::utils::compute_layout;
use pi_sdf::utils::OutlineInfo;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
/// 用圆弧曲线模拟字符轮廓， 并用于计算距离值的方案
use std::{
    cell::OnceCell,
    collections::{hash_map::Entry, HashMap},
    hash::{DefaultHasher, Hash, Hasher},
    mem::transmute,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
        Arc, Mutex, OnceLock,
    },
};

#[derive(Default, Debug)]
pub struct SdfResultInner {
    pub font_result: Vec<(DefaultKey, SdfInfo2, SdfType)>,
    pub svg_result: Vec<(u64, SdfInfo2, SdfType)>,
    pub box_result: Vec<(u64, BoxInfo, Vec<u8>)>,
}

#[derive(Debug, Default, Clone)]
pub struct SdfResult(pub Arc<ShareMutex<SdfResultInner>>);
// use pi_sdf::utils::GlyphInfo;
// use pi_sdf::shape::ArcOutline;
use crate::font_brush::TexInfo2;
use pi_share::{Share, ShareMutex};
use pi_slotmap::{DefaultKey, SecondaryMap, SlotMap};

use super::blur::compute_box_layout;
use super::blur::BoxInfo;
use super::{
    blur::{blur_box, gaussian_blur},
    font::{
        Block, FontFaceId, FontFamilyId, FontId, FontImage, FontInfo, Glyph, GlyphId, GlyphIdDesc,
        Size,
    },
    sdf_table::MetricsInfo,
    text_pack::TextPacker,
};

pub use crate::font_brush::TexInfo;
use crate::{
    font_brush::{load_font_sdf, FontFace, SdfInfo2},
    runtime::MULTI_MEDIA_RUNTIME,
    stroe::{self, init_local_store},
    svg::{compute_shape_sdf_tex, SvgInfo},
};
use pi_async_rt::prelude::AsyncRuntime;
// use pi_async_rt::prelude::serial::AsyncRuntime;

static INTI_STROE_VALUE: Mutex<Vec<AsyncValue<()>>> = Mutex::new(Vec::new());
static INTI_STROE: AtomicBool = AtomicBool::new(false);

pub static FONT_SIZE: usize = 32;
pub static PXRANGE: u32 = 10;
// /// 二维装箱
// pub struct Packer2D {

// }

// 此函数决将绘制的字体字号映射为需要的sdf字号（因为一些文字很大时， 小的sdf纹理， 不能满足其精度需求）
pub fn sdf_font_size(_font_size: usize) -> usize {
    FONT_SIZE
}

pub struct Sdf2Table {
    pub fonts: SecondaryMap<DefaultKey, FontFace>, // DefaultKey为FontFaceId
    pub metrics: SecondaryMap<DefaultKey, MetricsInfo>, // DefaultKey为FontFaceId
    pub max_boxs: SecondaryMap<DefaultKey, Aabb>,  // DefaultKey为FontId
    // text_infos: SecondaryMap<DefaultKey, TexInfo>,

    // blob_arcs: Vec<(BlobArc, HashMap<String, u64>)>,
    glyph_id_map: XHashMap<(FontFamilyId, char), GlyphId>,
    pub glyphs: SlotMap<DefaultKey, GlyphIdDesc>,

    pub(crate) index_packer: TextPacker,
    pub data_packer: TextPacker,
    pub outline_info: XHashMap<(DefaultKey, char), OutlineInfo>,

    // 字体阴影参数， u32: 模糊半径; NotNan<f32>: 粗体正常和细体
    pub font_shadow: XHashMap<GlyphId, Vec<(u32, NotNan<f32>)>>,
    // 字体外发光参数 u32: 发光半径
    pub font_outer_glow: XHashMap<GlyphId, Vec<u32>>,
    // 字体阴影sdf纹理数据信息
    pub font_shadow_info: XHashMap<(GlyphId, u32, NotNan<f32>), Glyph>,
    // 字体外发光sdf纹理数据信息
    pub font_outer_glow_info: XHashMap<(GlyphId, u32), Glyph>,

    // box_shadow 参数信息；box信息, 生成: sdf纹理大小, f32: 模糊半径
    pub bboxs: XHashMap<u64, BoxInfo>,
    // svg 参数信息；SvgInfo 圆弧数据， usize: 生成sdf纹理大小, u32: 梯度递减范围(直径), u32: cut_off范围(半径)
    pub shapes: XHashMap<u64, (SvgInfo, usize, u32, u32)>,
    // svg 阴影参数信息; u32 模糊半径
    pub shapes_shadow: XHashMap<u64, Vec<u32>>,
    // svg 外发光参数信息; u32 发光半径
    pub shapes_outer_glow: XHashMap<u64, Vec<u32>>,

    // svg sdf纹理数据信息
    pub shapes_tex_info: XHashMap<u64, SvgTexInfo>,
    // svg 阴影sdf纹理数据信息
    pub shapes_shadow_tex_info: XHashMap<(u64, u32), SvgTexInfo>,
    // svg 外发光sdf纹理数据信息
    pub shapes_outer_glow_tex_info: XHashMap<(u64, u32), SvgTexInfo>,
}

#[derive(Debug)]
pub enum SdfType {
    Normal,
    Shadow(u32, NotNan<f32>),
    OuterGlow(u32),
}

#[derive(Debug, Clone, Default)]
pub struct SvgTexInfo {
    pub x: f32,
    pub y: f32,
    pub width: usize,
    pub height: usize,
}

impl Sdf2Table {
    pub fn new(width: usize, height: usize) -> Self {
        let _ = MULTI_MEDIA_RUNTIME.spawn(async move {
            if !INTI_STROE.load(Ordering::Relaxed) {
                init_local_store().await;
                log::error!("init_local_store end");
                INTI_STROE.store(true, Ordering::Relaxed);
                for v in INTI_STROE_VALUE.lock().unwrap().drain(..) {
                    v.set(());
                }
            }
        });

        Self {
            fonts: Default::default(),
            metrics: Default::default(),
            max_boxs: Default::default(),
            // text_infos: Default::default(),
            // blob_arcs: Default::default(),
            glyph_id_map: XHashMap::default(),
            glyphs: SlotMap::default(),
            outline_info: XHashMap::default(),
            // base_glyphs: SlotMap<DefaultKey, BaseCharDesc>,
            index_packer: TextPacker::new(width, height),
            data_packer: TextPacker::new(width, height),
            // size: Size {
            // 	width,
            // 	height
            // },
            bboxs: XHashMap::default(),
            shapes: XHashMap::default(),
            shapes_tex_info: XHashMap::default(),
            font_shadow: XHashMap::default(),
            font_outer_glow: XHashMap::default(),
            font_shadow_info: XHashMap::default(),
            font_outer_glow_info: XHashMap::default(),
            shapes_shadow: XHashMap::default(),
            shapes_outer_glow: XHashMap::default(),
            shapes_shadow_tex_info: XHashMap::default(),
            shapes_outer_glow_tex_info: XHashMap::default(),
            // texs: XHashMap::default(),
        }
    }

    pub fn data_packer_size(&self) -> Size<usize> {
        Size {
            width: self.data_packer.width,
            height: self.data_packer.height,
        }
    }

    pub fn index_packer_size(&self) -> Size<usize> {
        Size {
            width: self.index_packer.width,
            height: self.index_packer.height,
        }
    }

    // 添加字体
    pub fn add_font(&mut self, font_id: FontFaceId, buffer: Share<Vec<u8>>) {
        // #[cfg(all(not(target_arch="wasm32"), not(feature="empty")))]
        let face = FontFace::new(buffer);
        // #[cfg(all(target_arch="wasm32", not(feature="empty")))]
        let ascender = face.ascender();
        let descender = face.descender();
        let height = ascender - descender;

        self.metrics.insert(
            font_id.0,
            MetricsInfo {
                font_size: FONT_SIZE as f32,
                distance_range: PXRANGE as f32,
                line_height: height,
                max_height: height,
                ascender: ascender,
                descender: descender,
                underline_y: 0.0,         // todo 暂时不用，先写0
                underline_thickness: 0.0, // todo
                em_size: 1.0,
                // units_per_em: r.units_per_em(),
            },
        );

        let max_box = face.max_box();
        self.fonts.insert(font_id.0, face);
        self.max_boxs.insert(font_id.0, max_box.0);
    }

    // 文字高度
    pub fn height(&mut self, font: &FontInfo) -> (f32, f32 /*max_height*/) {
        let mut ret = (0.0, 0.0);
        for font_id in font.font_ids.iter() {
            if let Some(r) = self.fonts.get(font_id.0) {
                let height = r.ascender() - r.descender();
                if height > ret.0 {
                    ret.0 = height;
                }
            };
        }
        ret.1 = ret.0;
        ret
    }

    pub fn metrics(&self, glyph_id: GlyphId, font: &FontInfo) -> Option<&MetricsInfo> {
        let glyph = &self.glyphs[glyph_id.0];
        if glyph.font_face_index.is_null() {
            return None;
        } else {
            let face_id = font.font_ids[glyph.font_face_index].0;
            if let Some(r) = self.metrics.get(face_id) {
                return Some(r);
            } else {
                return None;
            }
        }
    }

    pub fn fontface_metrics(&self, face_id: FontFaceId) -> Option<&MetricsInfo> {
        if let Some(r) = self.metrics.get(face_id.0) {
            return Some(r);
        } else {
            return None;
        }
    }

    // 文字宽度
    pub fn width(&mut self, font_id: FontId, font: &mut FontInfo, char: char) -> (f32, GlyphId) {
        let glyph_id = self.glyph_id(font_id, font, char);
        if self.glyphs[glyph_id.0].font_face_index.is_null() {
            for (index, font_id) in font.font_ids.iter().enumerate() {
                if let Some(r) = self.fonts.get_mut(font_id.0) {
                    let horizontal_advance = r.horizontal_advance(char);
                    if horizontal_advance >= 0.0 {
                        self.glyphs[glyph_id.0].font_face_index = index;
                        log::debug!(
                            "mesure char width first, char: {}, font_id: {:?}, width: {:?}",
                            char,
                            font_id,
                            (
                                horizontal_advance,
                                font.font.font_size as f32,
                                horizontal_advance * font.font.font_size as f32
                            )
                        );
                        self.glyphs[glyph_id.0].glyph.advance = horizontal_advance;
                        return (horizontal_advance * font.font.font_size as f32, glyph_id);
                    }
                };
            }
        } else {
            return (
                self.glyphs[glyph_id.0].glyph.advance * font.font.font_size as f32,
                glyph_id,
            );
        }

        return (0.0, glyph_id);
    }

    pub fn glyph_id_desc(&self, glyph_id: GlyphId) -> &GlyphIdDesc {
        &self.glyphs[glyph_id.0]
    }

    // 字形id
    pub fn glyph_id(&mut self, font_id: FontId, font_info: &mut FontInfo, char: char) -> GlyphId {
        let id = match self.glyph_id_map.entry((font_info.font_family_id, char)) {
            Entry::Occupied(r) => {
                let id = r.get().clone();
                return id;
            }
            Entry::Vacant(r) => {
                // 分配GlyphId
                let id = GlyphId(self.glyphs.insert(GlyphIdDesc {
                    font_id,
                    char,
                    font_face_index: pi_null::Null::null(),
                    glyph: Glyph::default(),
                }));

                if self.glyphs[id.0].font_face_index.is_null() {
                    for (index, font_id) in font_info.font_ids.iter().enumerate() {
                        if let Some(font_face) = self.fonts.get_mut(font_id.0) {
                            let r = font_face.to_outline3(char);

                            let (plane_bounds, atlas_bounds, _, tex_size) = compute_layout(
                                &mut r.bbox.clone(),
                                FONT_SIZE,
                                PXRANGE,
                                r.units_per_em,
                                PXRANGE,
                                false,
                            );
                            let offset = self.index_packer.alloc(tex_size, tex_size).unwrap();

                            let glyph = Glyph {
                                ox: plane_bounds.mins.x,
                                oy: plane_bounds.mins.y,
                                x: offset.x as f32 + atlas_bounds.mins.x,
                                y: offset.y as f32 + atlas_bounds.mins.y,
                                width: atlas_bounds.maxs.x - atlas_bounds.mins.x,
                                height: atlas_bounds.maxs.y - atlas_bounds.mins.x,
                                advance: r.advance as f32,
                            };
                            self.glyphs[id.0].glyph = glyph;
                            self.outline_info.insert((font_id.0, char), r);
                        };
                    }
                }

                // 放入等待队列, 并统计等待队列的总宽度
                // font.await_info.size.width += size.width.ceil() as usize;
                // font.await_info.size.height += size.height.ceil() as usize;

                if !char.is_whitespace() {
                    // 不是空白符， 才需要放入等待队列
                    font_info.await_info.wait_list.push(id);
                }
                r.insert(id).clone()
            }
        };

        return id;
    }

    // 字形id
    pub fn add_font_shadow(&mut self, id: GlyphId, radius: u32, weight: NotNan<f32>) {
        // let id =  self.glyph_id_map.get((font_info.font_family_id, char));
        if let Entry::Vacant(vacant_entry) = self.font_shadow_info.entry((id, radius, weight)) {
            let c = &self.glyphs[id.0];
            let outline_info = self.outline_info.get(&(c.font_id.0, c.char)).unwrap();

            let (plane_bounds, atlas_bounds, _, tex_size) = compute_layout(
                &mut outline_info.bbox.clone(),
                FONT_SIZE,
                radius,
                outline_info.units_per_em,
                radius,
                false,
            );
            let offset = self.index_packer.alloc(tex_size, tex_size).unwrap();

            let glyph = Glyph {
                ox: plane_bounds.mins.x,
                oy: plane_bounds.mins.y,
                x: offset.x as f32 + atlas_bounds.mins.x,
                y: offset.y as f32 + atlas_bounds.mins.y,
                width: atlas_bounds.maxs.x - atlas_bounds.mins.x,
                height: atlas_bounds.maxs.y - atlas_bounds.mins.x,
                advance: outline_info.advance as f32,
            };
            vacant_entry.insert(glyph);

            if let Some(v) = self.font_shadow.get_mut(&id) {
                v.push((radius, weight));
            } else {
                self.font_shadow.insert(id, vec![(radius, weight)]);
            }
        }
        
    }

    // 字形id
    pub fn add_font_outer_glow(&mut self, id: GlyphId, range: u32) {
        if let Entry::Vacant(vacant_entry) = self.font_outer_glow_info.entry((id, range)) {
            let c = &self.glyphs[id.0];
            let outline_info = self.outline_info.get(&(c.font_id.0, c.char)).unwrap();

            let (plane_bounds, atlas_bounds, _, tex_size) = compute_layout(
                &mut outline_info.bbox.clone(),
                FONT_SIZE,
                range,
                outline_info.units_per_em,
                range,
                false,
            );
            let offset = self.index_packer.alloc(tex_size, tex_size).unwrap();

            let glyph = Glyph {
                ox: plane_bounds.mins.x,
                oy: plane_bounds.mins.y,
                x: offset.x as f32 + atlas_bounds.mins.x,
                y: offset.y as f32 + atlas_bounds.mins.y,
                width: atlas_bounds.maxs.x - atlas_bounds.mins.x,
                height: atlas_bounds.maxs.y - atlas_bounds.mins.x,
                advance: outline_info.advance as f32,
            };
            vacant_entry.insert(glyph);

            if let Some(v) = self.font_outer_glow.get_mut(&id) {
                v.push(range);
            } else {
                self.font_outer_glow.insert(id, vec![range]);
            }
        }
    }

    pub fn add_box_shadow(&mut self, hash: u64, bbox: Aabb, tex_size: usize, radius: u32) -> u64 {
        let info = compute_box_layout(bbox, tex_size, radius);
        self.bboxs.insert(hash, info.clone());

        let index_position = self
            .index_packer
            .alloc(info.p_w as usize, info.p_h as usize)
            .unwrap();

        self.shapes_shadow_tex_info.insert(
            (hash, radius),
            SvgTexInfo {
                x: index_position.x as f32 + info.atlas_bounds.mins.x,
                y: index_position.y as f32 + info.atlas_bounds.mins.y,
                width: (info.atlas_bounds.maxs.x - info.atlas_bounds.mins.x) as usize,
                height: (info.atlas_bounds.maxs.y - info.atlas_bounds.mins.y) as usize,
            },
        );
        hash
    }

    pub fn add_shape(
        &mut self,
        hash: u64,
        info: SvgInfo,
        tex_size: usize,
        pxrang: u32,
        cut_off: u32,
    ) {
        let info2 = info.compute_layout(tex_size, pxrang, cut_off);

        // let mut texinfo = TexInfo2 {
        //     sdf_offset_x: 0,
        //     sdf_offset_y: 0,
        //     advance: 0.0,
        //     char: ' ',
        //     plane_min_x: info2[0],
        //     plane_min_y: info2[1],
        //     plane_max_x: info2[2],
        //     plane_max_y: info2[3],
        //     atlas_min_x: info2[4],
        //     atlas_min_y: info2[5],
        //     atlas_max_x: info2[6],
        //     atlas_max_y: info2[7],
        // };
        // let index_packer: &'static mut TextPacker = unsafe { transmute(&mut self.index_packer) };
        let index_position = self
            .index_packer
            .alloc(info2[8] as usize, info2[8] as usize)
            .unwrap();

        self.shapes_tex_info.insert(
            hash,
            SvgTexInfo {
                x: index_position.x as f32 + info2[4],
                y: index_position.y as f32 + info2[5],
                width: info2[8] as usize,
                height: info2[8] as usize,
            },
        );
        self.shapes.insert(hash, (info, tex_size, pxrang, cut_off));
    }

    pub fn add_shape_shadow(&mut self, id: u64, radius: u32) {
        if let Some((info, tex_size, pxrang, cut_off)) = self.shapes.get(&id) {
            if *cut_off <= radius {
                let info = self.shapes_tex_info.get(&id).unwrap().clone();
                self.shapes_shadow_tex_info.insert((id, radius), info);
            } else {
                let info = info.compute_layout(*tex_size, radius, radius);
                let index_position = self
                    .index_packer
                    .alloc(info[8] as usize, info[8] as usize)
                    .unwrap();
                self.shapes_shadow_tex_info.insert(
                    (id, radius),
                    SvgTexInfo {
                        x: index_position.x as f32 + info[4],
                        y: index_position.y as f32 + info[5],
                        width: (info[6] - info[4]) as usize,
                        height: (info[7] - info[5]) as usize,
                    },
                );
            }

            if let Some(v) = self.shapes_shadow.get_mut(&id) {
                v.push(radius);
            } else {
                self.shapes_shadow.insert(id, vec![radius]);
            }
        }
    }

    pub fn add_shape_outer_glow(&mut self, id: u64, radius: u32) {
        if let Some((info, tex_size, pxrang, cut_off)) = self.shapes.get(&id) {
            if *cut_off <= radius {
                self.shapes_outer_glow_tex_info
                    .insert((id, radius), self.shapes_tex_info.get(&id).unwrap().clone());
            } else {
                let info = info.compute_layout(*tex_size, radius, radius);
                let index_position = self
                    .index_packer
                    .alloc(info[8] as usize, info[8] as usize)
                    .unwrap();
                self.shapes_outer_glow_tex_info.insert(
                    (id, radius),
                    SvgTexInfo {
                        x: index_position.x as f32 + info[4],
                        y: index_position.y as f32 + info[5],
                        width: (info[6] - info[4]) as usize,
                        height: (info[7] - info[5]) as usize,
                    },
                );
            }

            if let Some(v) = self.shapes_outer_glow.get_mut(&id) {
                v.push(radius);
            } else {
                self.shapes_outer_glow.insert(id, vec![radius]);
            }
        }
    }

    /// 取到字形信息
    pub fn glyph(&self, id: GlyphId) -> &Glyph {
        if self.glyphs.get(*id).is_none() {
            panic!("glyph is not exist, {:?}", id);
        }
        &self.glyphs[*id].glyph
    }

    // /// 更新字形信息（计算圆弧信息）
    // /// 通过回调函数更新
    // pub fn update<F: FnMut(Block, FontImage) + Clone + ThreadSync + 'static>(&mut self, fonts: &mut SlotMap<DefaultKey, FontInfo>, mut update: F) {

    // 	let mut await_count = 0;
    // 	for (_, font_info) in fonts.iter() {
    // 		await_count += font_info.await_info.wait_list.len();
    // 	}

    // 	// 轮廓信息（贝塞尔曲线）
    // 	let mut outline_infos = Vec::with_capacity(await_count);

    // 	// let mut sdf_all_draw_slotmap = Vec::new();
    // 	// let mut f = Vec::new();
    // 	// 遍历所有的等待文字， 取到文字的贝塞尔曲线描述
    // 	for (_, font_info) in fonts.iter_mut() {
    // 		let await_info = &mut font_info.await_info;
    // 		if await_info.wait_list.len() == 0 {
    // 			continue;
    // 		}

    // 		for glyph_id in await_info.wait_list.drain(..) {
    // 			let g = &self.glyphs[*glyph_id];
    // 			let font_face_id = font_info.font_ids[g.font_face_index];
    // 			outline_infos.push((self.fonts[font_face_id.0].to_outline(g.char), font_face_id.0, glyph_id)); // 先取到贝塞尔曲线
    // 		}
    // 	}

    // 	let texture_data = Vec::with_capacity(await_count);
    // 	let result = Share::new(ShareMutex::new((0, texture_data)));
    // 	let async_value = AsyncValue::new();

    // 	let max_boxs: &'static SecondaryMap<DefaultKey,  parry2d::bounding_volume::Aabb> = unsafe { transmute(&self.max_boxs) };
    // 	// 遍历所有等待处理的字符贝塞尔曲线，将曲线转化为圆弧描述（多线程）
    // 	for glyph_visitor in outline_infos.drain(..) {
    // 		let async_value1 = async_value.clone();
    // 		let result1 = result.clone();
    // 		MULTI_MEDIA_RUNTIME.spawn(async move {
    // 			let (mut blod_arc, map) = FontFace::get_char_arc(max_boxs[glyph_visitor.1].clone(), glyph_visitor.0);

    // 			log::trace!("encode_data_tex======{:?}, {:?}", blod_arc.grid_size(), map);
    // 			let data_tex = blod_arc.encode_data_tex1(&map);
    // 			// println!("data_map: {}", map.len());
    // 			let (info, index_tex) = blod_arc.encode_index_tex1( map, data_tex.len() / 4);

    // 			// log::debug!("load========={:?}, {:?}", lock.0, len);
    // 			let mut lock = result1.lock();
    // 			lock.1.push((glyph_visitor.1, info, data_tex, index_tex));
    // 			lock.0 += 1;
    // 			if lock.0 == await_count {
    // 				async_value1.set(true);
    // 			}
    // 		}).unwrap();
    // 	}

    // 	let index_packer: &'static mut TextPacker = unsafe { transmute(&mut self.index_packer)};
    // 	let data_packer: &'static mut TextPacker = unsafe { transmute(&mut self.data_packer)};
    // 	let glyphs: &'static mut SlotMap<DefaultKey, GlyphIdDesc> = unsafe { transmute(&mut self.glyphs)};

    // 	MULTI_MEDIA_RUNTIME.spawn(async move {
    // 		log::debug!("sdf2 load1=========");
    // 		async_value.await;
    // 		let mut lock = result.lock();
    // 		let r = &mut lock.1;
    // 		log::debug!("sdf2 load2========={:?}", r.len());

    // 		while let Some((glyph_id, mut text_info, mut data_tex, index_tex)) = r.pop() {
    // 			// 索引纹理更新
    // 			let index_tex_position = index_packer.alloc(
    // 			text_info.grid_w as usize,
    // 			text_info.grid_h as usize);
    // 			let index_position = match index_tex_position {
    // 				Some(r) => r,
    // 				None => panic!("aaaa================"),
    // 			};
    // 			let index_img = FontImage {
    // 				width: text_info.grid_w as usize,
    // 				height: text_info.grid_h as usize,
    // 				buffer: index_tex,
    // 			};
    // 			text_info.index_offset = (index_position.x, index_position.y);
    // 			let index_block = Block {
    // 				y: index_position.x as f32,
    // 				x: index_position.y as f32,
    // 				width: index_img.width as f32,
    // 				height: index_img.height as f32,
    // 			};
    // 			// log::warn!("update index tex========={:?}", (&index_block,index_img.width, index_img.height, index_img.buffer.len(), &text_info) );
    // 			(update.clone())(index_block, index_img);

    // 			// 数据纹理更新
    // 			let data_len = data_tex.len() / 4;
    // 			let mut patch = 8.0 - data_len as f32 % 8.0;
    // 			let p1 = patch;
    // 			while patch > 0.0 {
    // 				data_tex.extend_from_slice(&[0, 0, 0, 0]); // 补0
    // 				patch -= 1.0;
    // 			}

    // 			let h = (data_len as f32 / 8.0).ceil() as usize;
    // 			let data_img = FontImage {
    // 				width: 8,
    // 				height: h,
    // 				buffer: data_tex,
    // 			};
    // 			let data_position = data_packer.alloc(
    // 				text_info.grid_w as usize,
    // 				text_info.grid_h as usize);
    // 			let data_position = match data_position {
    // 				Some(r) => r,
    // 				None => panic!("bbb================"),
    // 			};
    // 			text_info.data_offset = (data_position.x, data_position.y);
    // 			let data_block = Block {
    // 				y: data_position.x as f32,
    // 				x: data_position.y as f32,
    // 				width: data_img.width as f32,
    // 				height: data_img.height as f32,
    // 			};
    // 			// log::warn!("update data tex========={:?}", (&data_block, data_img.width, data_img.height, data_img.buffer.len(), data_len, p1) );
    // 			update(data_block, data_img);

    // 			glyphs[glyph_id].glyph = text_info;

    // 			log::warn!("text_info=========={:?}, {:?}", glyph_id, glyphs[glyph_id].glyph);

    // 		}
    // 	}).unwrap();

    // }

    /// 更新字形信息（计算圆弧信息）
    pub fn draw_await(
        &mut self,
        fonts: &mut SlotMap<DefaultKey, FontInfo>,
        index: usize,
        result: SdfResult,
    ) -> AsyncValue<()> {
        let task_count = Arc::new(AtomicUsize::new(0));
        let task_num = Arc::new(AtomicUsize::new(0));
        let async_value = AsyncValue::new();
        self.draw_font_await(
            fonts,
            result.clone(),
            index,
            task_count.clone(),
            task_num.clone(),
            async_value.clone(),
        );
        self.draw_svg_await(
            result.clone(),
            task_count.clone(),
            task_num.clone(),
            async_value.clone(),
        );
        self.draw_box_shadow_await(
            result.clone(),
            task_count.clone(),
            task_num.clone(),
            async_value.clone(),
        );

        async_value
    }

    /// 更新字形信息（计算圆弧信息）
    pub fn draw_font_await(
        &mut self,
        fonts: &mut SlotMap<DefaultKey, FontInfo>,
        result: SdfResult,
        index: usize,
        task_count: Arc<AtomicUsize>,
        task_num: Arc<AtomicUsize>,
        async_value: AsyncValue<()>,
    ) {
        // let mut await_count = 0;
        let await_count = task_count;
        for (_, font_info) in fonts.iter() {
            await_count.fetch_add(font_info.await_info.wait_list.len(), Ordering::Relaxed);
        }

        // let async_value = AsyncValue::new();
        if await_count.load(Ordering::Relaxed) == 0 {
            // async_value.clone().set(());
            // println!("encode_data_texzzzzz===={:?}, {:?}", index, await_count);
            return;
        }

        // 轮廓信息（贝塞尔曲线）
        let mut outline_infos = Vec::with_capacity(await_count.load(Ordering::Relaxed));
        // let mut outline_infos2: HashMap<DefaultKey, Vec<(char, GlyphId, &str)>> = HashMap::with_capacity(await_count);
        let mut chars = Vec::new();
        let mut keys = Vec::new();
        // let mut sdf_all_draw_slotmap = Vec::new();
        // let mut f = Vec::new();
        // 遍历所有的等待文字， 取到文字的贝塞尔曲线描述
        if await_count.load(Ordering::Relaxed) != 0 {
            for (_, font_info) in fonts.iter_mut() {
                let await_info = &mut font_info.await_info;
                if await_info.wait_list.len() == 0 {
                    continue;
                }

                for glyph_id in await_info.wait_list.drain(..) {
                    let g = &self.glyphs[*glyph_id];
                    // font_face_index不存在， 不需要计算
                    if g.font_face_index.is_null() {
                        continue;
                    }
                    let font_face_id = font_info.font_ids[g.font_face_index];
                    if let Some(font_face) = self.fonts.get_mut(font_face_id.0) {
                        let glyph_index = font_face.glyph_index(g.char);

                        // log::error!("{} glyph_index: {}", g.char, glyph_index);
                        // 字体中不存在字符
                        if glyph_index == 0 {
                            await_count.fetch_sub(1, Ordering::Relaxed);
                            continue;
                        }
                        let is_outer_glow = self.font_outer_glow.remove(&glyph_id);
                        let shadow = self.font_shadow.remove(&glyph_id);
                        // #[cfg(all(not(target_arch="wasm32"), not(feature="empty")))]
                        // ;
                        outline_infos.push((
                            self.outline_info.remove(&(font_face_id.0, g.char)).unwrap(),
                            font_face_id.0,
                            glyph_id,
                            is_outer_glow,
                            shadow,
                        )); // 先取到贝塞尔曲线
                        keys.push(format!(
                            "{}{}",
                            g.char,
                            font_info.font.font_family[g.font_face_index].as_str()
                        ));
                        chars.push(g.char)
                    }
                }
            }
        }

        result.0.lock().unwrap().font_result =
            Vec::with_capacity(await_count.load(Ordering::Relaxed));

        let mut ll = 0;

        // let temp_value = async_value.clone();

        MULTI_MEDIA_RUNTIME
            .spawn(async move {
                if !INTI_STROE.load(Ordering::Relaxed) {
                    log::error!("=============存储未初始化");
                    let async_value3 = AsyncValue::new();
                    INTI_STROE_VALUE.lock().unwrap().push(async_value3.clone());
                    async_value3.await;
                }
                // log::error!("encode_data_texxxx===={:?}, {:?}, {:?}, {:?}", index, ll, await_count, chars);
                // 遍历所有等待处理的字符贝塞尔曲线，将曲线转化为圆弧描述（多线程）
                for mut glyph_visitor in outline_infos.drain(..) {
                    let async_value1 = async_value.clone();
                    let result = result.clone();
                    // println!("encode_data_tex===={:?}", index);
                    let task_num = task_num.clone();
                    let await_count = await_count.clone();

                    let key = keys[ll].clone();
                    MULTI_MEDIA_RUNTIME
                        .spawn(async move {
                            let temp_key = key.clone();
                            let mut hasher = DefaultHasher::new();
                            key.hash(&mut hasher);
                            let key = hasher.finish().to_string();
                            let result_arcs = if let Some(buffer) = stroe::get(key.clone()).await {
                                let arcs: CellInfo = bincode::deserialize(&buffer[..]).unwrap();
                                arcs
                            // log::trace!("store is have: {}, sdf_tex: {}, atlas_bounds: {:?}", temp_key, sdf_tex.len(), atlas_bounds);
                            // (char, plane_bounds, atlas_bounds, advance, sdf_tex, tex_size)
                            } else {
                                // let (mut blod_arc, map) = FontFace::get_char_arc(max_boxs[glyph_visitor.1].clone(), glyph_visitor.0);

                                // let data_tex = blod_arc.encode_data_tex1(&map);
                                // // println!("data_map: {}", map.len());
                                // let (info, index_tex,sdf_tex1, sdf_tex2, sdf_tex3, sdf_tex4) = blod_arc.encode_index_tex1( map, data_tex.len() / 4);
                                #[cfg(all(not(target_arch = "wasm32"), not(feature = "empty")))]
                                {
                                    let arcs = glyph_visitor.0.compute_near_arcs(1.0);
                                    let buffer = bincode::serialize(&arcs).unwrap();
                                    stroe::write(key, buffer).await;
                                    arcs
                                }

                                #[cfg(all(target_arch = "wasm32", not(feature = "empty")))]
                                {
                                    let buffer = FontFace::compute_sdf_tex(
                                        glyph_visitor.0,
                                        FONT_SIZE,
                                        PXRANGE,
                                    )
                                    .await;

                                    let sdf: SdfInfo2 = bincode::deserialize(&buffer).unwrap();

                                    stroe::write(key, buffer).await;
                                    sdf
                                }
                            };
                            let lock = &mut result.0.lock().unwrap().font_result;
                            let sdf = glyph_visitor.0.compute_sdf_tex(
                                result_arcs.clone(),
                                FONT_SIZE,
                                PXRANGE,
                                false,
                            );
                            lock.push((glyph_visitor.2 .0, sdf, SdfType::Normal));

                            if let Some(outer_ranges) = glyph_visitor.3 {
                                for v in outer_ranges {
                                    let outer_glow_sdf = glyph_visitor.0.compute_sdf_tex(
                                        result_arcs.clone(),
                                        FONT_SIZE,
                                        v,
                                        true,
                                    );
                                    lock.push((
                                        glyph_visitor.2 .0,
                                        outer_glow_sdf,
                                        SdfType::OuterGlow(v),
                                    ));
                                }
                            }

                            if let Some(args) = glyph_visitor.4 {
                                for (shadow_range, weight) in args {
                                    let SdfInfo2 {
                                        tex_info,
                                        sdf_tex,
                                        tex_size,
                                    } = glyph_visitor.0.compute_sdf_tex(
                                        result_arcs.clone(),
                                        FONT_SIZE,
                                        shadow_range + 2,
                                        false,
                                    );
                                    let sdf_tex = gaussian_blur(
                                        sdf_tex,
                                        tex_size as u32,
                                        tex_size as u32,
                                        shadow_range,
                                        f32::from(weight),
                                    );
                                    lock.push((
                                        glyph_visitor.2 .0,
                                        SdfInfo2 {
                                            tex_info,
                                            sdf_tex,
                                            tex_size,
                                        },
                                        SdfType::Shadow(shadow_range, weight),
                                    ));
                                }
                            }

                            // log::debug!("load========={:?}, {:?}", lock.0, len);
                            // let mut lock = result1.lock().unwrap();
                            task_num.fetch_add(1, Ordering::Relaxed);
                            // println!("encode_data_tex0===={:?}", (index, ll));
                            // log::trace!("encode_data_tex======cur_count: {:?}, atlas_bounds={:?}, await_count={:?}, tex_size={:?}", lock.0, atlas_bounds, await_count, tex_size);
                            // lock.1.push((glyph_visitor.2 .0, sdf));
                            if task_num.load(Ordering::Relaxed)
                                == await_count.load(Ordering::Relaxed)
                            {
                                log::trace!("encode_data_tex1");
                                async_value1.set(());
                                // println!("encode_data_tex1===={}", index);
                                log::trace!("encode_data_tex2");
                            }
                        })
                        .unwrap();
                    ll += 1;
                }
            })
            .unwrap();
    }

    pub fn update<F: FnMut(Block, FontImage) + Clone + 'static>(
        &mut self,
        update: F,
        // mut updtae_shadow: F1,
        result: SdfResult,
    ) {
        let mut result = result.0.lock().unwrap();
        self.update_font(update.clone(), &mut result.font_result);
        self.update_box_shadow(update.clone(), &mut result.box_result);
        self.update_svg(update, &mut result.svg_result);
    }

    pub fn update_font<F: FnMut(Block, FontImage) + Clone + 'static>(
        &mut self,
        update: F,
        // mut updtae_shadow: F1,
        result: &mut Vec<(DefaultKey, SdfInfo2, SdfType)>,
    ) {
        let index_packer: &'static mut TextPacker = unsafe { transmute(&mut self.index_packer) };
        // let data_packer: &'static mut TextPacker = unsafe { transmute(&mut self.data_packer)};
        let glyphs: &'static mut SlotMap<DefaultKey, GlyphIdDesc> =
            unsafe { transmute(&mut self.glyphs) };

        // let mut lock = result.lock().unwrap();
        let r = result;
        log::debug!("sdf2 load2========={:?}", r.len());

        while let Some((
            glyph_id,
            SdfInfo2 {
                tex_info,
                sdf_tex,
                tex_size,
            },
            sdf_type,
        )) = r.pop()
        {
            // let r =
            // 索引纹理更新
            // let tex_position = index_packer.alloc(tex_size as usize, tex_size as usize);
            // let sdf_position = match tex_position {
            //     Some(r) => r,
            //     None => panic!("aaaa================"),
            // };
            let sdf_img = FontImage {
                width: tex_size as usize,
                height: tex_size as usize,
                buffer: sdf_tex,
            };
            let glyph = &glyphs[glyph_id].glyph;
            // let index_offset_x = sdf_position.x as f32 + tex_info.atlas_min_x ;
            // let index_offset_y = sdf_position.y as f32 + tex_info.atlas_min_y;
            // tex_info.sdf_offset_x = glyph.x -  tex_info.atlas_min_x ;
            // tex_info.sdf_offset_x = glyph.y -  tex_info.atlas_min_y ;;
            let sdf_block = Block {
                x: glyph.x - tex_info.atlas_min_x as f32,
                y: glyph.y - tex_info.atlas_min_y as f32,
                width: tex_size as f32,
                height: tex_size as f32,
            };
            // log::warn!("update index tex========={:?}", (&index_block,index_img.width, index_img.height, index_img.buffer.len(), &text_info) );
            (update.clone())(sdf_block, sdf_img);

            // let advance = glyphs[glyph_id].glyph.advance;
            // let glyph = Glyph {
            //     ox: tex_info.plane_min_x,
            //     oy: tex_info.plane_min_y,
            //     x: tex_info.sdf_offset_x as f32 + tex_info.atlas_min_x,
            //     y: tex_info.sdf_offset_y as f32 + tex_info.atlas_min_y,
            //     width: tex_info.atlas_max_x - tex_info.atlas_min_x,
            //     height: tex_info.atlas_max_y - tex_info.atlas_min_y,
            //     advance,
            // };
            // match sdf_type {
            //     SdfType::Normal => {
            //         // glyphs[glyph_id].glyph = glyph;
            //     }
            //     SdfType::Shadow(range, weight) => {
            //         // self.font_shadow_info
            //         //     .insert((GlyphId(glyph_id), range, weight), glyph);
            //     }
            //     SdfType::OuterGlow(range) => {
            //         // self.font_outer_glow_info
            //         //     .insert((GlyphId(glyph_id), range), glyph);
            //     }
            // }

            // log::trace!("text_info=========={:?}, {:?}, {:?}, {:?}", glyph_id, glyphs[glyph_id].glyph, index_position, data_position);
        }
    }

    pub fn has_box_shadow(&mut self, hash: u64) -> bool {
        self.shapes_tex_info.get(&hash).is_some()
    }

    // pub fn allow_tex(&mut self, hash: u64, width: usize, height: usize) -> (usize, usize) {
    //     let index_packer: &'static mut TextPacker = unsafe { transmute(&mut self.index_packer) };
    //     let index_tex_position = index_packer.alloc(width as usize, height as usize);
    //     match index_tex_position {
    //         Some(r) => {
    //             self.texs.insert(hash, r);
    //             (r.x, r.y)
    //         }
    //         None => panic!("aaaa================"),
    //     }
    // }

    /// 更新字形信息（计算圆弧信息）
    pub fn draw_box_shadow_await(
        &mut self,
        result: SdfResult,
        task_count: Arc<AtomicUsize>,
        task_num: Arc<AtomicUsize>,
        async_value: AsyncValue<()>,
    ) {
        let _ = task_count.fetch_add(self.bboxs.len(), Ordering::Relaxed);
        let await_count = task_count;

        // let async_value = AsyncValue::new();

        // 遍历所有等待处理的字符贝塞尔曲线，将曲线转化为圆弧描述（多线程）
        for (hash, box_info) in self.bboxs.drain() {
            let async_value1 = async_value.clone();
            let result1 = result.clone();
            let task_num = task_num.clone();
            let await_count = await_count.clone();
            MULTI_MEDIA_RUNTIME
                .spawn(async move {
                    let sdfinfo = blur_box(box_info.clone());

                    // log::debug!("load========={:?}, {:?}", lock.0, len);
                    let lock = &mut result1.0.lock().unwrap().box_result;
                    task_num.fetch_add(1, Ordering::Relaxed);
                    // log::trace!("encode_data_tex======cur_count: {:?}, grid_size={:?}, await_count={:?}, text_info={:?}", lock.0, await_count);
                    lock.push((hash, box_info, sdfinfo));
                    if task_num.load(Ordering::Relaxed) == await_count.load(Ordering::Relaxed) {
                        log::trace!("encode_data_tex1");
                        async_value1.set(());
                        log::trace!("encode_data_tex2");
                    }
                })
                .unwrap();
        }
        // async_value
    }

    pub fn update_box_shadow<F: FnMut(Block, FontImage) + Clone + 'static>(
        &mut self,
        update: F,
        result: &mut Vec<(u64, BoxInfo, Vec<u8>)>,
    ) {
        let index_packer: &'static mut TextPacker = unsafe { transmute(&mut self.index_packer) };
        // let data_packer: &'static mut TextPacker = unsafe { transmute(&mut self.data_packer) };
        // let shapes: &'static mut XHashMap<u64, TexInfo2> =
        //     unsafe { transmute(&mut self.shapes_tex_info) };

        // let mut lock = result.lock().unwrap();
        let r = result;
        log::debug!("sdf2 load2========={:?}", r.len());

        while let Some((hash, box_info, tex)) = r.pop() {
            // 索引纹理更新
            // let mut is_have = false;
            let index_position = self
                .shapes_shadow_tex_info
                .get(&(hash, box_info.radius))
                .unwrap();
            let index_img = FontImage {
                width: box_info.p_w as usize,
                height: box_info.p_h as usize,
                buffer: tex,
            };

            let index_block = Block {
                x: index_position.x - box_info.atlas_bounds.mins.x as f32 + 1.0,
                y: index_position.y - box_info.atlas_bounds.mins.y as f32 + 1.0,
                width: index_img.width as f32,
                height: index_img.height as f32,
            };
            // log::warn!("update index tex========={:?}", (&index_block,index_img.width, index_img.height, index_img.buffer.len(), &text_info) );
            (update.clone())(index_block, index_img);

            // if !is_have {
            //     let mut tex_info = SvgTexInfo::default();

            //     tex_info.x = index_position.x + box_info.atlas_bounds.mins.x;
            //     tex_info.y = index_position.y + box_info.atlas_bounds.mins.y;
            //     tex_info.width = box_info.p_w as usize;
            //     tex_info.height = box_info.p_h as usize;

            //     self.shapes_shadow_tex_info.insert((hash, 0), tex_info);
            // }
            // log::trace!("text_info=========={:?}, {:?}, {:?}, {:?}", glyph_id, glyphs[glyph_id].glyph, index_position, data_position);
        }
    }

    pub fn set_view_box(&mut self, mins_x: f32, mins_y: f32, maxs_x: f32, maxs_y: f32) {
        // self.svg.view_box = Aabb::new(Point::new(mins_x, mins_y), Point::new(maxs_x, maxs_y))
    }

    pub fn get_shape(&mut self, hash: u64) -> Option<&SvgTexInfo> {
        self.shapes_tex_info.get(&hash)
    }

    /// 更新svg信息（计算圆弧信息）
    pub fn draw_svg_await(
        &mut self,
        result: SdfResult,
        task_count: Arc<AtomicUsize>,
        task_num: Arc<AtomicUsize>,
        async_value: AsyncValue<()>,
    ) {
        task_count.fetch_add(self.shapes_tex_info.len(), Ordering::Relaxed);
        let await_count = task_count;
        // let texture_data = Vec::with_capacity(await_count);
        // result.lock().unwrap().1.reserve(await_count);
        // let result: Arc<ShareMutex<(usize, Vec<(u64, SdfInfo2)>)>> =
        //     Share::new(ShareMutex::new((0, texture_data)));
        // let async_value = AsyncValue::new();

        // 遍历所有等待处理的字符贝塞尔曲线，将曲线转化为圆弧描述（多线程）
        for (hash, (info, size, pxrange, cur_off)) in self.shapes.drain() {
            let outer_glow = self.shapes_outer_glow.remove(&hash);
            let shadow = self.shapes_shadow.remove(&hash);

            let async_value1 = async_value.clone();
            let result1 = result.clone();

            let task_num = task_num.clone();
            let await_count = await_count.clone();
            MULTI_MEDIA_RUNTIME
                .spawn(async move {
                    let lock = &mut result1.0.lock().unwrap().svg_result;
                    let sdfinfo =
                        compute_shape_sdf_tex(info.clone(), size, pxrange, false, cur_off);
                    lock.push((hash, sdfinfo.clone(), SdfType::Normal));
                    if let Some(outer_glow) = outer_glow {
                        for v in outer_glow {
                            let sdfinfo = compute_shape_sdf_tex(info.clone(), size, v, true, v / 2);
                            lock.push((hash, sdfinfo, SdfType::OuterGlow(v)));
                        }
                    }

                    if let Some(shadow) = shadow {
                        let SdfInfo2 {
                            tex_info,
                            sdf_tex,
                            tex_size,
                        } = sdfinfo;
                        for shadow_range in shadow {
                            let sdf_tex = gaussian_blur(
                                sdf_tex.clone(),
                                tex_size as u32,
                                tex_size as u32,
                                shadow_range,
                                0.0,
                            );
                            lock.push((
                                hash,
                                SdfInfo2 {
                                    tex_info: tex_info.clone(),
                                    sdf_tex,
                                    tex_size,
                                },
                                SdfType::Shadow(shadow_range, NotNan::new(0.0).unwrap()),
                            ));
                        }
                    }

                    // log::debug!("load========={:?}, {:?}", lock.0, len);

                    task_num.fetch_add(1, Ordering::Relaxed);
                    // log::trace!("encode_data_tex======cur_count: {:?}, grid_size={:?}, await_count={:?}, text_info={:?}", lock.0, await_count);

                    if await_count.load(Ordering::Relaxed) == task_num.load(Ordering::Relaxed) {
                        log::trace!("encode_data_tex1");
                        async_value1.set(());
                        log::trace!("encode_data_tex2");
                    }
                })
                .unwrap();
        }
    }

    pub fn update_svg<F: FnMut(Block, FontImage) + Clone + 'static>(
        &mut self,
        update: F,
        result: &mut Vec<(u64, SdfInfo2, SdfType)>,
    ) {
        let index_packer: &'static mut TextPacker = unsafe { transmute(&mut self.index_packer) };
        // let data_packer: &'static mut TextPacker = unsafe { transmute(&mut self.data_packer) };
        // let shapes: &'static mut XHashMap<u64, TexInfo2> =
        //     unsafe { transmute(&mut self.shapes_tex_info) };

        // let mut lock = result.lock().unwrap();
        let r = result;
        log::debug!("sdf2 load2========={:?}", r.len());

        while let Some((
            hash,
            SdfInfo2 {
                tex_info,
                sdf_tex,
                tex_size,
            },
            svg_type,
        )) = r.pop()
        {
            let mut is_have = false;
            let index_position = self.shapes_tex_info.get(&hash).unwrap();

            // // 索引纹理更新
            // let index_tex_position = index_packer.alloc(tex_size as usize, tex_size as usize);
            // let index_position = match index_tex_position {
            //     Some(r) => r,
            //     None => panic!("aaaa================"),
            // };
            let index_img = FontImage {
                width: tex_size as usize,
                height: tex_size as usize,
                buffer: sdf_tex,
            };
            // tex_info.sdf_offset_x = index_position.x;
            // tex_info.sdf_offset_x = index_position.y;
            let index_block = Block {
                x: index_position.x - tex_info.atlas_min_x as f32 + 1.0,
                y: index_position.y - tex_info.atlas_min_y as f32 + 1.0,
                width: index_img.width as f32,
                height: index_img.height as f32,
            };
            // log::warn!("update index tex========={:?}", (&index_block,index_img.width, index_img.height, index_img.buffer.len(), &text_info) );
            (update.clone())(index_block, index_img);

            // if !is_have {
            //     let info = SvgTexInfo {
            //         x: index_position.x + tex_info.atlas_min_x,
            //         y: index_position.y + tex_info.atlas_min_y,
            //         width: (tex_info.atlas_max_x - tex_info.atlas_min_x) as usize,
            //         height: (tex_info.atlas_max_y - tex_info.atlas_min_y) as usize,
            //     };

            //     match svg_type {
            //         SdfType::Normal => self.shapes_tex_info.insert(hash, info),
            //         SdfType::Shadow(r, _) => self.shapes_shadow_tex_info.insert((hash, r), info),
            //         SdfType::OuterGlow(r) => {
            //             self.shapes_outer_glow_tex_info.insert((hash, r), info)
            //         }
            //     };
            // }

            // log::trace!("text_info=========={:?}, {:?}, {:?}, {:?}", glyph_id, glyphs[glyph_id].glyph, index_position, data_position);
        }
    }
}

use crate::font_brush::ArcEndpoint;
pub fn create_svg_info(binding_box: Aabb, arc_endpoints: Vec<ArcEndpoint>) -> SvgInfo {
    SvgInfo::new(SdfAabb(binding_box), arc_endpoints)
}

#[derive(Debug)]
pub struct AwaitDraw {
    pub char: char,
    pub font_id: FontFaceId,
}

pub struct OnceCellWrap(pub OnceCell<ShareCb>);
unsafe impl Sync for OnceCellWrap {}

pub struct OnceLockWrap(pub OnceLock<ShareCb>);

#[cfg(not(target_arch = "wasm32"))]
static LOAD_CB_SDF: OnceLockWrap = OnceLockWrap(OnceLock::new());

#[cfg(target_arch = "wasm32")]
static LOAD_CB_SDF: OnceCellWrap = OnceCellWrap(OnceCell::new());
// pub static SDF_LOADER: OnceCell<Box<dyn FnMut()>> = OnceCellWrap(OnceCell::new());
lazy_static! {
    pub static ref LOAD_MAP_SDF: Mutex<SlotMap<DefaultKey, AsyncValue<Vec<Vec<u8>>>>> =
        Mutex::new(SlotMap::new());
}

#[cfg(target_arch = "wasm32")]
pub trait Cb: Fn(DefaultKey, usize, &[char]) {}
#[cfg(target_arch = "wasm32")]
impl<T: Fn(DefaultKey, usize, &[char])> Cb for T {}
#[cfg(target_arch = "wasm32")]
pub type ShareCb = std::rc::Rc<dyn Cb>;

#[cfg(not(target_arch = "wasm32"))]
pub trait Cb: Fn(DefaultKey, usize, &[char]) + Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Fn(DefaultKey, usize, &[char]) + Send + Sync> Cb for T {}
#[cfg(not(target_arch = "wasm32"))]
pub type ShareCb = Arc<dyn Cb>;

pub fn init_load_cb(cb: ShareCb) {
    match LOAD_CB_SDF.0.set(cb) {
        Ok(r) => r,
        Err(_e) => panic!("LOAD_CB_SDF.set"),
    };
}

pub fn on_load(key: DefaultKey, data: Vec<Vec<u8>>) {
    let v = LOAD_MAP_SDF.lock().unwrap().remove(key).unwrap();
    v.set(data);
}

pub fn create_async_value(font: &Atom, chars: &[char]) -> AsyncValue<Vec<Vec<u8>>> {
    let mut lock = LOAD_MAP_SDF.lock().unwrap();
    let r = AsyncValue::new();
    let k = lock.insert(r.clone());
    if let Some(cb) = LOAD_CB_SDF.0.get() {
        cb(k, font.str_hash(), chars);
    } else {
    }
    r
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct FontCfg {
//     pub name: String,
//     pub metrics: MetricsInfo,
//     pub glyphs: XHashMap<char, GlyphInfo>,
// }

// // 字符的sdf值
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct CharSdf {
// 	pub unicode: u32,        // 字符的unicode编码
//     pub buffer: Vec<u8>,  // 字符的sdf buffer
// }
