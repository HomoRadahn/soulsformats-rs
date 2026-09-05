use std::io::{self, Cursor, ErrorKind, Read, Seek};

use crate::io::Endian;
use crate::{
    DCX,
    dcx::compression_info::{CompressionInfo},
    io::BinaryReader,
};

pub(crate) fn get_decompressed_binary_reader<R>(
    br: &mut BinaryReader<R>,
) -> io::Result<(BinaryReader<Cursor<Vec<u8>>>, CompressionInfo)>
where
    R: Read + Seek,
{
    if DCX::is(br)? {
        let len = br.length()?;
        let dcx = DCX::decompress_bytes(br.get_u8_vec(0, len)?)?;
        return Ok((
            BinaryReader::from_bytes(dcx.data, Endian::Little, false),
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

/// Calls the `R::try_from` function on `I`, returning `io::Result<R>`
pub(crate) fn try_from_to_io_result<I, R>(num: I) -> io::Result<R>
where
    R: TryFrom<I>,
{
    R::try_from(num).map_err(|_| io::Error::new(
        ErrorKind::InvalidData,
        format!("Error parsing {} into {}", std::any::type_name::<I>(), std::any::type_name::<R>()),
    ))
}