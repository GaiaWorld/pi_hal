use std::hash::{DefaultHasher, Hash, Hasher};

use pi_atom::Atom;
use pi_share::Share;

use crate::{Arg, create_async_value, texture::RES_MAP};

pub async fn load_from_url(path: &Atom) -> Result<Share<Vec<u8>>, String> {
    // println!("=========== file load_from_url: {:?}", path);
    let data = {
        let r = RES_MAP.read().unwrap();
        let r = r.get(path).cloned();
        if let Some(data) = r {
            // println!("=========== file load_from_url form crash: {:?}", path);
            return Ok(data);
        }
    };
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let v = create_async_value("file", "", hasher.finish(), vec![Arg::String(path.to_string())]);
    let v = v.await?;
    let r = if v.len() > 0 {
			v
    } else {
        if let Some(data) = RES_MAP.read().unwrap().get(path) {
            data.clone()
        } else {
            return Err("RES_MAP 缓存 异常!!!".to_string());
        }
    };
    // println!("=========== file load_from_url: {:?}", path);
    Ok(r)
}
