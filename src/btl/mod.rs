use crate::dcx::compression_info::CompressionInfo;

pub mod light;

pub use light::Light;
pub use light::LightType;

pub struct BTL {
    pub compression: Box<dyn CompressionInfo>,
    pub version: i32,
    pub offsets_64bit: bool,
    pub lights: Vec<Light>,
}
