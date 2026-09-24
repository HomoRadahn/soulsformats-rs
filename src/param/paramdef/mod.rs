use std::io::{self, Read, Seek, Write};

use crate::{ByteIO, FileIO, io::{BinaryReader, BinaryWriter, Endian, StreamIO}};

pub mod field;
use field::Field;

pub struct PARAMDEF {
    /// Indicates a revision of the row data structure
    pub data_version: i16,
    /// Identifies corresponding `PARAM` and `PARAMDEF`
    pub param_type: String,
    /// Endianness of data
    pub endian: Endian,
    /// Write certain strings as UTF-16, otherwise Shift-JIS
    pub unicode: bool,
    /// Determines format of the file.
    //   0 - Armored Core Formula Front PS2, possibly not used yet
    // 101 - Enchanted Arms, Chromehounds, Armored Core 4/For Answer/V/Verdict Day, Shadow Assault: Tenchu
    // 102 - Demon's Souls
    // 103 - Ninja Blade, Another Century's Episode: R
    // 104 - Dark Souls, Steel Battalion: Heavy Armor
    // 106 - Elden Ring (deprecated ObjectParam)
    // 201 - Bloodborne
    // 202 - Dark Souls 3
    // 203 - Elden Ring, Armored Core 6
    pub format_version: i16,
    /// Fields in each param row, in order of appearance
    pub fields: Vec<Field>,
    /// PARAMDEF is "regulation version aware" and can be applied to older regulation params that may have a
    /// different layout that the latest params if the XML paramdef supports it.
    pub version_aware: bool,
    /// Only basic fields are present;
    /// This is used in Armored Core Formula Front for PS2.
    pub basic_fields: bool,
}

impl PARAMDEF {
    /// Whether field default, minimum, maximum, and increment may be variable type. If false, they are always floats.
    pub fn variable_editor_value_types(&self) -> bool {
        self.format_version >= 203
    }

    /// Creates an empty `PARAMDEF` formatted for DS1
    pub fn empty() -> Self {
        Self {
            data_version: 0,
            param_type: String::new(),
            endian: Endian::Little,
            unicode: false,
            format_version: 104,
            fields: Vec::new(),
            version_aware: false,
            basic_fields: false,
        }
    }
}

impl StreamIO<PARAMDEF> for PARAMDEF {
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<PARAMDEF>
    where
        R: Read + Seek
    {
        let mut out = Self::empty();

        out.endian = match br.get_i8(0x2C)? == -1 {
            true => Endian::Big,
            false => Endian::Little
        };

        br.endian = out.endian;

        out.format_version = br.get_i16(0x2E)?;
        

        Ok(out)
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek
    {
        todo!()
    }
}

impl ByteIO<PARAMDEF> for PARAMDEF {}
impl FileIO<PARAMDEF> for PARAMDEF {}
