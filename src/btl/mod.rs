use std::io::{self, Read, Seek, Write};

use crate::{
    SoulsFile, dcx::compression_info::*, format::SoulsFileInternal, io::{BinaryReader, BinaryWriter, Endian}, util,
};

pub mod light;

pub use light::Light;
pub use light::LightType;

/// Point light sources in a map, used in BB, DS3, and Sekiro
pub struct BTL {
    /// Compression of this BTL
    pub compression: CompressionInfo,
    /// Version
    pub version: i32,
    /// Whether offsets are 64-bit; set to false for Dark Souls 2
    pub offsets_64bit: bool,
    /// Light sources in this BT
    pub lights: Vec<Light>,
}

impl BTL {
    pub fn new(version: i32, offsets_64bit: bool) -> Self {
        Self {
            compression: CompressionInfo::None,
            version,
            offsets_64bit,
            lights: Vec::new(),
        }
    }
}

impl SoulsFileInternal<BTL> for BTL {
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let (mut reader, compression) = util::get_decompressed_binary_reader(br)?;
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
        reader.set_varint_64bit(light_res);

        let names_start = reader.position()?;
        reader.skip(names_length as i64)?;
        let mut lights: Vec<Light> = Vec::with_capacity(lights_count as usize);

        for _ in 0..lights_count {
            lights.push(Light::read(
                &mut reader,
                util::try_from_to_io_result(names_start)?,
                version,
            )?);
        }

        Ok(Self {
            compression,
            version,
            offsets_64bit,
            lights,
        })
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bw.set_endian(Endian::Little);
        bw.set_varint_64bit(self.offsets_64bit);

        bw.write_i32(2)?;
        bw.write_i32(self.version)?;
        bw.write_i32(util::try_from_to_io_result(self.lights.len())?)?;
        bw.reserve_i32("names_length")?;
        bw.write_i32(0)?;
        let next_var = if self.version >= 16 {
            0xE8
        } else if self.offsets_64bit {
            0xC8
        } else {
            0xC0
        };
        bw.write_i32(next_var)?;
        bw.write_pattern(0x24, 0x00)?;

        let names_start = bw.position()?;
        let mut name_offsets: Vec<i64> = Vec::with_capacity(self.lights.len());

        for entry in &self.lights {
            let name_offset: i64 = util::try_from_to_io_result(bw.position()? - names_start)?;
            name_offsets.push(name_offset);
            bw.write_utf16(&entry.name, true)?;
            if name_offset % 0x10 != 0 {
                bw.write_pattern(util::try_from_to_io_result(0x10 - (name_offset % 0x10))?, 0x00)?;
            }
        }

        let pos = bw.position()?;
        bw.fill_i32("names_length", util::try_from_to_io_result(pos - names_start)?)?;

        for i in 0..self.lights.len() {
            self.lights[i].write(bw, name_offsets[i])?;
        }

        Ok(())
    }

    fn get_compression(&self) -> CompressionInfo {
        self.compression
    }
}

impl SoulsFile<BTL> for BTL {}
