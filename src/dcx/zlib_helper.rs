use crate::{dcx::deflate_helper, util};
use std::io::{self, Read, Seek, Write};

use crate::io::{BinaryReader, BinaryWriter};

pub fn write_zlib<W>(bw: &mut BinaryWriter<W>, format_byte: u8, input: &Vec<u8>) -> io::Result<i32>
where
    W: Write + Seek,
{
    let start = bw.position()?;
    bw.write_u8(0x78)?;
    bw.write_u8(format_byte)?;

    bw.write_vec_u8(deflate_helper::compress_deflate_bytes(input.as_slice())?)?;

    bw.write_u32(adler32(input)?)?;

    util::convert_num(bw.position()? - start)
}

/// Reads a Zlib block from a `BinaryReader` and returns the uncompressed data
pub fn read_zlib<R>(br: &mut BinaryReader<R>, compressed_size: u64) -> io::Result<Vec<u8>>
where
    R: Read + Seek,
{
    br.assert_u8(&[0x78])?;
    br.assert_u8(&[0x01, 0x5E, 0x9C, 0xDA])?;

    deflate_helper::decompress_deflate_bytes(&br.read_vec_u8(compressed_size - 2)?)
}

fn adler32(data: &Vec<u8>) -> Result<u32, io::Error> {
    let mut adler_a: u32 = 1;
    let mut adler_b: u32 = 0;

    for byte in data {
        adler_a = (adler_a + util::convert_num::<u8, u32>(*byte)?) % 65521;
        adler_b = (adler_b + adler_a) % 65521;
    }

    Ok((adler_b << 16) | adler_a)
}
