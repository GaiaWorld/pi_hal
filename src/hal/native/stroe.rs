use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    str::FromStr,
};

use parking_lot::RwLock;

use crate::{create_async_value, Arg, LOAD_CB};

static STORE_INIT_LOCAL_KEY: &'static str = "STORE_INIT_LOCAL_KEY";
static STORE_GET_KEY: &'static str = "STORE_GET_KEY";
static STORE_WRITE_KEY: &'static str = "STORE_WRITE_KEY";
static STORE_DELETE_KEY: &'static str = "STORE_DELETE_KEY";
lazy_static! {
    pub static ref STROE_PATH: RwLock<Option<String>> = RwLock::new(None);
}

pub async fn init_local_store() {
    let mut hasher = DefaultHasher::new();
    STORE_INIT_LOCAL_KEY.hash(&mut hasher);
    let v = create_async_value("store", "initLocalStore", hasher.finish(), vec![]);

    if let Ok(byte) = v.await {
        if let Ok(path) = String::from_utf8(byte.to_vec()) {
            *STROE_PATH.write() = Some(path);
        }
    }
}

/**
 * 从indexDb读数据
 */
// tslint:disable-next-line:no-reserved-keywords
pub async fn get(key: String) -> Option<Vec<u8>> {
    if let Some(path) = STROE_PATH.read().as_ref() {
        let path = PathBuf::from_str(path).unwrap();
        let path = path.join("sdf_font").join(key);
        if let Ok(data) = std::fs::read(path) {
            return Some(data);
        }
    }
    None
}

/**
 * 往indexDb写数据
 */
pub async fn write(key: String, data: Vec<u8>) {
    if let Some(path) = STROE_PATH.read().as_ref() {
        let path = PathBuf::from_str(path).unwrap();
        let path = path.join("sdf_font").join(key);
        let _ = std::fs::write(path, data);
    }
}

/**
 * 从indexDb删除数据
 */
pub async fn delete_key(key: String) {
    if let Some(path) = STROE_PATH.read().as_ref() {
        let path = PathBuf::from_str(path).unwrap();
        let path = path.join("sdf_font").join(key);
        let _ = std::fs::remove_file(path);
    }
}
