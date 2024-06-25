use crate::initLocalStore;

pub async fn init_local_store (){
    initLocalStore().await;
}
    
/**
 * 从indexDb读数据
 */
// tslint:disable-next-line:no-reserved-keywords
pub async fn get (key: String) -> Option<Vec<u8>>{
    match super::get(key).await{
        Ok(r) => {
            // log::error!("r: {:?}", )
            if r.is_undefined() || r.is_null(){
                return None;
            }
            return Some(js_sys::Uint8Array::from(r).to_vec())},
        Err(_) => None,
    }
    
}

/**
 * 往indexDb写数据
 */
pub async fn write(key: String, data: Vec<u8>){
    super::write(key, data).await
}

/**
 * 从indexDb删除数据
 */
pub async fn delete_key(key: String){
    super::deleteKey(key).await
}