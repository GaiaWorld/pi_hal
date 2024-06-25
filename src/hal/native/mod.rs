use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use parking_lot::Mutex;
use pi_async_rt::rt::AsyncValueNonBlocking as AsyncValue;

pub mod compressed_texture;
pub mod file;
pub mod font_brush;
pub mod image;
pub mod runtime;
pub mod stroe;
// pub use basis_universal::TranscoderTextureFormat::*;

pub enum Arg {
    Number(u64),
    String(String),
    Buffer(Vec<u8>),
    None
}

lazy_static! {
    pub static ref LOAD_CB: RwLock<Option<Arc<dyn Fn(String, String, String, Vec<Arg>) + Send + Sync>>> = RwLock::new(None);
    pub static ref LOAD_MAP: Mutex<HashMap<u64, Vec<AsyncValue<Result<Vec<u8>, String>>>>> =
        Mutex::new(HashMap::new());
}

pub fn init_load_cb(cb: Arc<dyn Fn(String, String, String, Vec<Arg>) + Send + Sync>) {
    *LOAD_CB.write().unwrap() = Some(cb);
}

pub fn on_load(hash: u64, data: Result<Vec<u8>, String>) {
    let mut v = LOAD_MAP.lock().remove(&hash).unwrap();
    v.drain(..).for_each(|v| {
        v.set(data.clone());
    });
}

pub fn create_async_value(modules: &str, func: &str, hash: u64, args: Vec<Arg>) -> AsyncValue<Result<Vec<u8>, String>> {
    let mut lock = LOAD_MAP.lock();
    if let Some(vec) = lock.get_mut(&hash) {
        let v = AsyncValue::new();
        vec.push(v.clone());
        v
    } else {
        let v = AsyncValue::new();
        lock.insert(hash, vec![v.clone()]);

        if let Some(cb) = LOAD_CB.read().unwrap().as_ref() {
            cb(modules.to_string(), func.to_string(), hash.to_string(), args);
        }

        v
    }
}

