use crate::io::BinaryReader;
use std::io::{self, Read, Seek};
use zstd;
use zstd::bulk::Compressor;
use zstd::zstd_safe::CParameter;

pub struct ZstdHelper;

impl ZstdHelper {
    /// Reads a Zstd block from a `BinaryReader` and returns the uncompressed data
    pub fn read_zstd<R>(br: &mut BinaryReader<R>, compressed_size: u64) -> io::Result<Vec<u8>>
    where
        R: Read + Seek,
    {
        let compressed = br.read_vec_u8(compressed_size)?;
        zstd::decode_all(compressed.as_slice())
    }

    /// Writes `data` as Zstd with `compression_level`
    pub fn write_zstd(data: &[u8], compression_level: u8) -> io::Result<Vec<u8>> {
        let mut compressor = Compressor::new(compression_level as i32)?;
        compressor.set_parameter(CParameter::ContentSizeFlag(false))?;
        compressor.set_parameter(CParameter::WindowLog(16))?;
        compressor.compress(data)
    }
}
