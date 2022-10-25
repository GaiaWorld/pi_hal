// use basis_universal::{TranscoderTextureFormat, TranscodeError, Transcoder, TranscodeParameters};
// use pi_atom::Atom;

// use crate::create_async_value;

// pub fn from_path(path: &str, transcode_format: TranscoderTextureFormat,) -> Result<Vec<u8>, TranscodeError> {
//     let buf = std::fs::read(path).unwrap();
//     from_memory(&buf, transcode_format)
// }

// pub fn from_memory(buf: &[u8], transcode_format: TranscoderTextureFormat,) -> Result<Vec<u8>, TranscodeError> {
//     let mut transcoder = Transcoder::new();

//     transcoder.prepare_transcoding(buf).unwrap();

//     transcoder.transcode_image_level(
//         buf,
//         transcode_format,
//         TranscodeParameters {
//             image_index: 0,
//             level_index: 0,
//             ..Default::default()
//         },
//     )
// }

// pub async fn load_compressed_texture_async(
//     path: &Atom,
//     transcode_format: TranscoderTextureFormat,
// ) -> Result<Vec<u8>, TranscodeError> {
//     let v = create_async_value(path);
//     from_memory(&v.await, transcode_format)
// }