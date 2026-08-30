use std::io::{self, Read, Seek, Write};
use crate::dcx::deflate_helper::{DeflateHelper};

use crate::{io::{BinaryReader, BinaryWriter}};

pub struct ZlibHelper;

impl ZlibHelper {
    #[allow(dead_code, unused_variables)]
    pub fn write_zlib<W>(bw: &mut BinaryWriter<W>, format_byte: u8, input: &Vec<u8>) -> io::Result<i32>
    where 
        W: Write + Seek
    {
        let start = bw.position()?;
        bw.write_u8(0x78)?;
        bw.write_u8(format_byte)?;

        bw.write_u8_vec(DeflateHelper::compress_deflate_bytes(input.as_slice())?)?;

        bw.write_u32(ZlibHelper::adler32(&input))?;

        Ok(i32::try_from(bw.position()? - start).unwrap())
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

    fn adler32(data: &Vec<u8>) -> u32 {
        let mut adler_a: u32 = 1;
        let mut adler_b: u32 = 0;

        for byte in data {
            adler_a = (adler_a + u32::try_from(*byte).unwrap()) % 65521;
            adler_b = (adler_b + adler_a) % 65521;
        };

        (adler_b << 16) | adler_a
    }
}