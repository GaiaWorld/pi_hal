use crate::initLocalStore;
use js_sys::Array;

use super::loadSdfGlyphs;

pub async fn init_local_store() -> Option<Vec<u8>> {
    initLocalStore().await;
    let r = crate::loadFontSdf().await;

    // log::error!("r: {:?}", )
    if r.is_undefined() || r.is_null() {
        return None;
    }
    return Some(js_sys::Uint8Array::from(r).to_vec());
}

/**
 * 从indexDb读数据
 */
// tslint:disable-next-line:no-reserved-keywords
pub async fn get(key: String) -> Option<Vec<u8>> {
    match super::get(key).await {
        Ok(r) => {
            // log::error!("r: {:?}", )
            if r.is_undefined() || r.is_null() {
                return None;
            }
            return Some(js_sys::Uint8Array::from(r).to_vec());
        }
        Err(_) => None,
    }
}

/**
 * 往indexDb写数据
 */
pub async fn write(key: String, data: Vec<u8>) {
    super::write(key, data).await
}

/**
 * 从indexDb删除数据
 */
pub async fn delete_key(key: String) {
    super::deleteKey(key).await
}

pub async fn load_sdf_glyphs(font_name: &str, chars: &[char]) -> Vec<Vec<u8>> {
    let codepoints: Vec<u32> = chars.iter().map(|value| *value as u32).collect();
    let result = loadSdfGlyphs(font_name.to_owned(), &codepoints).await;
    let array = match result {
        Ok(value) if Array::is_array(&value) => Array::from(&value),
        Ok(_) => return vec![Vec::new(); chars.len()],
        Err(error) => {
            log::warn!("load SDF glyphs for {} failed: {:?}", font_name, error);
            return vec![Vec::new(); chars.len()];
        }
    };

    chars.iter().enumerate().map(|(index, _)| {
        let value = array.get(index as u32);
        if value.is_null() || value.is_undefined() {
            Vec::new()
        } else {
            js_sys::Uint8Array::new(&value).to_vec()
        }
    }).collect()
}
