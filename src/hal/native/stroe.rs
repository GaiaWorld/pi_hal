use crate::LOAD_CB;

pub async fn init_local_store (){
    // if let Some(cb) = LOAD_CB.read().unwrap().as_ref() {
    //     cb(1, "init_local_store".to_string());
    // }
}
    
/**
 * 从indexDb读数据
 */
// tslint:disable-next-line:no-reserved-keywords
pub async fn get (key: String) -> Option<Vec<u8>>{
    return Some(vec![])
}

/**
 * 往indexDb写数据
 */
pub async fn write  (key: String, data: Vec<u8>){
    // super::write(key, data)
}

/**
 * 从indexDb删除数据
 */
pub async fn delete_key(key: String){
    // super::deleteKey(key)
}