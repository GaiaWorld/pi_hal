//! 字体SDF渲染核心模块
//! 
//! 本模块实现基于有向距离场(SDF)的字体渲染系统，主要功能包括：
//! - 字体资源管理
//! - SDF纹理生成与处理
//! - 阴影/外发光特效支持
//! - 多线程异步渲染管线
//! - GPU加速计算

use crate::font::text_split::text_split;
use crate::font_brush::CellInfo;
use crate::font_brush::LayoutInfo;
use ordered_float::NotNan;
use parry2d::math::Vector;
use parry2d::{bounding_volume::Aabb, math::Point};
use pi_async_rt::prelude::AsyncValue;
use pi_atom::Atom;
use pi_hash::XHashMap;
use pi_null::Null;
use crate::font_brush::OutlineInfo;
use pi_wgpu as wgpu;
use std::char;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
/// 用圆弧曲线模拟字符轮廓，并用于计算距离值的方案
use std::{
    cell::OnceCell,
    collections::{hash_map::Entry, HashMap},
    hash::{DefaultHasher, Hash, Hasher},
    mem::transmute,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
        Arc, Mutex, OnceLock,RwLock
    },
};
/// SDF处理结果存储结构
#[derive(Default, Debug)]
pub struct SdfResultInner {
    /// 字体相关处理结果存储：(字体Key, SDF信息, SDF类型)
    pub font_result: Vec<(DefaultKey, SdfInfo2, SdfType)>,
    pub svg_result: Vec<(u64, Option<SdfInfo2>, SdfType)>,
    pub box_result: Vec<(u64, BoxInfo, Vec<u8>)>,
}

/// 线程安全的SDF处理结果包装
#[derive(Debug, Default, Clone)]
pub struct SdfResult(pub Arc<ShareMutex<SdfResultInner>>);
// use pi_sdf::utils::GlyphInfo;
// use pi_sdf::shape::ArcOutline;
// use crate::font_brush::TexInfo2;
use pi_share::{Share, ShareMutex};
use pi_slotmap::{DefaultKey, SecondaryMap, SlotMap};

use super::blur::compute_box_layout;
use super::blur::BoxInfo;
use super::font::GlyphSheet;
use super::sdf_gpu::GPUState;
use super::text_split::SplitChar;
use super::text_split::SplitChar2;
// use super::sdf_gpu::gpu_draw;
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
    font_brush::{FontFace, SdfInfo2},
    runtime::MULTI_MEDIA_RUNTIME,
    stroe::{self, init_local_store},
    svg::SvgInfo,
};
use pi_async_rt::prelude::AsyncRuntime;
// use pi_async_rt::prelude::serial::AsyncRuntime;

static INTI_STROE_VALUE: Mutex<Vec<AsyncValue<()>>> = Mutex::new(Vec::new());
static SDF_FONT: Mutex<Option<HashMap<String, Vec<u8>>>> = Mutex::new(None);
static GPU: RwLock<Option<GPUState>> = RwLock::new(None);
static INTI_STROE: AtomicBool = AtomicBool::new(false);
static IS_FIRST: AtomicBool = AtomicBool::new(true);
pub static FONT_SIZE: usize = 32;
// pub static PXRANGE: u32 = 7;
// /// 二维装箱
// pub struct Packer2D {

// }

// 此函数决将绘制的字体字号映射为需要的sdf字号（因为一些文字很大时， 小的sdf纹理， 不能满足其精度需求）
pub fn sdf_font_size(_font_size: usize) -> usize {
    FONT_SIZE
}

// 拓展0.5个单位防止采样到边
static EXPAND: f32 = 0.5;
// 超出5，以step步进增加
static STEP: f32 = 3.0;
pub fn compute_px_range(font_info: &FontInfo) -> u32 {
    let pxrange = f32::from(font_info.font.stroke) / font_info.font.font_size as f32 * FONT_SIZE as f32 + EXPAND;
    let pxrange = if pxrange < 5.0 {
         5
    } else {
        (((pxrange - 5.0) / STEP).ceil() * STEP + 5.0).round() as u32
    };
    log::error!("compute_px_range: stroke: {:?}, font_size: {}", font_info.font.stroke, font_info.font.font_size);
    pxrange
}

pub struct Sdf2Table {
    pub fonts: SecondaryMap<DefaultKey, FontFace>, // DefaultKey为FontFaceId
    pub metrics: SecondaryMap<DefaultKey, MetricsInfo>, // DefaultKey为FontFaceId
    pub max_boxs: SecondaryMap<DefaultKey, Aabb>,  // DefaultKey为FontId
    // text_infos: SecondaryMap<DefaultKey, TexInfo>,

    // blob_arcs: Vec<(BlobArc, HashMap<String, u64>)>,
    glyph_id_map: XHashMap<(FontFaceId, u16, u32), GlyphId>,
    pub glyphs: SlotMap<DefaultKey, GlyphIdDesc>,

    pub(crate) index_packer: TextPacker,
    pub data_packer: TextPacker,
    pub outline_info: XHashMap<(DefaultKey, u16, u32), OutlineInfo>,

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

/// SDF类型枚举
#[derive(Debug)]
pub enum SdfType {
    /// 普通SDF
    Normal,
    Shadow(u32, NotNan<f32>),
    OuterGlow(u32),
}

/// SVG纹理信息结构
#[derive(Debug, Clone, Default)]
pub struct SvgTexInfo {
    /// 纹理坐标X
    pub x: f32,
    pub y: f32,
    pub width: usize,
    pub height: usize,
    layout: LayoutInfo,
}

impl Sdf2Table {
    pub fn new(width: usize, height: usize, device: Share<wgpu::Device>, queue: Share<wgpu::Queue>,) -> Self {
        if !INTI_STROE.load(Ordering::Relaxed) && IS_FIRST.load(Ordering::Relaxed) {
            IS_FIRST.store(false, Ordering::Relaxed);
            // pi_app版本不能放到多线程运行时里调用pi_wgpu，否则会报错，所以放到主线程
            // 跨平台注意事项：在非WebAssembly平台初始化GPU状态
            #[cfg(all(not(target_arch = "wasm32"), not(feature = "empty")))]
            {
                *GPU.write().unwrap() = Some(GPUState::init(device, queue));
            }
            
            let _ = MULTI_MEDIA_RUNTIME.spawn(async move {
                if let Some(buf) = init_local_store().await {
                    let mut map: HashMap<String, Vec<u8>> = HashMap::new();
                    if !buf.is_empty(){
                        map = bitcode::deserialize(&buf).unwrap();
                    }
                    *SDF_FONT.lock().unwrap() = Some(map);
                }

                // log::warn!("init_local_store end");
                INTI_STROE.store(true, Ordering::Relaxed);
                // for v in INTI_STROE_VALUE.lock().unwrap().drain(..) {
                //     v.set(());
                // } 
                // wasm版本 单线程放到异步运行时去初始化
                #[cfg(all(target_arch = "wasm32", not(feature = "empty")))]
                {
                    *GPU.write().unwrap() = Some(GPUState::init(device, queue));
                }
                
            });
        }

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
                distance_range: 0.0 as f32,
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
        self.max_boxs.insert(
            font_id.0,
            Aabb::new(
                Point::new(max_box[0], max_box[1]),
                Point::new(max_box[2], max_box[3]),
            ),
        );
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
        if let Some(glyph_id) = self.glyph_id(font_id, font, char) {
            if glyph_id.0.is_null(){
                return (font.font.font_size as f32 * 0.5, glyph_id,);
            }
            if self.glyphs[glyph_id.0].font_face_index.is_null() {
                for (index, font_id) in font.font_ids.iter().enumerate() {
                    if let Some(r) = self.fonts.get_mut(font_id.0) {
                        let horizontal_advance = r.horizontal_advance(self.glyphs[glyph_id.0].char);
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
        }

        return (0.0, GlyphId(DefaultKey::null()));
    }

    // 文字宽度
    pub fn width_of_glyph_id(&mut self, font_id: FontId, font: &mut FontInfo, glyph_id: GlyphId) -> f32 {
        if glyph_id.0.is_null(){
            return  0.5 * font.font.font_size as f32;
        }
        let glyph = &mut self.glyphs[glyph_id.0];
        if glyph.font_face_index.is_null() {
            for (index, font_id) in font.font_ids.iter().enumerate() {
                if let Some(r) = self.fonts.get_mut(font_id.0) {
                    
                    let horizontal_advance = r.horizontal_advance_of_glyph_index(glyph.glyph_index);
                    // println!("======= {} width: {}", glyph.char, horizontal_advance);
                    if horizontal_advance >= 0.0 {
                        glyph.font_face_index = index;
                        log::debug!(
                            "mesure char width first, char: {}, font_id: {:?}, width: {:?}",
                            glyph.char,
                            font_id,
                            (
                                horizontal_advance,
                                font.font.font_size as f32,
                                horizontal_advance * font.font.font_size as f32
                            )
                        );
                        self.glyphs[glyph_id.0].glyph.advance = horizontal_advance;
                        return horizontal_advance * font.font.font_size as f32;
                    }
                };
            }
        }
        return  self.glyphs[glyph_id.0].glyph.advance * font.font.font_size as f32;
    }

    pub fn glyph_id_desc(&self, glyph_id: GlyphId) -> &GlyphIdDesc {
        &self.glyphs[glyph_id.0]
    }

    // 字形id
    pub fn glyph_id(
        &mut self,
        font_id: FontId,
        font_info: &mut FontInfo,
        char: char,
    ) -> Option<GlyphId> {
        // log::error!("glyph_id: {:?}",(&font_id, char));
        for (_index, font_face_id) in font_info.font_ids.iter().enumerate() {
            if let Some(font_face) = self.fonts.get_mut(font_face_id.0) {
                let mut glyph_index = font_face.glyph_index(char);
                let mut char = char;
                if glyph_index == 0 {
                    log::warn!("{:?} is not have {}", font_info.font.font_family_string.as_str(), char);
                }
                if glyph_index == 0 {
                    char = '□';
                    glyph_index = font_face.glyph_index('□');
                }
                if glyph_index == 0 {
                    char = ' ';
                    glyph_index = font_face.glyph_index(' ');
                }

                // 字体中存在字符
                if glyph_index > 0 {
                    let pxrange = compute_px_range(font_info);
                    log::error!("compute_px_range: char: {}, font_size: {}, stroke_width: {}", char, font_info.font.font_size, font_info.font.stroke);
                    match self.glyph_id_map.entry((*font_face_id, glyph_index, pxrange)) {
                        Entry::Occupied(r) => {
                            let id = r.get().clone();
                            return Some(id);
                        }
                        Entry::Vacant(r) => {
                            // 分配GlyphId
                            let id = GlyphId(self.glyphs.insert(GlyphIdDesc {
                                font_id,
                                char,
                                glyph_index,
                                font_face_index: pi_null::Null::null(),
                                glyph: Glyph::default(),
                            }));

                            // log::error!("glyph_id===============: {:p}", (font_id, char, id));

                            // if self.glyphs[id.0].font_face_index.is_null() {

                            let outline_info = font_face.to_outline(char);

                            let LayoutInfo {
                                atlas_bounds,
                                tex_size,
                                ..
                            } = outline_info.compute_layout(FONT_SIZE, pxrange, pxrange);
                            let offset = self
                                .index_packer
                                .alloc(tex_size as usize, tex_size as usize)?;
                            let bbox = Aabb::new(
                                Point::new(outline_info.bbox[0], outline_info.bbox[1]),
                                Point::new(outline_info.bbox[2], outline_info.bbox[3]),
                            );
                            let plane_bounds = bbox.scaled(&Vector::new(
                                1.0 / outline_info.units_per_em as f32,
                                1.0 / outline_info.units_per_em as f32,
                            ));

                            let glyph = Glyph {
                                plane_min_x: plane_bounds.mins.x,
                                plane_min_y: plane_bounds.mins.y,
                                plane_max_x: plane_bounds.maxs.x,
                                plane_max_y: plane_bounds.maxs.y,
                                x: offset.x as f32 + atlas_bounds[0],
                                y: offset.y as f32 + atlas_bounds[1],
                                width: atlas_bounds[2] - atlas_bounds[0],
                                height: atlas_bounds[3] - atlas_bounds[1],
                                advance: outline_info.advance as f32,
                            };
                            self.glyphs[id.0].glyph = glyph;

                            self.outline_info
                                .insert((font_face_id.0, glyph_index, pxrange), outline_info);

                            if !char.is_whitespace() {
                                // 不是空白符， 才需要放入等待队列
                                // log::error!("================ glyph_id: {:?}",( font_info.await_info.wait_list.len(), self.glyphs[id.0].char, char, id));
                                font_info.await_info.wait_list.push(id);
                            }
                            let _ = r.insert(id).clone();
                            return Some(id);
                        }
                    };
                } else {
                    log::warn!("{:?} is not have ' ' or '□'", font_info.font.font_family_string.as_str());
                    return Some(GlyphId(DefaultKey::null()));
                }
            }
        }
        None
    }

    // 字形id
    pub fn glyph_indexs(
        &mut self,
        font_id: FontId,
        font_info: &mut FontInfo,
        text: &str,
        is_reverse: bool
    ) -> (String,Vec<Option<GlyphId>> ){
        log::debug!("glyph_indexs: {:?}",(&font_id, text));
        let mut glyph_ids = vec![None; text.len()];
        let mut str = text.to_string();
        // if 
        let mut is_loop = true;
        for (_index, font_face_id) in font_info.font_ids.iter().enumerate() {
            if !is_loop {
                break;
            }
            if let Some(font_face) = self.fonts.get_mut(font_face_id.0) {
                log::debug!("========= text_split: {}, is_reverse: {}", text, is_reverse);
                str = text_split(text, is_reverse);
                
                let glyph_indexs = font_face.glyph_indexs(&str, 0);
                log::debug!("========= glyph_indexs2: {:?}, is_reverse: {}", glyph_indexs, str);
                assert_eq!(glyph_indexs.len(), text.chars().count());
                let mut index = 0;
                for (mut glyph_index, mut char) in glyph_indexs.into_iter().zip(text.chars()){
                    log::debug!("========= glyph_index: {}, char: {}", glyph_index, char);
                    if glyph_index == 0 {
                        char = '□';
                        glyph_index = font_face.glyph_index('□');
                    }
                    if glyph_index == 0 {
                        char = ' ';
                        glyph_index = font_face.glyph_index(' ');
                    }
                    // 字体中存在字符
                    if glyph_index > 0 {
                        let pxrange = compute_px_range(font_info);
                        log::error!("compute_px_range: char: {}, font_size: {}, stroke_width: {}", char, font_info.font.font_size, font_info.font.stroke);
                        match self.glyph_id_map.entry((*font_face_id, glyph_index, pxrange)) {
                            Entry::Occupied(r) => {
                                let id = r.get().clone();
                                glyph_ids[index] = Some(id);
                                if is_loop {
                                    is_loop = false;
                                }
                            }
                            Entry::Vacant(r) => {
                                // 分配GlyphId
                                let id = GlyphId(self.glyphs.insert(GlyphIdDesc {
                                    font_id,
                                    char,
                                    glyph_index,
                                    font_face_index: pi_null::Null::null(),
                                    glyph: Glyph::default(),
                                }));

                                // log::error!("glyph_id===============: {:p}", (font_id, char, id));

                                // if self.glyphs[id.0].font_face_index.is_null() {

                                let outline_info = font_face.to_outline_of_glyph_index(glyph_index);

                                let LayoutInfo {
                                    atlas_bounds,
                                    tex_size,
                                    ..
                                } = outline_info.compute_layout(FONT_SIZE, pxrange, pxrange);
                                let offset = if let Some(r) = self
                                    .index_packer
                                    .alloc(tex_size as usize, tex_size as usize){
                                        r
                                    }else{
                                        continue;
                                    };
                                let bbox = Aabb::new(
                                    Point::new(outline_info.bbox[0], outline_info.bbox[1]),
                                    Point::new(outline_info.bbox[2], outline_info.bbox[3]),
                                );
                                let plane_bounds = bbox.scaled(&Vector::new(
                                    1.0 / outline_info.units_per_em as f32,
                                    1.0 / outline_info.units_per_em as f32,
                                ));

                                let glyph = Glyph {
                                    plane_min_x: plane_bounds.mins.x,
                                    plane_min_y: plane_bounds.mins.y,
                                    plane_max_x: plane_bounds.maxs.x,
                                    plane_max_y: plane_bounds.maxs.y,
                                    x: offset.x as f32 + atlas_bounds[0],
                                    y: offset.y as f32 + atlas_bounds[1],
                                    width: atlas_bounds[2] - atlas_bounds[0],
                                    height: atlas_bounds[3] - atlas_bounds[1],
                                    advance: outline_info.advance as f32,
                                };
                                self.glyphs[id.0].glyph = glyph;

                                self.outline_info
                                    .insert((font_face_id.0, glyph_index, pxrange), outline_info);

                                if !char.is_whitespace() {
                                    // 不是空白符， 才需要放入等待队列
                                    // log::error!("================ glyph_id: {:?}",( font_info.await_info.wait_list.len(), self.glyphs[id.0].char, char, id));
                                    font_info.await_info.wait_list.push(id);
                                }
                                let _ = r.insert(id).clone();
                                glyph_ids[index] = Some(id);
                                if is_loop {
                                    is_loop = false;
                                }
                            }
                        };
                    } else {
                        glyph_ids[index] = Some(GlyphId(DefaultKey::null()));
                    }
                    index += 1;
                }
            }
        }
        (str, glyph_ids)
    }

    pub fn split<'a>(&mut self, font_id: FontId, font_info: &mut FontInfo, text: &'a str, word_split: bool, merge_whitespace: bool, is_reverse: bool) -> SplitChar2<'a>{
        let (text2, glyph_ids) = self.glyph_indexs(font_id, font_info, text, is_reverse);
        let mut i = text.chars();
        let last = i.next();
        SplitChar2 {
            text: text2.chars().collect::<Vec<char>>(),
            glyph_ids,
            cur_index: 0,
            iter: i,
            word_split,
            merge_whitespace: merge_whitespace,
            last: last,
            type_id: 0,
        }
    }

    // 字形id
    pub fn add_font_shadow(
        &mut self,
        id: GlyphId,
        font_info: &FontInfo,
        radius: u32,
        weight: NotNan<f32>,
    ) {
        // let id =  self.glyph_id_map.get((font_info.font_family_id, char));
        if let Entry::Vacant(vacant_entry) = self.font_shadow_info.entry((id, radius, weight)) {
            let c = &self.glyphs[id.0];
            let font_face_id = font_info.font_ids[c.font_face_index];
            println!("add_font_shadow ============={:?}", (c.font_id.0, c.char));
            let pxrange = compute_px_range(font_info);
            let outline_info = self.outline_info.get(&(font_face_id.0, c.glyph_index, pxrange)).unwrap();

            let LayoutInfo {
                atlas_bounds,
                tex_size,
                ..
            } = outline_info.compute_layout(
                FONT_SIZE,
                pxrange,
                (radius as f32 + f32::from(weight) * 3.0) as u32 + 2,
            );
            let offset = self
                .index_packer
                .alloc(tex_size as usize, tex_size as usize)
                .unwrap();
            let bbox = Aabb::new(
                Point::new(outline_info.bbox[0], outline_info.bbox[1]),
                Point::new(outline_info.bbox[2], outline_info.bbox[3]),
            );
            let plane_bounds = bbox.scaled(&Vector::new(
                1.0 / outline_info.units_per_em as f32,
                1.0 / outline_info.units_per_em as f32,
            ));
            let glyph = Glyph {
                plane_min_x: plane_bounds.mins.x,
                plane_min_y: plane_bounds.mins.y,
                plane_max_x: plane_bounds.maxs.x,
                plane_max_y: plane_bounds.maxs.y,
                x: offset.x as f32 + atlas_bounds[0],
                y: offset.y as f32 + atlas_bounds[1],
                width: atlas_bounds[2] - atlas_bounds[0],
                height: atlas_bounds[3] - atlas_bounds[1],
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
    pub fn add_font_outer_glow(&mut self, id: GlyphId, font_info: &FontInfo, range: u32) {
        if let Entry::Vacant(vacant_entry) = self.font_outer_glow_info.entry((id, range)) {
            println!("add_font_outer_glow======={:?}", range);
            let c = &self.glyphs[id.0];
            let font_face_id = font_info.font_ids[c.font_face_index];
            let pxrange = compute_px_range(font_info);
            let outline_info = self.outline_info.get(&(font_face_id.0, c.glyph_index, pxrange)).unwrap();

            let LayoutInfo {
                atlas_bounds,
                tex_size,
                ..
            } = outline_info.compute_layout(FONT_SIZE, range, range);
            let bbox = Aabb::new(
                Point::new(outline_info.bbox[0], outline_info.bbox[1]),
                Point::new(outline_info.bbox[2], outline_info.bbox[3]),
            );
            let plane_bounds = bbox.scaled(&Vector::new(
                1.0 / outline_info.units_per_em as f32,
                1.0 / outline_info.units_per_em as f32,
            ));
            let offset = self
                .index_packer
                .alloc(tex_size as usize, tex_size as usize)
                .unwrap();

            let glyph = Glyph {
                plane_min_x: plane_bounds.mins.x,
                plane_min_y: plane_bounds.mins.y,
                plane_max_x: plane_bounds.maxs.x,
                plane_max_y: plane_bounds.maxs.y,
                x: offset.x as f32 + atlas_bounds[0],
                y: offset.y as f32 + atlas_bounds[1],
                width: atlas_bounds[2] - atlas_bounds[0],
                height: atlas_bounds[3] - atlas_bounds[1],
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
                layout: LayoutInfo::default(),
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
        log::debug!("======== add_shape");
        if self.shapes_tex_info.get(&hash).is_some(){
            log::debug!("======== add_shape: is have!!1");
            return;
        }
        let info2 = info.compute_layout(tex_size, pxrang, cut_off);

        let index_position = self
            .index_packer
            .alloc(info2.tex_size as usize, info2.tex_size as usize)
            .unwrap();

        self.shapes_tex_info.insert(
            hash,
            SvgTexInfo {
                x: index_position.x as f32 + info2.atlas_bounds[0],
                y: index_position.y as f32 + info2.atlas_bounds[1],
                width: (info2.atlas_bounds[2] - info2.atlas_bounds[0]) as usize,
                height: (info2.atlas_bounds[3] - info2.atlas_bounds[1]) as usize,
                layout: info2,
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
                    .alloc(info.tex_size as usize, info.tex_size as usize)
                    .unwrap();
                self.shapes_shadow_tex_info.insert(
                    (id, radius),
                    SvgTexInfo {
                        x: index_position.x as f32 + info.atlas_bounds[0],
                        y: index_position.y as f32 + info.atlas_bounds[1],
                        width: (info.atlas_bounds[2] - info.atlas_bounds[0]) as usize,
                        height: (info.atlas_bounds[3] - info.atlas_bounds[1]) as usize,
                        layout: info,
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
                    .alloc(info.tex_size as usize, info.tex_size as usize)
                    .unwrap();
                self.shapes_outer_glow_tex_info.insert(
                    (id, radius),
                    SvgTexInfo {
                        x: index_position.x as f32 + info.atlas_bounds[0],
                        y: index_position.y as f32 + info.atlas_bounds[1],
                        width: (info.atlas_bounds[2] - info.atlas_bounds[0]) as usize,
                        height: (info.atlas_bounds[3] - info.atlas_bounds[1]) as usize,
                        layout: info,
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
        if self.glyphs.get(id.0).is_none() {
            panic!("glyph is not exist, {:?}", id);
        }
        &self.glyphs[id.0].glyph
    }

    /// 更新字形信息（计算圆弧信息）
    pub fn draw_await(
        &mut self,
        texture: Share<wgpu::Texture>,
        sheet: &mut GlyphSheet,
        index: usize,
        result: SdfResult,
        await_count: usize,
    ) -> AsyncValue<()> {
        let async_value = AsyncValue::new();
        if await_count == 0 {
            let async_value1 = async_value.clone();
            let _ = MULTI_MEDIA_RUNTIME.spawn(async move {
                async_value1.set(());
            });
            return async_value;
        }

        let task_count = Arc::new(AtomicUsize::new(await_count));
        self.draw_font_await(
            sheet,
            result.clone(),
            index,
            task_count.clone(),
            async_value.clone(),
        );
        self.draw_svg_await(
            texture,
            result.clone(),
            task_count.clone(),
            async_value.clone(),
        );
        self.draw_box_shadow_await(result.clone(), task_count.clone(), async_value.clone());

        async_value
    }
    pub fn draw_count(&self, fonts: &SlotMap<DefaultKey, FontInfo>) -> usize {
        let mut count = 0;
        // 文字数量
        for (_, font_info) in fonts.iter() {
            count += font_info.await_info.wait_list.len();
        }
        count += self.shapes.len();
        count += self.bboxs.len();
        if count > 0 {
            log::info!(
                "task_count=========={:?}",
                (
                    count - self.bboxs.len() - self.shapes.len(),
                    self.shapes.len(),
                    self.bboxs.len(),
                )
            );
        }
        count
    }

    /// 更新字形信息（计算圆弧信息）
    pub fn draw_font_await(
        &mut self,
        sheet: &mut GlyphSheet,
        result: SdfResult,
        index: usize,
        await_count: Arc<AtomicUsize>,
        async_value: AsyncValue<()>,
    ) {
        // 轮廓信息（贝塞尔曲线）
        let mut outline_infos = Vec::with_capacity(await_count.load(Ordering::Relaxed));
        let mut chars = Vec::new();
        let mut keys = Vec::new();

        // 遍历所有的等待文字， 取到文字的贝塞尔曲线描述
        if await_count.load(Ordering::Relaxed) != 0 {
            for (_, font_info) in sheet.fonts.iter_mut() {
                let pxrange = compute_px_range(font_info);
                let await_info = &mut font_info.await_info;
                if await_info.wait_list.len() == 0 {
                    continue;
                }

                for glyph_id in await_info.wait_list.drain(..) {
                    let g = &self.glyphs[glyph_id.0];
                    // font_face_index不存在， 不需要计算
                    if g.font_face_index.is_null() {
                        log::warn!("font_face_index null=============");
                        await_count.fetch_sub(1, Ordering::Relaxed);
                        continue;
                    }
                    let font_face_id = font_info.font_ids[g.font_face_index];
                    if let Some(font_face) = self.fonts.get_mut(font_face_id.0) {
                        let glyph_index = g.glyph_index;

                        // 字体中不存在字符
                        if glyph_index == 0 {
                            await_count.fetch_sub(1, Ordering::Relaxed);
                            continue;
                        }
                        let is_outer_glow = self.font_outer_glow.remove(&glyph_id);
                        let shadow = self.font_shadow.remove(&glyph_id);

                        let font_name = &sheet.font_names[font_face_id.0];
                        outline_infos.push((
                            self.outline_info.remove(&(font_face_id.0, g.glyph_index, pxrange)).expect(&format!("font_face_id.0: {:?}, g.char: {}, glyph_id: {:?}, self: {:?}, not in outline_info!!!", font_face_id.0, g.char, glyph_id, 1)),
                            font_face_id.0,
                            glyph_id,
                            is_outer_glow,
                            shadow,
                            pxrange
                        )); // 先取到贝塞尔曲线
                        keys.push(format!("{}{}", g.char, font_name));
                        chars.push(g.char)
                    }
                }
            }
        }

        result.0.lock().unwrap().font_result =
            Vec::with_capacity(await_count.load(Ordering::Relaxed));

        let mut ll = 0;

        // 遍历所有等待处理的字符贝塞尔曲线，将曲线转化为圆弧描述（多线程）
        let mut index = 0;
        for glyph_visitor in outline_infos.drain(..) {
            let async_value1 = async_value.clone();
            let result = result.clone();
            // println!("encode_data_tex===={:?}", index);
            let await_count = await_count.clone();
            let char = chars[index];
            index += 1;
            let key = keys[ll].clone();
            MULTI_MEDIA_RUNTIME
                .spawn(async move {
                    let mut crach_info = None;
                    {
                        let mut sdf_map = SDF_FONT.lock().unwrap();
                        if sdf_map.is_some()
                            && let Some(buffer) = sdf_map.as_mut().unwrap().remove(&key)
                        {
                            #[cfg(all(not(target_arch = "wasm32"), not(feature = "empty")))]
                            {
                                crach_info =
                                    Some(bitcode::deserialize::<CellInfo>(&buffer[..]).unwrap());
                            }

                            #[cfg(all(target_arch = "wasm32", not(feature = "empty")))]
                            {
                                crach_info = Some(buffer);
                            }
                        }
                    }

                    let result_arcs = if let Some(info) = crach_info {
                        info
                    } else if let Some(buffer) = stroe::get(key.clone()).await {
                        #[cfg(all(not(target_arch = "wasm32"), not(feature = "empty")))]
                        {
                            bitcode::deserialize::<CellInfo>(&buffer[..]).unwrap()
                        }

                        #[cfg(all(target_arch = "wasm32", not(feature = "empty")))]
                        buffer
                    } else {
                        #[cfg(all(not(target_arch = "wasm32"), not(feature = "empty")))]
                        {
                            let arcs = glyph_visitor.0.compute_near_arcs(2.0);
                            let buffer = bitcode::serialize(&arcs).unwrap();
                            stroe::write(key, buffer).await;
                            arcs
                        }

                        #[cfg(all(target_arch = "wasm32", not(feature = "empty")))]
                        {
                            let buffer = glyph_visitor.0.compute_near_arcs(1.0).await;
                            stroe::write(key, buffer.clone()).await;
                            buffer
                        }
                    };
                    // log::error!("computer char {}, time: {:?}. glyph_id: {:?}", char, time.elapsed(), glyph_visitor.2);
                    let lock = &mut result.0.lock().unwrap().font_result;
                    let mut sdf = glyph_visitor.0.compute_sdf_tex(
                        result_arcs.clone(),
                        FONT_SIZE,
                        glyph_visitor.5,
                        false,
                        glyph_visitor.5,
                    );
                    sdf.tex_info.char = char;
                    // log::error!("char: {:?}", sdf.tex_info);
                    lock.push((glyph_visitor.2 .0, sdf, SdfType::Normal));

                    if let Some(outer_ranges) = glyph_visitor.3 {
                        for v in outer_ranges {
                            let outer_glow_sdf = glyph_visitor.0.compute_sdf_tex(
                                result_arcs.clone(),
                                FONT_SIZE,
                                v,
                                false,
                                v,
                            );
                            lock.push((glyph_visitor.2 .0, outer_glow_sdf, SdfType::OuterGlow(v)));
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
                                glyph_visitor.5,
                                false,
                                (shadow_range as f32 + f32::from(weight) * 3.0) as u32 + 2,
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

                    // log::trace!("encode_data_tex======cur_count: {:?}, atlas_bounds={:?}, await_count={:?}, tex_size={:?}", lock.0, atlas_bounds, await_count, tex_size);
                    let r = await_count.fetch_sub(1, Ordering::Relaxed);
                    // log::warn!("r1============{:?}", r);
                    if r == 1 {
                        log::trace!("encode_data_tex1");
                        async_value1.set(());
                    }
                })
                .unwrap();
            ll += 1;
        }
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
        let glyphs = &mut self.glyphs;

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
            let sdf_img = FontImage {
                width: tex_size as usize,
                height: tex_size as usize,
                buffer: sdf_tex,
            };
            let glyph = match sdf_type {
                SdfType::Normal => &glyphs[glyph_id].glyph,
                SdfType::Shadow(radius, weight) => self
                    .font_shadow_info
                    .get(&(GlyphId(glyph_id), radius, weight))
                    .unwrap(),
                SdfType::OuterGlow(radius) => self
                    .font_outer_glow_info
                    .get(&(GlyphId(glyph_id), radius))
                    .unwrap(),
            };

            let sdf_block = Block {
                x: glyph.x - tex_info.atlas_min_x as f32,
                y: glyph.y - tex_info.atlas_min_y as f32,
                width: tex_size as f32,
                height: tex_size as f32,
            };
            // log::warn!("update index tex========={:?}", (&index_block,index_img.width, index_img.height, index_img.buffer.len(), &text_info) );
            (update.clone())(sdf_block, sdf_img);
        }
    }

    pub fn has_box_shadow(&mut self, hash: u64) -> bool {
        self.shapes_tex_info.get(&hash).is_some()
    }

    /// 更新字形信息（计算圆弧信息）
    pub fn draw_box_shadow_await(
        &mut self,
        result: SdfResult,
        await_count: Arc<AtomicUsize>,
        async_value: AsyncValue<()>,
    ) {
        // let async_value = AsyncValue::new();
        let mut bboxs = self.bboxs.clone();
        self.bboxs.clear();

        // 遍历所有等待处理的字符贝塞尔曲线，将曲线转化为圆弧描述（多线程）
        for (hash, box_info) in bboxs.drain() {
            let async_value1 = async_value.clone();
            let result1 = result.clone();
            let await_count = await_count.clone();
            MULTI_MEDIA_RUNTIME
                .spawn(async move {
                    let sdfinfo = blur_box(box_info.clone());

                    // log::debug!("load========={:?}, {:?}", lock.0, len);
                    let lock = &mut result1.0.lock().unwrap().box_result;
                    // log::trace!("encode_data_tex======cur_count: {:?}, grid_size={:?}, await_count={:?}, text_info={:?}", lock.0, await_count);
                    lock.push((hash, box_info, sdfinfo));
                    if await_count.fetch_sub(1, Ordering::Relaxed) == 1 {
                        log::trace!("encode_data_tex1");
                        async_value1.set(());
                        log::trace!("encode_data_tex2");
                    }
                })
                .unwrap();
        }
    }

    pub fn update_box_shadow<F: FnMut(Block, FontImage) + Clone + 'static>(
        &mut self,
        update: F,
        result: &mut Vec<(u64, BoxInfo, Vec<u8>)>,
    ) {
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
                x: index_position.x - box_info.atlas_bounds.mins.x as f32,
                y: index_position.y - box_info.atlas_bounds.mins.y as f32,
                width: index_img.width as f32,
                height: index_img.height as f32,
            };
            // log::warn!("update index tex========={:?}", (&index_block,index_img.width, index_img.height, index_img.buffer.len(), &text_info) );
            (update.clone())(index_block, index_img);
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
        texture: Share<wgpu::Texture>,
        result: SdfResult,
        await_count: Arc<AtomicUsize>,
        async_value: AsyncValue<()>,
    ) {
        let mut shapes = self.shapes.clone();
        self.shapes.clear();
        let mut shapes_outer_glow = self.shapes_outer_glow.clone();
        self.shapes_outer_glow.clear();
        let mut shapes_shadow = self.shapes_shadow.clone();
        self.shapes_shadow.clear();
        let shapes_tex_info = self.shapes_tex_info.clone();

        // 遍历所有等待处理的字符贝塞尔曲线，将曲线转化为圆弧描述（多线程）
        for (hash, (info, size, pxrange, cur_off)) in shapes.drain() {
            let outer_glow = shapes_outer_glow.remove(&hash);
            let shadow = shapes_shadow.remove(&hash);

            let async_value1 = async_value.clone();
            let result1 = result.clone();
            let await_count = await_count.clone();
            let gpu = GPU.read().unwrap();
            if size > 256 && gpu.is_some() {
                let index_position = shapes_tex_info.get(&hash).unwrap().clone();
                let tex_offset = (
                    (index_position.x - index_position.layout.atlas_bounds[0]) as u32,
                    (index_position.y - index_position.layout.atlas_bounds[1]) as u32,
                );
                let scale = 1.0;
                gpu.as_ref().unwrap().draw(
                    &texture,
                    info,
                    tex_offset,
                    size as u32,
                    pxrange as f32,
                    cur_off,
                    scale
                );
                {
                    let lock = &mut result1.0.lock().unwrap().svg_result;
                    lock.push((hash, None, SdfType::Normal));
                }
                
                if await_count.fetch_sub(1, Ordering::Relaxed) == 1 {
                    log::trace!("encode_data_tex1");
                    async_value1.set(());
                    log::trace!("encode_data_tex2");
                }
            } else {
                MULTI_MEDIA_RUNTIME
                    .spawn(async move {
                        #[cfg(all(not(target_arch = "wasm32"), not(feature = "empty")))]
                        let sdfinfo =
                            Some(info.compute_sdf_tex(size, pxrange, false, cur_off, 1.0));

                        #[cfg(all(target_arch = "wasm32", not(feature = "empty")))]
                        let sdfinfo = Some(info
                            .compute_sdf_tex(size, pxrange, false, cur_off, 1.0)
                            .await);

                        {
                            let lock = &mut result1.0.lock().unwrap().svg_result;
                            lock.push((hash, sdfinfo.clone(), SdfType::Normal));
                        }
                        if let Some(outer_glow) = outer_glow {
                            for v in outer_glow {
                                // let sdfinfo = info.compute_sdf_tex( size, v, true, v / 2, 1.0).await;
                                #[cfg(all(not(target_arch = "wasm32"), not(feature = "empty")))]
                                let sdfinfo = info.compute_sdf_tex(size, v, true, v / 2, 1.0);
                                #[cfg(all(target_arch = "wasm32", not(feature = "empty")))]
                                let sdfinfo = info.compute_sdf_tex(size, v, true, v / 2, 1.0).await;
                                let lock2 = &mut result1.0.lock().unwrap().svg_result;
                                lock2.push((hash, Some(sdfinfo), SdfType::OuterGlow(v)));
                            }
                        }

                        if let Some(shadow) = shadow {
                            if let Some(SdfInfo2 {
                                tex_info,
                                sdf_tex,
                                tex_size,
                            }) = sdfinfo
                            {
                                let lock1 = &mut result1.0.lock().unwrap().svg_result;
                                for shadow_range in shadow {
                                    let sdf_tex = gaussian_blur(
                                        sdf_tex.clone(),
                                        tex_size as u32,
                                        tex_size as u32,
                                        shadow_range,
                                        0.0,
                                    );
                                    lock1.push((
                                        hash,
                                        Some(SdfInfo2 {
                                            tex_info: tex_info.clone(),
                                            sdf_tex,
                                            tex_size,
                                        }),
                                        SdfType::Shadow(shadow_range, NotNan::new(0.0).unwrap()),
                                    ));
                                }
                            }
                        }

                        if await_count.fetch_sub(1, Ordering::Relaxed) == 1 {
                            log::trace!("encode_data_tex1");
                            async_value1.set(());
                            log::trace!("encode_data_tex2");
                        }
                    })
                    .unwrap();
            }
        }
    }

    /// 提前同步计算svg
    pub fn advance_computer_svg(
        &mut self,
        result: SdfResult,
    ) {
        let mut shapes = self.shapes.clone();
        self.shapes.clear();

        // 遍历所有等待处理的字符贝塞尔曲线，将曲线转化为圆弧描述（多线程）
        for (hash, (info, size, pxrange, cur_off)) in shapes.drain() {

            #[cfg(all(not(target_arch = "wasm32"), not(feature = "empty")))]
            let sdfinfo =
                Some(info.compute_sdf_tex(size, pxrange, false, cur_off, 1.0));

            #[cfg(all(target_arch = "wasm32", not(feature = "empty")))]
            let sdfinfo = Some(info
                .compute_sdf_tex_sync(size, pxrange, false, cur_off, 1.0));
            {
                let lock = &mut result.0.lock().unwrap().svg_result;
                lock.push((hash, sdfinfo.clone(), SdfType::Normal));
            }
        }
    }

    pub fn update_svg<F: FnMut(Block, FontImage) + Clone + 'static>(
        &mut self,
        update: F,
        result: &mut Vec<(u64, Option<SdfInfo2>, SdfType)>,
    ) {
        // let mut lock = result.lock().unwrap();
        let r = result;
        log::debug!("sdf2 load2========={:?}", r.len());

        while let Some((hash, info, svg_type)) = r.pop() {
            if let Some(SdfInfo2 {
                tex_info,
                sdf_tex,
                tex_size,
            }) = info
            {
                let index_position = self.shapes_tex_info.get(&hash).unwrap();

                let index_img = FontImage {
                    width: tex_size as usize,
                    height: tex_size as usize,
                    buffer: sdf_tex,
                };
                // tex_info.sdf_offset_x = index_position.x;
                // tex_info.sdf_offset_x = index_position.y;
                let index_block = Block {
                    x: index_position.x - tex_info.atlas_min_x as f32,
                    y: index_position.y - tex_info.atlas_min_y as f32,
                    width: index_img.width as f32,
                    height: index_img.height as f32,
                };
                // log::warn!("update index tex========={:?}", (&index_block,index_img.width, index_img.height, index_img.buffer.len(), &text_info) );
                (update.clone())(index_block, index_img);
            }
        }
    }
}

// use crate::font_brush::ArcEndpoint;
// pub fn create_svg_info(binding_box: Aabb, arc_endpoints: Vec<ArcEndpoint>) -> SvgInfo {
//     SvgInfo::new_from_arc_endpoint(SdfAabb(binding_box), arc_endpoints)
// }

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
pub trait Cb: Fn(DefaultKey, f64, &[char]) {}
#[cfg(target_arch = "wasm32")]
impl<T: Fn(DefaultKey, f64, &[char])> Cb for T {}
#[cfg(target_arch = "wasm32")]
pub type ShareCb = std::rc::Rc<dyn Cb>;

#[cfg(not(target_arch = "wasm32"))]
pub trait Cb: Fn(DefaultKey, f64, &[char]) + Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Fn(DefaultKey, f64, &[char]) + Send + Sync> Cb for T {}
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
        cb(k, unsafe { transmute::<_, f64>(font.str_hash()) }, chars);
    } else {
    }
    r
}
