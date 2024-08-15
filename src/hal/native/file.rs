use std::hash::{DefaultHasher, Hash, Hasher};

use pi_atom::Atom;
use pi_share::Share;

use crate::{create_async_value, Arg};

pub async fn load_from_url(path: &Atom) -> Result<Share<Vec<u8>>, String> {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let v = create_async_value("file", "", hasher.finish(), vec![Arg::String(path.to_string())]);
    Ok(v.await?)
}
