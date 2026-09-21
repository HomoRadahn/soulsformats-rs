use std::io::{self};
use bitflags::bitflags;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An enum for the different supported file path modes
pub enum FilePathMode {
    /// Files in this BND have no name
    Nameless = 0,
    /// Files in this BND only have file names
    FileName = 1,
    /// All files use a full file path
    FullPath = 2,
    /// Add a base directory all paths start from, then write the rest of the path as each file name
    BaseDirectory = 3,
}

impl TryFrom<u8> for FilePathMode {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Nameless),
            1 => Ok(Self::FileName),
            2 => Ok(Self::FullPath),
            3 => Ok(Self::BaseDirectory),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid enum value",
            )),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    /// Header Info flags describing what features are enabled
    pub struct HeaderInfoFlags: u8 {
        const HeaderItem = 0b00000001;
        const Endian = 0b00000010;
        const FileVersion = 0b00000100;
        const FileSize = 0b00001000;
        const FileNum = 0b00010000;
        const BaseDirOffset = 0b00100000;
        const AlignmentSize = 0b01000000;
        const Option = 0b10000000;
    }
}

impl TryFrom<u8> for HeaderInfoFlags {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid format flags"))
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    /// File Info flags describing what features are enabled
    pub struct FileInfoFlags: u8 {
        const ID = 0b00000001;
        const Offset = 0b00000010;
        const Size = 0b00000100;
        const NameOffset = 0b00001000;
        const Flag5 = 0b00010000;
        const Flag6 = 0b00100000;
        const Flag7 = 0b01000000;
        const Flag8 = 0b10000000;
    }
}

impl TryFrom<u8> for FileInfoFlags {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid format flags"))
    }
}
