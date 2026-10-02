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

#[derive(Debug, Clone, PartialEq)]
/// Values each cell can take, each holding the value itself within
pub enum CellValue {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    Bool(bool),
    F32(f32),
    Angle(f32),
    F64(f64),
    /// Byte or array of bytes used for padding / placeholding
    ArrayU8(Vec<u8>),
    /// Fixed-width Shift-JIS string
    StringShiftJIS(String),
    /// Fixed-width UTF-16 string
    StringUTF16(String),
}

/// Assigns a value to the payload of a matching [`CellValue`] variant.
pub trait CellValueExt {
    /// Updates the payload, returning an error if the current variant is incompatible.
    fn set_inner(self, target: &mut CellValue) -> io::Result<()>;
}

macro_rules! impl_cell_value_inner {
    ($inner:ty, $variant:ident) => {
        impl CellValueExt for $inner {
            fn set_inner(self, target: &mut CellValue) -> io::Result<()> {
                match target {
                    CellValue::$variant(value) => {
                        *value = self;
                        Ok(())
                    }
                    _ => Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        concat!("value does not match CellValue::", stringify!($variant)),
                    )),
                }
            }
        }
    };
}

impl_cell_value_inner!(i8, I8);
impl_cell_value_inner!(u8, U8);
impl_cell_value_inner!(i16, I16);
impl_cell_value_inner!(u16, U16);
impl_cell_value_inner!(i32, I32);
impl_cell_value_inner!(u32, U32);
impl_cell_value_inner!(bool, Bool);
impl_cell_value_inner!(Vec<u8>, ArrayU8);

impl CellValueExt for f32 {
    fn set_inner(self, target: &mut CellValue) -> io::Result<()> {
        match target {
            CellValue::F32(value) | CellValue::Angle(value) => {
                *value = self;
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "value does not match CellValue::F32 or CellValue::Angle",
            )),
        }
    }
}

impl CellValueExt for f64 {
    fn set_inner(self, target: &mut CellValue) -> io::Result<()> {
        match target {
            CellValue::F64(value) => {
                *value = self;
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "value does not match CellValue::F64",
            )),
        }
    }
}

impl CellValueExt for String {
    fn set_inner(self, target: &mut CellValue) -> io::Result<()> {
        match target {
            CellValue::StringShiftJIS(value) | CellValue::StringUTF16(value) => {
                *value = self;
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "value does not match a CellValue string variant",
            )),
        }
    }
}
