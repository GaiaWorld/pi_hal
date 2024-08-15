use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{create_async_value, Arg, LOAD_CB};

static STORE_INIT_LOCAL_KEY: &'static str = "STORE_INIT_LOCAL_KEY";
static STORE_GET_KEY: &'static str = "STORE_GET_KEY";
static STORE_WRITE_KEY: &'static str = "STORE_WRITE_KEY";
static STORE_DELETE_KEY: &'static str = "STORE_DELETE_KEY";

pub async fn init_local_store() {
    let mut hasher = DefaultHasher::new();
    STORE_INIT_LOCAL_KEY.hash(&mut hasher);
    let v = create_async_value("store", "initLocalStore", hasher.finish(), vec![]);

    let _ = v.await;
}

/**
 * 从indexDb读数据
 */
// tslint:disable-next-line:no-reserved-keywords
pub async fn get(key: String) -> Option<Vec<u8>> {
    return None;
    let mut hash = key.to_string();
    // hash.push_str(STORE_GET_KEY);

    // let mut hasher = DefaultHasher::new();
    // hash.hash(&mut hasher);

    // let v = create_async_value("store", "get", hasher.finish(), vec![Arg::String(key)]);
    // match v.await {
    //     Ok(v) => {
    //         if v.is_empty() {
    //             return None;
    //         } else {
    //             return Some(v);
    //         }
    //     }
    //     Err(_) => return None,
    // }
}

/**
 * 往indexDb写数据
 */
pub async fn write(key: String, data: Vec<u8>) {
    return ;
    let mut hash = key.to_string();
    hash.push_str(STORE_WRITE_KEY);

    let mut hasher = DefaultHasher::new();
    hash.hash(&mut hasher);

    let v = create_async_value("store", "write", hasher.finish(), vec![Arg::String(key), Arg::Buffer(data)]);
    let _ = v.await;
}

/**
 * 从indexDb删除数据
 */
pub async fn delete_key(key: String) {
    let mut hash = key.to_string();
    hash.push_str(STORE_DELETE_KEY);

    let mut hasher = DefaultHasher::new();
    hash.hash(&mut hasher);

    let v = create_async_value("store", "delete", hasher.finish(), vec![Arg::String(key)]);
    let _ = v.await;
}
