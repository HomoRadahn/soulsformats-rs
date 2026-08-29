use std::io::{self, Read};
use flate2::read::DeflateDecoder;

pub struct DeflateHelper;

impl DeflateHelper {
    /// Decompresses zlib bytes coming after a zlib header
    pub fn decompress_deflate_bytes(compressed_bytes: &[u8]) -> io::Result<Vec<u8>> {
        let mut decoder = DeflateDecoder::new(compressed_bytes);
        let mut decompressed = Vec::new();

        decoder.read_to_end(&mut decompressed)?;

        Ok(decompressed)
    }
}