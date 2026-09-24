use std::io;

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FormatFlags1: u8 {
        const None = 0;
        const Flag01 = 0b0000_0001;
        const DataOffsets_32bit = 0b0000_0010;
        const DataOffsets_64bit = 0b0000_0100;
        const Flag08 = 0b0000_1000;
        const Flag10 = 0b0001_0000;
        const Flag20 = 0b0010_0000;
        const Flag40 = 0b0100_0000;
        const OffsetParamType = 0b1000_0000;
    }
}

impl TryFrom<u8> for FormatFlags1 {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid format flags"))
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FormatFlags2: u8 {
        const None = 0;
        const UnicodeRowNames = 0b0000_0001;
        const Flag02 = 0b0000_0010;
        const Flag04 = 0b0000_0100;
        const Flag08 = 0b0000_1000;
        const Flag10 = 0b0001_0000;
        const Flag20 = 0b0010_0000;
        const Flag40 = 0b0100_0000;
        const Flag80 = 0b1000_0000;
    }
}

impl TryFrom<u8> for FormatFlags2 {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid format flags"))
    }
}
