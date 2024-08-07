/// 用圆弧曲线模拟字符轮廓， 并用于计算距离值的方案

use std::{cell::OnceCell, collections::{hash_map::Entry, HashMap}, hash::{DefaultHasher, Hash, Hasher}, mem::transmute, sync::{atomic::{AtomicBool, Ordering}, mpsc::channel, Arc, Mutex, OnceLock}};

use parry2d::{bounding_volume::Aabb, math::Point};
use pi_async_rt::prelude::AsyncValueNonBlocking as AsyncValue;
use pi_atom::Atom;
use pi_hash::XHashMap;
use pi_null::Null;
// use pi_sdf::shape::ArcOutline;
use pi_share::{ShareMutex, Share};
use pi_slotmap::{SecondaryMap, DefaultKey, SlotMap};

use super::{font::{FontId, Block, FontImage, FontInfo, FontFaceId, GlyphId, Size, FontFamilyId}, text_pack::TextPacker};

use crate::{font::font::ShadowImage, font_brush::{FontFace, SdfInfo, load_font_sdf}, svg::{SvgInfo, computer_svg_sdf}, runtime::MULTI_MEDIA_RUNTIME, stroe::{self, init_local_store}};
use pi_async_rt::prelude::AsyncRuntime;
pub use crate::font_brush::TexInfo;

static INTI_STROE_VALUE: Mutex<Vec<AsyncValue<()>>> = Mutex::new(Vec::new());
static INTI_STROE: AtomicBool = AtomicBool::new(false);
// /// 二维装箱
// pub struct Packer2D {

// }

pub struct Sdf2Table {
	pub fonts: SecondaryMap<DefaultKey, FontFace>, // DefaultKey为FontId
	pub max_boxs: SecondaryMap<DefaultKey, Aabb>, // DefaultKey为FontId
	// text_infos: SecondaryMap<DefaultKey, TexInfo>,

	// blob_arcs: Vec<(BlobArc, HashMap<String, u64>)>,

	glyph_id_map: XHashMap<(FontFamilyId, char), GlyphId>,
	pub glyphs: SlotMap<DefaultKey, GlyphIdDesc>,

	pub(crate) index_packer: TextPacker,
	pub data_packer: TextPacker,
	// pub(crate) size: Size<usize>,
	pub svg: XHashMap<u64, SvgInfo>,
	pub shapes: XHashMap<u64, TexInfo>,
}

impl Sdf2Table {
	pub fn new(width: usize, height: usize) -> Self {
		let _ = MULTI_MEDIA_RUNTIME.spawn(async move {
			if !INTI_STROE.load(Ordering::Relaxed){
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
			max_boxs: Default::default(),
			// text_infos: Default::default(),
			// blob_arcs: Default::default(),

			glyph_id_map: XHashMap::default(),
			glyphs: SlotMap::default(),
			// base_glyphs: SlotMap<DefaultKey, BaseCharDesc>,

			index_packer: TextPacker::new(width, height),
			data_packer: TextPacker::new(width, height),
			// size: Size {
			// 	width,
			// 	height
			// },
			svg: XHashMap::default(),
			shapes: XHashMap::default(),
		}

		
	}

	pub fn data_packer_size(&self) -> Size<usize> {
		Size {width:self.data_packer.width, height: self.data_packer.height}
	}

	pub fn index_packer_size(&self) -> Size<usize> {
		Size {width:self.index_packer.width, height: self.index_packer.height}
	}

	// 添加字体
	pub fn add_font(&mut self, font_id: FontFaceId, buffer: Vec<u8>) {
		// #[cfg(all(not(target_arch="wasm32"), not(feature="empty")))]
		let face = FontFace::new(buffer);
		// #[cfg(all(target_arch="wasm32", not(feature="empty")))]

		let max_box = face.max_box();
		self.fonts.insert(font_id.0, face);
		self.max_boxs.insert(font_id.0, max_box);
	}

	// 文字高度
	pub fn height(&mut self, font: &FontInfo) -> (f32, f32/*max_height*/) {
		let mut ret = (0.0, 0.0);
		for font_id in font.font_ids.iter() {
			if let Some(r) = self.fonts.get(font_id.0) {
				let height = r.ascender() - r.descender();
				if  height > ret.0 {
					ret.0 = height;
				}
			};
		}
		ret.1 = ret.0;
		ret
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
						log::debug!("mesure char width first, char: {}, font_id: {:?}, width: {:?}", char, font_id, (horizontal_advance, font.font.font_size as f32, horizontal_advance * font.font.font_size as f32));
						return (horizontal_advance * font.font.font_size as f32, glyph_id);
					}
				};
			}
		} else {
			if let Some(r) = self.fonts.get_mut(font.font_ids[self.glyphs[glyph_id.0].font_face_index].0) {
				let horizontal_advance = r.horizontal_advance(char);
				if horizontal_advance >= 0.0 {
					log::debug!("mesure char width, char: {}, font_id: {:?}, width: {:?}", char, font.font_ids[self.glyphs[glyph_id.0].font_face_index].0, (horizontal_advance, font.font.font_size as f32, horizontal_advance * font.font.font_size as f32));
					return (horizontal_advance * font.font.font_size as f32, glyph_id);
				}
			}
		}

		return (0.0, glyph_id)
	}

	// 字形id
	pub fn glyph_id(&mut self, font_id: FontId, font_info: &mut FontInfo, char: char) -> GlyphId {
		let id = match self.glyph_id_map.entry((font_info.font_family_id, char)) {
			Entry::Occupied(r) => {
				let id = r.get().clone();
				return id;
			},
			Entry::Vacant(r) => {
				
				// 分配GlyphId
				let id = GlyphId(self.glyphs.insert(GlyphIdDesc{
					font_id,
					char,
					font_face_index: pi_null::Null::null(),
					glyph: TexInfo::default()}));


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

	/// 取到字形信息
	pub fn glyph(&self, id: GlyphId) -> &TexInfo {
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
	pub fn draw_await(&mut self, fonts: &mut SlotMap<DefaultKey, FontInfo>, result: Arc<ShareMutex< (usize, Vec<(DefaultKey, TexInfo, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>,)>)>>, index: usize) -> AsyncValue<()> {
		let mut await_count = 0;
		for (_, font_info) in fonts.iter() {
			await_count += font_info.await_info.wait_list.len();
		}

		let async_value = AsyncValue::new();
		if await_count == 0 {
			async_value.clone().set(());
			// println!("encode_data_texzzzzz===={:?}, {:?}", index, await_count);
			return async_value;
		}

		// 轮廓信息（贝塞尔曲线）
		let mut outline_infos = Vec::with_capacity(await_count);
		// let mut outline_infos2: HashMap<DefaultKey, Vec<(char, GlyphId, &str)>> = HashMap::with_capacity(await_count);
		let mut chars = Vec::new();
		let mut keys = Vec::new();
		// let mut sdf_all_draw_slotmap = Vec::new();
		// let mut f = Vec::new();
		// 遍历所有的等待文字， 取到文字的贝塞尔曲线描述
		if await_count != 0 {
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
							await_count -= 1;
							continue;
						}
						// #[cfg(all(not(target_arch="wasm32"), not(feature="empty")))]
						outline_infos.push((font_face.to_outline(g.char), font_face_id.0, glyph_id)); // 先取到贝塞尔曲线						
						keys.push(format!("{}{}", g.char, font_info.font.font_family[g.font_face_index].as_str()));
						chars.push(g.char)
					}
				}
			}
		}

		result.lock().unwrap().1 = Vec::with_capacity(await_count);

		let max_boxs: &'static SecondaryMap<DefaultKey,  Aabb> = unsafe { transmute(&self.max_boxs) };
		let mut ll = 0;
	
		let temp_value = async_value.clone();		
		MULTI_MEDIA_RUNTIME.spawn(async move {
			if !INTI_STROE.load(Ordering::Relaxed){
				log::error!("=============存储未初始化");
				let async_value3 = AsyncValue::new(); 
				INTI_STROE_VALUE.lock().unwrap().push(async_value3.clone());
				async_value3.await;
			}
			// log::error!("encode_data_texxxx===={:?}, {:?}, {:?}, {:?}", index, ll, await_count, chars);
			// 遍历所有等待处理的字符贝塞尔曲线，将曲线转化为圆弧描述（多线程）
			for glyph_visitor in outline_infos.drain(..) {
				let async_value1 = async_value.clone();
				let result1 = result.clone();
				// println!("encode_data_tex===={:?}", index);
				let key = keys[ll].clone();
				MULTI_MEDIA_RUNTIME.spawn(async move {
					let temp_key = key.clone();
					let mut hasher = DefaultHasher::new();
					key.hash(&mut hasher);
					let key = hasher.finish().to_string();
					let (info, data_tex, index_tex, sdf_tex0, sdf_tex1, sdf_tex2, sdf_tex3, grid_size) = if let Some(buffer) = stroe::get(key.clone()).await{
						let SdfInfo{ tex_info, data_tex, index_tex, sdf_tex1, sdf_tex2, sdf_tex3, sdf_tex4, grid_size } = bincode::deserialize(&buffer[..]).unwrap();
						log::trace!("store is have: {}, data_tex: {}, tex_info: {:?}", temp_key, data_tex.len(), tex_info);
						(tex_info, data_tex, index_tex, sdf_tex1, sdf_tex2, sdf_tex3, sdf_tex4, grid_size)
					}else{
						
						// let (mut blod_arc, map) = FontFace::get_char_arc(max_boxs[glyph_visitor.1].clone(), glyph_visitor.0);

						// let data_tex = blod_arc.encode_data_tex1(&map);
						// // println!("data_map: {}", map.len());
						// let (info, index_tex,sdf_tex1, sdf_tex2, sdf_tex3, sdf_tex4) = blod_arc.encode_index_tex1( map, data_tex.len() / 4);
						#[cfg(all(not(target_arch="wasm32"), not(feature="empty")))]
						let sdf = {
							let sdf = FontFace::compute_sdf(max_boxs[glyph_visitor.1].clone(), glyph_visitor.0);
							let buffer = bincode::serialize(&sdf).unwrap();
							stroe::write(key, buffer).await;
							sdf
						};
						

						#[cfg(all(target_arch="wasm32", not(feature="empty")))]
						let sdf = {
							let buffer = FontFace::compute_sdf(max_boxs[glyph_visitor.1].clone(), glyph_visitor.0).await;
							
							let sdf: SdfInfo = bincode::deserialize(&buffer).unwrap();
							
							stroe::write(key, buffer).await;
							sdf
						};
						
						// println!("stroe::write end!! ");
						let SdfInfo{tex_info, data_tex, index_tex, sdf_tex1, sdf_tex2, sdf_tex3, sdf_tex4, grid_size} = sdf;
						log::trace!("store is not have: {}, data_tex: {}, tex_info: {:?}", temp_key, data_tex.len(), tex_info);
						(tex_info, data_tex, index_tex, sdf_tex1, sdf_tex2, sdf_tex3, sdf_tex4, grid_size)
					};
					
					// log::debug!("load========={:?}, {:?}", lock.0, len);
					let mut lock = result1.lock().unwrap();
					lock.0 += 1;
					// println!("encode_data_tex0===={:?}", (index, ll));
					log::trace!("encode_data_tex======cur_count: {:?}, grid_size={:?}, await_count={:?}, text_info={:?}", lock.0, grid_size, await_count, info);
					lock.1.push((glyph_visitor.2.0, info, data_tex, index_tex, sdf_tex0, sdf_tex1, sdf_tex2, sdf_tex3));
					if lock.0 == await_count {
						log::trace!("encode_data_tex1");
						async_value1.set(());
						// println!("encode_data_tex1===={}", index);
						log::trace!("encode_data_tex2");
					}
				}).unwrap();
			ll += 1;
			}
		}).unwrap();
	temp_value
	}

	pub fn update<F: FnMut(Block, FontImage) + Clone + 'static,  F1: FnMut(Block, ShadowImage) + Clone + 'static>(&mut self, mut update: F, mut updtae_shadow: F1, result: Arc<ShareMutex< (usize, Vec<(DefaultKey, TexInfo, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)>)>>) {
		let index_packer: &'static mut TextPacker = unsafe { transmute(&mut self.index_packer)};
		let data_packer: &'static mut TextPacker = unsafe { transmute(&mut self.data_packer)};
		let glyphs: &'static mut SlotMap<DefaultKey, GlyphIdDesc> = unsafe { transmute(&mut self.glyphs)};

		let mut lock = result.lock().unwrap();
		let r = &mut lock.1;
		log::debug!("sdf2 load2========={:?}", r.len());

		while let Some((glyph_id, mut text_info, mut data_tex, index_tex, sdf_tex0, sdf_tex1, sdf_tex2, sdf_tex3)) = r.pop() {
			// 索引纹理更新
			let index_tex_position = index_packer.alloc(
			text_info.grid_w as usize, 
			text_info.grid_h as usize);
			let index_position = match index_tex_position {
				Some(r) => r,
				None => panic!("aaaa================"),
			};
			let index_img = FontImage {
				width: text_info.grid_w as usize,
				height: text_info.grid_h as usize,
				buffer: index_tex,
			};
			text_info.index_offset_x = index_position.x;
			text_info.index_offset_y =  index_position.y;
			let index_block = Block {
				x: index_position.x as f32,
				y: index_position.y as f32,
				width: index_img.width as f32,
				height: index_img.height as f32,
			};
			// log::warn!("update index tex========={:?}", (&index_block,index_img.width, index_img.height, index_img.buffer.len(), &text_info) );
			(update.clone())(index_block, index_img);

			// 更新阴影纹理
			let shadow_img = ShadowImage {
				width: text_info.grid_w as usize,
				height: text_info.grid_h as usize,
				minimip: vec![sdf_tex0, sdf_tex1, sdf_tex2, sdf_tex3]
			};
			updtae_shadow(index_block, shadow_img);

			// 数据纹理更新
			let data_len = data_tex.len() / 4;
			let mut patch = 8.0 - data_len as f32 % 8.0;
			while patch > 0.0 {
				data_tex.extend_from_slice(&[0, 0, 0, 0]); // 补0
				patch -= 1.0;
			}

			let h = (data_len as f32 / 8.0).ceil() as usize;
			let data_img = FontImage {
				width: 8,
				height: h,
				buffer: data_tex,
			};
			let data_position = data_packer.alloc(
				data_img.height,
				data_img.width);
			let data_position = match data_position {
				Some(r) => r,
				None => panic!("bbb================"),
			};
			text_info.data_offset_x = data_position.y;
			text_info.data_offset_y = data_position.x;
			let data_block = Block {
				x: data_position.y as f32,
				y: data_position.x as f32,
				width: data_img.width as f32,
				height: data_img.height as f32,
			};
			// log::warn!("update data tex========={:?}", (&data_block, data_img.width, data_img.height, data_img.buffer.len(), data_len, p1) );
			update(data_block, data_img);

			glyphs[glyph_id].glyph = text_info;

			log::trace!("text_info=========={:?}, {:?}, {:?}, {:?}", glyph_id, glyphs[glyph_id].glyph, index_position, data_position);
			
		}
	
	}

	pub fn set_view_box(&mut self, mins_x: f32,mins_y: f32,maxs_x: f32,maxs_y: f32, ){
		// self.svg.view_box = Aabb::new(Point::new(mins_x, mins_y), Point::new(maxs_x, maxs_y))
	}


	pub fn add_shape(&mut self, hash: u64, info: SvgInfo) {
		self.svg.insert(hash, info);
	}

	pub fn has_shape(&mut self, hash: u64) ->bool {
		self.svg.get(&hash).is_some()
	}

	/// 更新字形信息（计算圆弧信息）
	pub fn draw_svg_await(&mut self) -> AsyncValue<Arc<ShareMutex< (usize, Vec<(u64, TexInfo, Vec<u8>, Vec<u8>, Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>)>)>>> {
		let await_count = self.svg.len();

		let texture_data = Vec::with_capacity(await_count);
		let result: Arc<ShareMutex< (usize, Vec<(u64, TexInfo, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>,)>)>> = Share::new(ShareMutex::new((0, texture_data)));
		let async_value = AsyncValue::new();

		// 遍历所有等待处理的字符贝塞尔曲线，将曲线转化为圆弧描述（多线程）
		for (hash, info) in self.svg.drain() {
			let async_value1 = async_value.clone();
			let result1 = result.clone();
			MULTI_MEDIA_RUNTIME.spawn(async move {
	
				let SdfInfo{tex_info, data_tex, index_tex, sdf_tex1, sdf_tex2, sdf_tex3, sdf_tex4, grid_size} = computer_svg_sdf(info);

				// log::debug!("load========={:?}, {:?}", lock.0, len);
				let mut lock = result1.lock().unwrap();
				lock.0 += 1;
				// log::trace!("encode_data_tex======cur_count: {:?}, grid_size={:?}, await_count={:?}, text_info={:?}", lock.0, await_count);
				lock.1.push((hash, tex_info, data_tex, index_tex, sdf_tex1, sdf_tex2, sdf_tex3, sdf_tex4));
				if lock.0 == await_count {
					log::trace!("encode_data_tex1");
					async_value1.set(result1.clone());
					log::trace!("encode_data_tex2");
				}
			}).unwrap();
		}
		async_value
	}

	pub fn update_svg<F: FnMut(Block, FontImage) + Clone + 'static, F1: FnMut(Block, ShadowImage) + Clone + 'static>(&mut self, mut update: F, mut updtae_shadow: F1, result: Arc<ShareMutex< (usize, Vec<(u64, TexInfo, Vec<u8>, Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>,)>)>>) {
		let index_packer: &'static mut TextPacker = unsafe { transmute(&mut self.index_packer)};
		let data_packer: &'static mut TextPacker = unsafe { transmute(&mut self.data_packer)};
		let shapes: &'static mut XHashMap<u64, TexInfo> = unsafe { transmute(&mut self.shapes)};

		let mut lock = result.lock().unwrap();
		let r = &mut lock.1;
		log::debug!("sdf2 load2========={:?}", r.len());

		while let Some((hash, mut text_info,mut data_tex, index_tex, sdf_tex0, sdf_tex1, sdf_tex2, sdf_tex3)) = r.pop() {
			// 索引纹理更新
			let index_tex_position = index_packer.alloc(
			text_info.grid_w as usize, 
			text_info.grid_h as usize);
			let index_position = match index_tex_position {
				Some(r) => r,
				None => panic!("aaaa================"),
			};
			let index_img = FontImage {
				width: text_info.grid_w as usize,
				height: text_info.grid_h as usize,
				buffer: index_tex,
			};
			text_info.index_offset_x = index_position.x;
			text_info.index_offset_y =  index_position.y;
			let index_block = Block {
				x: index_position.x as f32,
				y: index_position.y as f32,
				width: index_img.width as f32,
				height: index_img.height as f32,
			};
			// log::warn!("update index tex========={:?}", (&index_block,index_img.width, index_img.height, index_img.buffer.len(), &text_info) );
			(update.clone())(index_block, index_img);

			// 更新阴影纹理
			let shadow_img = ShadowImage {
				width: text_info.grid_w as usize,
				height: text_info.grid_h as usize,
				minimip: vec![sdf_tex0, sdf_tex1, sdf_tex2, sdf_tex3]
			};
			updtae_shadow(index_block, shadow_img);

			// 数据纹理更新
			let data_len = data_tex.len() / 4;
			let mut patch = 8.0 - data_len as f32 % 8.0;
			while patch > 0.0 {
				data_tex.extend_from_slice(&[0, 0, 0, 0]); // 补0
				patch -= 1.0;
			}

			let h = (data_len as f32 / 8.0).ceil() as usize;
			let data_img = FontImage {
				width: 8,
				height: h,
				buffer: data_tex,
			};
			let data_position = data_packer.alloc(
				data_img.height,
				data_img.width);
			let data_position = match data_position {
				Some(r) => r,
				None => panic!("bbb================"),
			};
			text_info.data_offset_x = data_position.y;
			text_info.data_offset_y = data_position.x;
			let data_block = Block {
				x: data_position.y as f32,
				y: data_position.x as f32,
				width: data_img.width as f32,
				height: data_img.height as f32,
			};
			// log::warn!("update data tex========={:?}", (&data_block, data_img.width, data_img.height, data_img.buffer.len(), data_len, p1) );
			update(data_block, data_img);

			shapes.insert(hash, text_info);

			// log::trace!("text_info=========={:?}, {:?}, {:?}, {:?}", glyph_id, glyphs[glyph_id].glyph, index_position, data_position);
			
		}
	
	}

}


#[derive(Debug)]
pub struct AwaitDraw {
	pub char: char, 
	pub font_id: FontFaceId,
}


pub struct OnceCellWrap(pub OnceCell<ShareCb>);
unsafe impl Sync for OnceCellWrap {}

pub struct OnceLockWrap(pub OnceLock<ShareCb>);

#[cfg(not(target_arch="wasm32"))]
static LOAD_CB_SDF: OnceLockWrap = OnceLockWrap(OnceLock::new());

#[cfg(target_arch="wasm32")]
static LOAD_CB_SDF: OnceCellWrap = OnceCellWrap(OnceCell::new());
// pub static SDF_LOADER: OnceCell<Box<dyn FnMut()>> = OnceCellWrap(OnceCell::new());
lazy_static! {
	
	pub static ref LOAD_MAP_SDF: Mutex<SlotMap<DefaultKey, AsyncValue<Vec<Vec<u8>>>>> =
		Mutex::new(SlotMap::new());
}

#[cfg(target_arch="wasm32")]
pub trait Cb: Fn(DefaultKey, usize, &[char]) {}
#[cfg(target_arch="wasm32")]
impl<T: Fn(DefaultKey, usize, &[char])> Cb for T {}
#[cfg(target_arch="wasm32")]
pub type ShareCb = std::rc::Rc<dyn Cb>;

#[cfg(not(target_arch="wasm32"))]
pub trait Cb: Fn(DefaultKey, usize, &[char])  + Send + Sync {}
#[cfg(not(target_arch="wasm32"))]
impl<T: Fn(DefaultKey, usize, &[char]) + Send + Sync > Cb for T {}
#[cfg(not(target_arch="wasm32"))]
pub type ShareCb = Arc<dyn Cb>;




pub fn init_load_cb(cb: ShareCb) {
    match LOAD_CB_SDF.0.set(cb) {
		Ok(r) => r,
		Err(_e) => panic!("LOAD_CB_SDF.set")
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

pub struct GlyphIdDesc {
	pub font_id: FontId,
	pub char: char,
	pub glyph: TexInfo,
	pub font_face_index: usize,
}




