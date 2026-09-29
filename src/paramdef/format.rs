use std::io;

use bitflags::bitflags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Supported field types
pub enum ParamDefType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    /// Represented by 4 bytes
    Bool,
    F32,
    /// Represented by `f32`
    Angle,
    F64,
    /// Byte or array of bytes used for padding / placeholding
    ArrayU8,
    /// Fixed-width Shift-JIS string
    StringShiftJIS,
    /// Fixed-width UTF-16 string
    StringUTF16,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EditFlags: u8 {
        /// Value is editable and does not wrap
        const None = 0;
        /// Value wraps around when scrolled past the minimum or maximum
        const Wrap = 1;
        /// Value may not be edited
        const Lock = 4;
    }
}

impl TryFrom<u8> for EditFlags {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid format flags"))
    }
}
