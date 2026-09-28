use std::io;

use bitflags::bitflags;

#[allow(non_camel_case_types)]
/// Supported primitive field types, named after XML paramdef types
pub enum ParamDefType {
    s8,
    u8,
    s16,
    u16,
    s32,
    u32,
    /// 4 bytes representing `bool`
    b32,
    f32,
    /// 4 bytes representing `f32`, used for angles
    angle32,
    /// 8 bytes representing `f64`
    f64,
    /// Byte or array of bytes used for padding / placeholding
    dummy8,
    /// Fixed-width Shift-JIS string
    fixstr,
    /// Fixed-width UTF-16 string
    fixstrW
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