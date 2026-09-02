#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CompressionInfo {
    Unknown,
    None,
    Zlib,
    DcpEdge,
    DcpDflt,
    DcxEdge,
    DcxDflt(DcxDfltCompressionArgs),
    DcxKrak,
    DcxZstd(u8),
}

impl CompressionInfo {
    /// Initializes `DcxDfltCompressionInfo` from given values
    pub fn new_dcx_dflt(unk04: i32, unk10: i32, unk14: i32, unk30: i32, unk38: i32) -> Self {
        Self::DcxDflt( 
            DcxDfltCompressionArgs {
                unk_04: unk04,
                unk_10: unk10,
                unk_14: unk14,
                unk_30: unk30,
                unk_38: unk38,
            }
        )
    }

    /// Initializes `DcxDfltCompressionInfo` from given preset
    pub fn dcx_dflt_from_preset(preset: DcxDfltCompressionPreset) -> Self {
        match preset {
            DcxDfltCompressionPreset::DcxDflt10000_24_9 => Self::DcxDflt(
                DcxDfltCompressionArgs {
                    unk_04: 0x10000,
                    unk_10: 0x24,
                    unk_14: 0x2C,
                    unk_30: 9,
                    unk_38: 0,
                }
            ),
            DcxDfltCompressionPreset::DcxDflt10000_44_9 => Self::DcxDflt(
                DcxDfltCompressionArgs {
                    unk_04: 0x10000,
                    unk_10: 0x44,
                    unk_14: 0x4C,
                    unk_30: 9,
                    unk_38: 0,
                }
            ),
            DcxDfltCompressionPreset::DcxDflt11000_44_8 => Self::DcxDflt(
                DcxDfltCompressionArgs {
                    unk_04: 0x11000,
                    unk_10: 0x44,
                    unk_14: 0x4C,
                    unk_30: 8,
                    unk_38: 0,
                }
            ),
            DcxDfltCompressionPreset::DcxDflt11000_44_9 => Self::DcxDflt(
                DcxDfltCompressionArgs {
                    unk_04: 0x11000,
                    unk_10: 0x44,
                    unk_14: 0x4C,
                    unk_30: 9,
                    unk_38: 0,
                }
            ),
            DcxDfltCompressionPreset::DcxDflt11000_44_9_15 => Self::DcxDflt(
                DcxDfltCompressionArgs {
                    unk_04: 0x11000,
                    unk_10: 0x44,
                    unk_14: 0x4C,
                    unk_30: 8,
                    unk_38: 15,
                }
            ),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum DcxDfltCompressionPreset {
    DcxDflt10000_24_9,
    DcxDflt10000_44_9,
    DcxDflt11000_44_8,
    DcxDflt11000_44_9,
    DcxDflt11000_44_9_15,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcxDfltCompressionArgs {
    pub unk_04: i32,
    pub unk_10: i32,
    pub unk_14: i32,
    pub unk_30: i32,
    pub unk_38: i32,
}

#[derive(Clone)]
pub enum KrakCompressionPreset {
    EldenRing,
    ArmoredCore6,
}

pub struct EdgeChunk {
    pub compressed_offset: i32,
    pub compressed_length: i32,
    pub is_compressed: bool,
}
