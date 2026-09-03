use crate::{bnd::binder::FileFlags, dcx::compression_info::CompressionInfo};

/// A generic file in `BND3`, `BND4`, `BXF3`, `BXF4` containers
pub struct BinderFile {
    pub flags: FileFlags,
    pub id: i32,
    pub name: String,
    pub bytes: Vec<u8>,
    pub compression: CompressionInfo
}

impl BinderFile {
    /// Creates a new `BinderFile`
    pub fn new(flags: FileFlags, id: i32, name: String, bytes: Vec<u8>) -> Self {
        Self {
            flags, id, name, bytes, compression: CompressionInfo::Zlib
        }
    }
}

pub struct BinderFileHeader {
    
}