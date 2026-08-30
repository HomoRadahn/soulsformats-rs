use std::io::{self, ErrorKind::InvalidData};

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
    DcxZstd,
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

    fn get_dcx_dflt_args(&self) -> io::Result<DcxDfltCompressionArgs> {
        Err(io::Error::new(InvalidData, "not implemented for this type"))
    }

    fn get_dcx_zstd_args(&self) -> io::Result<u8> {
        Err(io::Error::new(InvalidData, "not implemented for this type"))
    }
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
    pub args: DcxDfltCompressionArgs,
}

#[derive(Clone, Copy)]
pub struct DcxDfltCompressionArgs {
    pub unk_04: i32,
    pub unk_10: i32,
    pub unk_14: i32,
    pub unk_30: i32,
    pub unk_38: i32,
}

impl CompressionInfo for DcxDfltCompressionInfo {
    fn get_type(&self) -> Type {
        Type::DcxDflt
    }

    fn get_dcx_dflt_args(&self) -> io::Result<DcxDfltCompressionArgs> {
        Ok(self.args)
    }
}

impl DcxDfltCompressionInfo {
    /// Initializes `DcxDfltCompressionInfo` from given values
    pub fn new(unk04: i32, unk10: i32, unk14: i32, unk30: i32, unk38: i32) -> Self {
        Self {
            args: DcxDfltCompressionArgs {
                unk_04: unk04,
                unk_10: unk10,
                unk_14: unk14,
                unk_30: unk30,
                unk_38: unk38,
            },
        }
    }

    /// Initializes `DcxDfltCompressionInfo` from given preset
    pub fn from_preset(preset: DfltCompressionPreset) -> Self {
        match preset {
            DfltCompressionPreset::DcxDflt10000_24_9 => Self {
                args: DcxDfltCompressionArgs {
                    unk_04: 0x10000,
                    unk_10: 0x24,
                    unk_14: 0x2C,
                    unk_30: 9,
                    unk_38: 0,
                },
            },
            DfltCompressionPreset::DcxDflt10000_44_9 => Self {
                args: DcxDfltCompressionArgs {
                    unk_04: 0x10000,
                    unk_10: 0x44,
                    unk_14: 0x4C,
                    unk_30: 9,
                    unk_38: 0,
                },
            },
            DfltCompressionPreset::DcxDflt11000_44_8 => Self {
                args: DcxDfltCompressionArgs {
                    unk_04: 0x11000,
                    unk_10: 0x44,
                    unk_14: 0x4C,
                    unk_30: 8,
                    unk_38: 0,
                },
            },
            DfltCompressionPreset::DcxDflt11000_44_9 => Self {
                args: DcxDfltCompressionArgs {
                    unk_04: 0x11000,
                    unk_10: 0x44,
                    unk_14: 0x4C,
                    unk_30: 9,
                    unk_38: 0,
                },
            },
            DfltCompressionPreset::DcxDflt11000_44_9_15 => Self {
                args: DcxDfltCompressionArgs {
                    unk_04: 0x11000,
                    unk_10: 0x44,
                    unk_14: 0x4C,
                    unk_30: 8,
                    unk_38: 15,
                },
            },
        }
    }
}

pub enum KrakCompressionPreset {
    EldenRing,
    ArmoredCore6,
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
    pub compression_level: u8,
}

impl CompressionInfo for DcxZstdCompressionInfo {
    fn get_type(&self) -> Type {
        Type::DcxZstd
    }

    fn get_dcx_zstd_args(&self) -> io::Result<u8> {
        Ok(self.compression_level)
    }
}

impl DcxZstdCompressionInfo {
    pub fn new(compression_level: u8) -> Self {
        Self { compression_level }
    }
}

#[allow(dead_code)]
pub struct EdgeChunk {
    pub compressed_offset: i32,
    pub compressed_length: i32,
    pub is_compressed: bool,
}
