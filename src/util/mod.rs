use std::io::{self, Cursor, Read, Seek};

use crate::dcx::compression_info::NoCompressionInfo;
use crate::io::Endian;
use crate::{DCX, dcx::compression_info::CompressionInfo, io::BinaryReader};

pub struct Util;

impl Util {
    pub fn get_decompressed_binary_reader<R>(
        mut br: BinaryReader<R>,
    ) -> io::Result<(BinaryReader<Cursor<Vec<u8>>>, Box<dyn CompressionInfo>)>
    where
        R: Read + Seek,
    {
        if DCX::is(&mut br)? {
            let (bytes, compression) = DCX::decompress(br)?;
            return Ok((
                BinaryReader::from_bytes(bytes, Endian::Little, true),
                compression,
            ));
        } else {
            let len = br.length()?;
            return Ok((
                BinaryReader::from_bytes(br.get_u8_vec(0, len)?, Endian::Little, false),
                Box::new(NoCompressionInfo),
            ));
        }
    }
}
