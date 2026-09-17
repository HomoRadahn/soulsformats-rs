use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use std::io::{self, Read, Write};

/// Decompresses deflate bytes
pub fn decompress_deflate_bytes(compressed_bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut decoder = DeflateDecoder::new(compressed_bytes);
    let mut decompressed = Vec::new();

    decoder.read_to_end(&mut decompressed)?;

    Ok(decompressed)
}

/// Compresses deflate bytes
pub fn compress_deflate_bytes(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&data)?;
    let output = encoder.finish()?;

    Ok(output)
}
