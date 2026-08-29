pub mod writer;
pub mod reader;

pub use writer::BinaryWriter;
pub use reader::BinaryReader;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ByteVector4 {
    pub x: u8,
    pub y: u8,
    pub z: u8,
    pub w: u8
}

#[derive(Debug, Clone, Copy)]
pub enum Endian {
    Big,
    Little
}