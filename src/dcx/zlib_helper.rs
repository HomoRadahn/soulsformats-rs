use std::io::{self, Read, Seek, Write};
use crate::dcx::deflate_helper::{DeflateHelper};

use crate::{io::{BinaryReader, BinaryWriter}};

pub struct ZlibHelper;

impl ZlibHelper {
    pub fn write_zlib<W>(bw: BinaryWriter<W>, format_byte: u8, input: Vec<u8>) -> io::Result<i32>
    where 
        W: Write + Seek
    {
        unimplemented!()
    }

    /// Reads a Zlib block from a `BinaryReader` and returns the uncompressed data
    pub fn read_zlib<R>(br: &mut BinaryReader<R>, compressed_size: u64) -> io::Result<Vec<u8>>
    where
        R: Read + Seek
    {
        br.assert_u8(&[0x78])?;
        br.assert_u8(&[0x01, 0x5E, 0x9C, 0xDA])?;
        
        DeflateHelper::decompress_deflate_bytes(&br.read_u8_vec(compressed_size - 2)?)
    }
}