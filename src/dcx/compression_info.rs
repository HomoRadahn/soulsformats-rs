#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Type {
    Unknown,
    None,
    Zlib,
    DcpEdge,
    DcpDflt,
    DcxEdge,
    DcxDflt,
    DcxKrak,
    DcxZstd
}

#[derive(PartialEq, Clone, Copy)]
pub enum DfltCompressionPreset {
    DcxDflt10000_24_9,
    DcxDflt10000_44_9,
    DcxDflt11000_44_8,
    DcxDflt11000_44_9,
    DcxDflt11000_44_9_15,
}

pub trait CompressionInfo {
    fn get_type(&self) -> Type;
}

pub struct UnkCompressionInfo;

impl CompressionInfo for UnkCompressionInfo {
    fn get_type(&self) -> Type {
        Type::Unknown
    }
}

pub struct NoCompressionInfo;

impl CompressionInfo for NoCompressionInfo {
    fn get_type(&self) -> Type {
        Type::None
    }
}

pub struct DcpDfltCompressionInfo;

impl CompressionInfo for DcpDfltCompressionInfo {
    fn get_type(&self) -> Type {
        Type::DcpDflt
    }
}

pub struct DcpEdgeCompressionInfo;

impl CompressionInfo for DcpEdgeCompressionInfo {
    fn get_type(&self) -> Type {
        Type::DcpEdge
    }
}

pub struct ZlibCompressionInfo;

impl CompressionInfo for ZlibCompressionInfo {
    fn get_type(&self) -> Type {
        Type::Zlib
    }
}

pub struct DcxEdgeCompressionInfo;

impl CompressionInfo for DcxEdgeCompressionInfo {
    fn get_type(&self) -> Type {
        Type::DcxEdge
    }
}

pub struct DcxDfltCompressionInfo {
    pub unk04: i32,
    pub unk10: i32,
    pub unk14: i32,
    pub unk30: i32,
    pub unk38: i32,
}

impl CompressionInfo for DcxDfltCompressionInfo {
    fn get_type(&self) -> Type {
        Type::DcxDflt
    }
}

impl DcxDfltCompressionInfo {
    /// Initializes `DcxDfltCompressionInfo` from given values
    pub fn new(unk04: i32, unk10: i32, unk14: i32, unk30: i32, unk38: i32) -> Self {
        Self { unk04, unk10, unk14, unk30, unk38 }
    }
    
    /// Initializes `DcxDfltCompressionInfo` from given preset
    pub fn from_preset(preset: DfltCompressionPreset) -> Self {
        match preset {
            DfltCompressionPreset::DcxDflt10000_24_9 => Self { unk04: 0x10000, unk10: 0x24, unk14: 0x2C, unk30: 9, unk38: 0 },
            DfltCompressionPreset::DcxDflt10000_44_9 => Self { unk04: 0x10000, unk10: 0x44, unk14: 0x4C, unk30: 9, unk38: 0 },
            DfltCompressionPreset::DcxDflt11000_44_8 => Self { unk04: 0x11000, unk10: 0x44, unk14: 0x4C, unk30: 8, unk38: 0 },
            DfltCompressionPreset::DcxDflt11000_44_9 => Self { unk04: 0x11000, unk10: 0x44, unk14: 0x4C, unk30: 9, unk38: 0 },
            DfltCompressionPreset::DcxDflt11000_44_9_15 => Self { unk04: 0x11000, unk10: 0x44, unk14: 0x4C, unk30: 8, unk38: 15 },
        }
    }
}

pub enum KrakCompressionPreset {
    EldenRing,
    ArmoredCore6
}

pub struct DcxKrakCompressionInfo;

impl CompressionInfo for DcxKrakCompressionInfo {
    fn get_type(&self) -> Type {
        unimplemented!()
    }
}

impl DcxKrakCompressionInfo {
    pub fn new() -> Self {
        unimplemented!()
    }
}

pub struct DcxZstdCompressionInfo {
    pub compression_level: u8
}

impl CompressionInfo for DcxZstdCompressionInfo {
    fn get_type(&self) -> Type {
        Type::DcxZstd
    }
}

impl DcxZstdCompressionInfo {
    pub fn new(compression_level: u8) -> Self {
        Self { compression_level }
    }
}

pub struct EdgeChunk {
    compressed_offset: u64,
    compressed_length: u64,
    is_compressed: bool
}