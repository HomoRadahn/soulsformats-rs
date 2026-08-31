use std::io::{self, Seek, Write, Read};

use crate::dcx::compression_info::*;
use crate::io::BinaryReader;
use crate::io::BinaryWriter;
use crate::io::Endian;
use crate::util::Util;

pub mod light;

pub use light::Light;
pub use light::LightType;

pub struct BTL {
    pub compression: Box<dyn CompressionInfo>,
    pub version: i32,
    pub offsets_64bit: bool,
    pub lights: Vec<Light>,
}

impl BTL {
    pub fn new(version: i32, offsets_64bit: bool) -> Self {
        Self { compression: Box::new(NoCompressionInfo), version, offsets_64bit, lights: Vec::new() }
    }

    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<Self>
    where 
        R: Read + Seek
    {
        let (mut reader, compression) = Util::get_decompressed_binary_reader(br)?;
        
        reader.set_endian(Endian::Little);

        reader.assert_i32(&[2])?;
        let version = reader.assert_i32(&[1, 2, 5, 6, 16, 18])?;
        let lights_count = reader.read_i32()?;
        let names_length = reader.read_i32()?;
        reader.assert_i32(&[0])?;
        let light_size = reader.assert_i32(&[0xC0, 0xC8, 0xE8])?;
        reader.assert_pattern(0x24, 0x00)?;
        let light_res = light_size != 0xC0;
        let offsets_64bit = light_res.clone();
        reader.set_varint_behavior(light_res);

        let names_start = reader.position()?;
        reader.skip(i64::try_from(names_length).unwrap())?;
        let mut lights: Vec<Light> = Vec::with_capacity(lights_count as usize);
        
        for _ in 0..lights_count {
            lights.push(Light::read(&mut reader, i64::try_from(names_start).unwrap(), version)?);
        };

        Ok(
            Self { compression, version, offsets_64bit, lights }
        )
    }

    /// Reads a new `BTL` from bytes, decompressing as necessary
    pub fn from_bytes(data: Vec<u8>) -> io::Result<Self> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        BTL::read(&mut br)
    }

    /// Reads a `BTL` from a file, decompressing as necessary
    pub fn from_file(path: &str) -> io::Result<Self> {
        let mut br = BinaryReader::from_file(path, Endian::Little, false)?;
        BTL::read(&mut br)
    }

    fn write<W>(bw: &mut BinaryWriter<W>, offsets_64bit: bool) -> io::Result<()>
    where 
        W: Write + Seek
    {
        bw.set_endian(Endian::Big);
        bw.set_varint_behavior(offsets_64bit);
        Ok(())
    }

    /// Writes the `BTL` to bytes, compressing as necessary
    pub fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Big, true);
        BTL::write(&mut bw, self.offsets_64bit)?;
        bw.close_bytes()
    }

    pub fn to_file(&self, path: &str) -> io::Result<()> {
        let mut bw = BinaryWriter::to_file(path, Endian::Big, true)?;
        BTL::write(&mut bw, self.offsets_64bit)
    }
}