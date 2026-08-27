#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32
}

impl Vector2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vector2 { x, y }
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl Vector3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vector3 { x, y, z }
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32
}

impl Vector4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Vector4 { x, y, z, w }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Endian {
    Big,
    Little
}