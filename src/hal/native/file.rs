use pi_atom::Atom;

use crate::create_async_value;

pub async fn load_from_url(path: &Atom) -> Result<Vec<u8>, String> {
    let v = create_async_value(path);
    Ok(v.await?)
}
