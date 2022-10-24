use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use parking_lot::Mutex;
use pi_async::rt::AsyncValue;

pub mod file;
pub mod font_brush;
pub mod image;
pub mod runtime;
pub mod compressed_texture;

pub use basis_universal::TranscoderTextureFormat::*;

lazy_static! {
    pub static ref LOAD_CB: RwLock<Option<Arc<dyn Fn(String) + Send + Sync>>> = RwLock::new(None);
    pub static ref LOAD_MAP: Mutex<HashMap<String, AsyncValue<Vec<u8>>>> =
        Mutex::new(HashMap::new());
}

pub fn init_load_cb(cb: Arc<dyn Fn(String) + Send + Sync>) {
    *LOAD_CB.write().unwrap() = Some(cb);
}

pub fn on_load(path: &str, data: Vec<u8>) {
    let v = LOAD_MAP.lock().remove(path).unwrap();
    v.set(data);
}

pub fn create_async_value(path: &str) -> AsyncValue<Vec<u8>> {
    let v = AsyncValue::new();

    LOAD_MAP.lock().insert(path.to_string(), v.clone());

    if let Some(cb) = LOAD_CB.read().unwrap().as_ref() {
        cb(path.to_string());
    }

    v
}
