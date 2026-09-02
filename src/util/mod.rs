use std::io::{self, Cursor, Read, Seek};

use crate::io::Endian;
use crate::{
    DCX,
    dcx::compression_info::{CompressionInfo},
    io::BinaryReader,
};

pub fn get_decompressed_binary_reader<R>(
    br: &mut BinaryReader<R>,
) -> io::Result<(BinaryReader<Cursor<Vec<u8>>>, CompressionInfo)>
where
    R: Read + Seek,
{
    if DCX::is(br)? {
        let len = br.length()?;
        let dcx = DCX::decompress_bytes(br.get_u8_vec(0, len)?)?;
        return Ok((
            BinaryReader::from_bytes(dcx.decompressed, Endian::Little, false),
            dcx.compression,
        ));
    } else {
        let len = br.length()?;
        return Ok((
            BinaryReader::from_bytes(br.get_u8_vec(0, len)?, Endian::Little, false),
            CompressionInfo::None,
        ));
    }
}