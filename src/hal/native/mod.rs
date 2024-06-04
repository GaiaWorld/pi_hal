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

// pub use basis_universal::TranscoderTextureFormat::*;

lazy_static! {
    pub static ref LOAD_CB: RwLock<Option<Arc<dyn Fn(String) + Send + Sync>>> = RwLock::new(None);
    pub static ref LOAD_MAP: Mutex<HashMap<String, Vec<AsyncValue<Result<Vec<u8>, String>>>>> =
        Mutex::new(HashMap::new());
}

pub fn init_load_cb(cb: Arc<dyn Fn(String) + Send + Sync>) {
    *LOAD_CB.write().unwrap() = Some(cb);
}

pub fn on_load(path: &str, data: Result<Vec<u8>, String>) {
    let mut v = LOAD_MAP.lock().remove(path).unwrap();
    v.drain(..).for_each(|v| {
        v.set(data.clone());
    });
}

pub fn create_async_value(path: &str) -> AsyncValue<Result<Vec<u8>, String>> {
    let mut lock = LOAD_MAP.lock();
    if let Some(vec) = lock.get_mut(path) {
        let v = AsyncValue::new();
        vec.push(v.clone());
        v
    } else {
        let v = AsyncValue::new();
        lock.insert(path.to_string(), vec![v.clone()]);

        if let Some(cb) = LOAD_CB.read().unwrap().as_ref() {
            cb(path.to_string());
        }

        v
    }
}

