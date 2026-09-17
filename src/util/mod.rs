use std::io::{self, Cursor, ErrorKind, Read, Seek};

use crate::io::Endian;
use crate::{DCX, dcx::compression_info::CompressionInfo, io::BinaryReader};

/// Decompresses data from `BinaryReader` if necessary, returning a new `BinaryReader` over bytes
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
    R::try_from(num).map_err(|_| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "Error parsing {} into {}",
                std::any::type_name::<I>(),
                std::any::type_name::<R>()
            ),
        )
    })
}

/// FromSoft's basic filename hashing algorithm, used in some BND and BXF formats
pub(crate) fn from_path_hash(text: impl Into<String>) -> u32 {
    let text_into = text.into();
    let mut hashable = text_into.to_lowercase().replace("\\", "/");
    if hashable.chars().next() != Some('/') {
        hashable = format!("/{}", hashable);
    };
    hashable.chars().fold(0u32, |i, c| i * 37 + c as u32)
}

/// Determines whether a number is prime or not
pub(crate) fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }

    if n == 2 {
        return true;
    }

    if n % 2 == 0 {
        return false;
    }

    for i in 2..=(n as f64).sqrt() as u32 {
        if n % i == 0 {
            return false;
        }
    }

    true
}
