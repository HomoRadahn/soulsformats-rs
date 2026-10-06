use crate::{
    io::{BinaryReader, BinaryWriter},
    util,
};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use std::io::{self, Read, Seek, Write};
use zstd::{self, bulk::Compressor, zstd_safe::CParameter};

/// Decompresses deflate bytes
pub(crate) fn decompress_deflate_bytes(compressed_bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut decoder = DeflateDecoder::new(compressed_bytes);
    let mut decompressed = Vec::new();

    decoder.read_to_end(&mut decompressed)?;

    Ok(decompressed)
}

/// Compresses deflate bytes
pub(crate) fn compress_deflate_bytes(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let output = encoder.finish()?;

    Ok(output)
}

/// Reads a Zstd block from a `BinaryReader` and returns the uncompressed data
pub(crate) fn read_zstd<R>(br: &mut BinaryReader<R>, compressed_size: u64) -> io::Result<Vec<u8>>
where
    R: Read + Seek,
{
    let compressed = br.read_vec_u8(compressed_size)?;
    zstd::decode_all(compressed.as_slice())
}

/// Writes `data` as Zstd with `compression_level`
pub(crate) fn write_zstd(data: &[u8], compression_level: u8) -> io::Result<Vec<u8>> {
    let mut compressor = Compressor::new(compression_level as i32)?;
    compressor.set_parameter(CParameter::ContentSizeFlag(false))?;
    compressor.set_parameter(CParameter::WindowLog(16))?;
    compressor.compress(data)
}

/// Size in bytes of one uncompressed EDGE chunk
pub(crate) const EDGE_CHUNK_SIZE: usize = 0x10000;

/// Returns the number of EDGE chunks needed for `len` bytes, and the size of the last partial
/// chunk (0 if the data divides evenly)
pub(crate) fn edge_chunk_layout(len: usize) -> (usize, usize) {
    (len.div_ceil(EDGE_CHUNK_SIZE), len % EDGE_CHUNK_SIZE)
}

/// Compresses chunk `index` of `data` for an EDGE container, falling back to the raw bytes when
/// deflate doesn't make them smaller. Returns the chunk bytes and whether they are compressed
pub(crate) fn compress_edge_chunk(data: &[u8], index: usize) -> io::Result<(Vec<u8>, bool)> {
    let start = index * EDGE_CHUNK_SIZE;
    let input = &data[start..data.len().min(start + EDGE_CHUNK_SIZE)];
    let compressed = compress_deflate_bytes(input)?;

    if compressed.len() < input.len() {
        Ok((compressed, true))
    } else {
        Ok((input.to_vec(), false))
    }
}

/// Writes a zlib stream and returns the number of bytes written
pub(crate) fn write_zlib<W>(
    bw: &mut BinaryWriter<W>,
    format_byte: u8,
    input: &[u8],
) -> io::Result<i32>
where
    W: Write + Seek,
{
    let start = bw.position()?;
    bw.write_u8(0x78)?;
    bw.write_u8(format_byte)?;

    bw.write_vec_u8(compress_deflate_bytes(input)?)?;

    bw.write_u32(adler32(input))?;

    util::convert_num(bw.position()? - start)
}

pub(crate) fn read_zlib<R>(br: &mut BinaryReader<R>, compressed_size: u64) -> io::Result<Vec<u8>>
where
    R: Read + Seek,
{
    br.assert_u8(&[0x78])?;
    br.assert_u8(&[0x01, 0x5E, 0x9C, 0xDA])?;

    let deflate_size = compressed_size
        .checked_sub(2)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "zlib stream is too short"))?;
    decompress_deflate_bytes(&br.read_vec_u8(deflate_size)?)
}

fn adler32(data: &[u8]) -> u32 {
    let mut adler_a: u32 = 1;
    let mut adler_b: u32 = 0;

    for byte in data {
        adler_a = (adler_a + u32::from(*byte)) % 65521;
        adler_b = (adler_b + adler_a) % 65521;
    }

    (adler_b << 16) | adler_a
}

/// Appends a chunk read from an EDGE container to `output`, inflating it if necessary
pub(crate) fn append_edge_chunk(
    output: &mut Vec<u8>,
    chunk: Vec<u8>,
    compressed: bool,
) -> io::Result<()> {
    if compressed {
        output.extend_from_slice(&decompress_deflate_bytes(&chunk)?);
    } else {
        output.extend_from_slice(&chunk);
    }
    Ok(())
}
