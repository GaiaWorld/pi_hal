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
    pub static ref STROE_PATH: RwLock<Option<PathBuf >> = RwLock::new(None);
}

pub async fn init_local_store() {
    let mut hasher = DefaultHasher::new();
    STORE_INIT_LOCAL_KEY.hash(&mut hasher);
    let v = create_async_value("store", "initLocalStore", hasher.finish(), vec![]);

    if let Ok(byte) = v.await {
        if let Ok(path) = String::from_utf8(byte.to_vec()) {
            let path = PathBuf::from_str(&path).unwrap();
            let path = path.join("sdf_font");
            let _ = std::fs::create_dir_all(&path);
            *STROE_PATH.write() = Some(path);
        }
    }
}

/**
 * 从indexDb读数据
 */
// tslint:disable-next-line:no-reserved-keywords
pub async fn get(key: String) -> Option<Vec<u8>> {
    println!("init_local_store get key: {}", key);
    if let Some(path) = STROE_PATH.read().as_ref() {
        let path = path.join(key);
        println!("init_local_store get222 key: {:?}", path);
        if let Ok(data) = std::fs::read(path) {
            println!("init_local_store get333 key: {:?}", data.len());
            return Some(data);
        }
    }
    None
}

/**
 * 往indexDb写数据
 */
pub async fn write(key: String, data: Vec<u8>) {
    println!("init_local_store write key: {}", key);
    if let Some(path) = STROE_PATH.read().as_ref() {
        let path = path.join(key);
        println!("init_local_store write222 key: {:?}", path);
        let _ = std::fs::write(path, data);
    }
}

/**
 * 从indexDb删除数据
 */
pub async fn delete_key(key: String) {
    println!("init_local_store delete_key: {}", key);
    if let Some(path) = STROE_PATH.read().as_ref() {
        let path = path.join(key);
        let _ = std::fs::remove_file(path);
    }
}
