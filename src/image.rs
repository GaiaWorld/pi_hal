/// 图片

pub fn from_path(path: &str) -> Result<(Vec<u8>, u32, u32), image::ImageError> {
    let dynamic_image = image::open(path)?;
    let image_buffer = dynamic_image.into_rgba8();
    let (width, height) = image_buffer.dimensions();
    Ok((image_buffer.into_raw(), width, height))
}

pub fn from_memory(buf: &[u8]) -> Result<(Vec<u8>, u32, u32), image::ImageError> {
    let dynamic_image = image::load_from_memory(buf)?;
    let image_buffer = dynamic_image.into_rgba8();
    let (width, height) = image_buffer.dimensions();
    Ok((image_buffer.into_raw(), width, height))
}
