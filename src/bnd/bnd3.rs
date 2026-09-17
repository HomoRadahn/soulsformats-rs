use crate::dcx::compression_info::CompressionInfo;

/// A general-purpose file container used before DS2
pub struct BND3 {
    /// DCX Compression info
    pub compression: CompressionInfo,
}
