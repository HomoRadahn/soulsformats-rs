mod format;
mod reader;
mod writer;

pub(crate) use format::StreamIO;
pub use format::{ByteIO, FileIO};
pub(crate) use reader::BinaryReader;
pub(crate) use writer::BinaryWriter;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(dead_code)]
/// A collection of two `f32` numbers
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(dead_code)]
/// A collection of three `f32` numbers
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(dead_code)]
/// A collection of four `f32` numbers
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(dead_code)]
/// A collection of four `u8` numbers. When used to represent color, the value of `w` represents alpha
pub struct ByteVector4 {
    pub x: u8,
    pub y: u8,
    pub z: u8,
    /// Used to represent alpha in colors
    pub w: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(dead_code)]
/// A collection of three `u8` numbers
pub struct ByteVector3 {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

#[derive(Debug, Clone, Copy)]
/// Represents data endianness. Also used for order of bits in BND flags
pub enum Endian {
    Big,
    Little,
}
