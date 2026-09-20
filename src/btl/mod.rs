use std::io::{self, Read, Seek, Write};

use crate::{
    ByteIO, FileIO,
    io::{BinaryReader, BinaryWriter, Endian, StreamIO},
    util,
};

pub mod light;

pub use light::Light;
pub use light::LightType;

/// Point light sources in a map, used in BB, DS3, and Sekiro
#[derive(Debug, Clone, PartialEq)]
pub struct BTL {
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
            version,
            offsets_64bit,
            lights: Vec::new(),
        }
    }
}

impl StreamIO<BTL> for BTL {
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        br.endian = Endian::Little;
        br.assert_i32(&[2])?;
        let version = br.assert_i32(&[1, 2, 5, 6, 16, 18])?;
        let lights_count = br.read_i32()?;
        let names_length = br.read_i32()?;
        br.assert_i32(&[0])?;
        let light_size = br.assert_i32(&[0xC0, 0xC8, 0xE8])?;
        br.assert_pattern(0x24, 0x00)?;
        let light_res = light_size != 0xC0;
        let offsets_64bit = light_res;
        br.varint_64bit = light_res;

        let names_start = br.position()?;
        br.skip(names_length as i64)?;
        let mut lights: Vec<Light> = Vec::with_capacity(lights_count as usize);

        for _ in 0..lights_count {
            lights.push(Light::read(br, util::convert_num(names_start)?, version)?);
        }

        Ok(Self {
            version,
            offsets_64bit,
            lights,
        })
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bw.endian = Endian::Little;
        bw.varint_64bit = self.offsets_64bit;

        bw.write_i32(2)?;
        bw.write_i32(self.version)?;
        bw.write_i32(util::convert_num(self.lights.len())?)?;
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
            let name_offset: i64 = util::convert_num(bw.position()? - names_start)?;
            name_offsets.push(name_offset);
            bw.write_utf16(&entry.name, true)?;
            if name_offset % 0x10 != 0 {
                bw.write_pattern(util::convert_num(0x10 - (name_offset % 0x10))?, 0x00)?;
            }
        }

        let pos = bw.position()?;
        bw.fill_i32("names_length", util::convert_num(pos - names_start)?)?;

        for (index, light) in self.lights.iter().enumerate() {
            light.write(bw, name_offsets[index])?;
        }

        Ok(())
    }
}

impl ByteIO<BTL> for BTL {}
impl FileIO<BTL> for BTL {}
