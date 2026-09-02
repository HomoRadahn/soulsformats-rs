use std::io::{self, Read, Seek};

use crate::io::{BinaryReader, Endian};

pub enum Format {
    /// Minimal file information.
    None = 0,

    /// File is big-endian regardless of the big-endian byte.
    BigEndian = 0b0000_0001,

    /// Files have ID numbers.
    IDs = 0b0000_0010,

    /// Files have name strings; Names2 may or may not be set. Perhaps the distinction is related to whether it's a full path or just the filename?
    Names1 = 0b0000_0100,

    /// Files have name strings; Names1 may or may not be set.
    Names2 = 0b0000_1000,

    /// File data offsets are 64-bit.
    LongOffsets = 0b0001_0000,

    /// Files may be compressed.
    Compression = 0b0010_0000,

    /// Unknown.
    Flag6 = 0b0100_0000,

    /// Unknown.
    Flag7 = 0b1000_0000,
}

impl TryFrom<u8> for Format {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            0b0000_0001 => Ok(Self::BigEndian),
            0b0000_0010 => Ok(Self::IDs),
            0b0000_0100 => Ok(Self::Names1),
            0b0000_1000 => Ok(Self::Names2),
            0b0001_0000 => Ok(Self::LongOffsets),
            0b0010_0000 => Ok(Self::Compression),
            0b0100_0000 => Ok(Self::Flag6),
            0b1000_0000 => Ok(Self::Flag7),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid enum value",
            )),
        }
    }
}

// /// Reads a binder format byte
pub fn read_format<R>(br: &mut BinaryReader<R>, bit_endian: Endian) -> io::Result<Format>
where 
    R: Read + Seek
{
    let raw_format = br.read_u8()?;

    let reverse = match bit_endian {
        Endian::Big => true,
        Endian::Little => false,
    } || (raw_format & 1) != 0 && (raw_format & 0b1000_0000) == 0;
}