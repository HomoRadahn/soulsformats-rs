use std::io::{self, Cursor, ErrorKind, Read, Seek};

use crate::io::Endian;
use crate::{DCX, dcx::compression_info::CompressionInfo, io::BinaryReader};

/// Decompresses data from `BinaryReader` if necessary, returning a new `BinaryReader` over bytes
pub fn get_decompressed_binary_reader<R>(
    br: &mut BinaryReader<R>,
) -> io::Result<(BinaryReader<Cursor<Vec<u8>>>, CompressionInfo)>
where
    R: Read + Seek,
{
    if DCX::is(br)? {
        let len = br.length()?;
        let dcx = DCX::from_bytes(br.get_vec_u8(0, len)?)?;
        Ok((
            BinaryReader::from_bytes(dcx.data, Endian::Little, false),
            dcx.compression,
        ))
    } else {
        let len = br.length()?;
        Ok((
            BinaryReader::from_bytes(br.get_vec_u8(0, len)?, Endian::Little, false),
            CompressionInfo::None,
        ))
    }
}

/// Calls the `R::try_from` function on `I`, returning `io::Result<R>`
pub(crate) fn convert_num<I, R>(num: I) -> io::Result<R>
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

/// FromSoft's basic filename hashing algorithm, used in `BND4` and `BXF4`
pub(crate) fn from_path_hash(text: impl Into<String>) -> u32 {
    let mut hashable = text.into().to_lowercase().replace("\\", "/");
    if !hashable.starts_with('/') {
        hashable = format!("/{}", hashable);
    };
    hashable.chars().fold(0u32, |hash, character| {
        hash.wrapping_mul(37).wrapping_add(character as u32)
    })
}

/// Determines whether a number is prime or not
pub(crate) fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }

    if n == 2 {
        return true;
    }

    if n.is_multiple_of(2) {
        return false;
    }

    for index in 2..=(n as f64).sqrt() as u32 {
        if n.is_multiple_of(index) {
            return false;
        }
    }

    true
}
