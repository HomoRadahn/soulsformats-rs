use std::io::{self, Read, Seek};
use crate::{io::BinaryReader};
use zstd;

pub struct ZstdHelper;

impl ZstdHelper {
    /// Reads a Zstd block from a `BinaryReader` and returns the uncompressed data
    pub fn read_zstd<R>(br: &mut BinaryReader<R>, compressed_size: u64) -> io::Result<Vec<u8>>
    where
        R: Read + Seek
    {
        let compressed = br.read_u8_vec(compressed_size)?;
        zstd::decode_all(compressed.as_slice())
    }
}