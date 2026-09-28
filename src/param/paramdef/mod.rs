use std::io::{self, ErrorKind::InvalidData, Read, Seek, Write};

use crate::{
    ByteIO, FileIO,
    io::{BinaryReader, BinaryWriter, Endian, StreamIO},
};

pub mod field;
pub mod format;
use field::Field;

pub struct ParamDef {
    /// Indicates a revision of the row data structure
    pub data_version: i16,
    /// Identifies corresponding `Param` and `ParamDEF`
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
    /// ParamDEF is "regulation version aware" and can be applied to older regulation params that may have a
    /// different layout that the latest params if the XML paramdef supports it.
    pub version_aware: bool,
    /// Only basic fields are present;
    /// This is used in Armored Core Formula Front for PS2.
    pub basic_fields: bool,
}

impl Default for ParamDef {
    fn default() -> Self {
        Self {
            data_version: Default::default(),
            param_type: Default::default(),
            endian: Endian::Little,
            unicode: Default::default(),
            format_version: 104,
            fields: Default::default(),
            version_aware: Default::default(),
            basic_fields: Default::default(),
        }
    }
}

impl ParamDef {
    /// Whether field default, minimum, maximum, and increment may be variable type. If false, they are always floats.
    pub fn variable_editor_value_types(&self) -> bool {
        self.format_version >= 203
    }
}

impl StreamIO<ParamDef> for ParamDef {
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<ParamDef>
    where
        R: Read + Seek,
    {
        let mut out = Self::default();

        out.endian = match br.get_i8(0x2C)? == -1 {
            true => Endian::Big,
            false => Endian::Little,
        };

        br.endian = out.endian;
        out.format_version = br.get_i16(0x2E)?;
        br.varint_64bit = out.format_version >= 200;

        br.read_i32()?;
        let header_size = br.assert_i16(&[0x30, 0xFF])?;
        out.data_version = br.read_i16()?;
        let field_count = br.read_i16()?;
        let field_size = br.assert_i16(&[0x48, 0x68, 0x6C, 0x88, 0x8C, 0xAC, 0xB0, 0xD0])?;

        if out.format_version >= 202 {
            br.assert_i32(&[0])?;
            let len = br.read_i64()?;
            out.param_type = br.get_shift_jis(len as u64)?;
            br.assert_i64(&[0])?;
            br.assert_i64(&[0])?;
            br.assert_i32(&[0])?;
        } else if out.format_version >= 106 && out.format_version < 200 {
            let len = br.read_i32()?;
            out.param_type = br.get_shift_jis(len as u64)?;
            br.assert_i64(&[0])?;
            br.assert_i64(&[0])?;
            br.assert_i64(&[0])?;
            br.assert_i32(&[0])?;
        } else {
            out.param_type = br.read_fix_str(0x20)?;
        }

        br.assert_i8(&[0, -1])?; // Endianness
        out.unicode = br.read_bool()?;
        br.assert_i16(&[0, 101, 102, 103, 104, 106, 201, 202, 203])?; // Format version
        if out.format_version >= 200 {
            br.assert_i64(&[0x38])?;
        }

        if !(out.format_version < 200 && header_size == 0x30
            || out.format_version >= 200 && header_size == 0xFF)
        {
            return Err(io::Error::new(
                InvalidData,
                format!(
                    "Unexpected header size 0x{header_size:X} for version {}",
                    out.format_version
                ),
            ));
        }

        out.basic_fields = out.format_version == 0 && field_size == 0x68;

        // Currently omitting format_version == 103, as SoulsFormatsNEXT labels its corresponding field_size as incorrect
        if !(out.basic_fields
            || out.format_version == 101 && field_size == 0x8C
            || out.format_version == 102 && field_size == 0xAC
            || out.format_version == 104 && field_size == 0xB0
            || out.format_version == 106 && field_size == 0x48
            || out.format_version == 201 && field_size == 0xD0
            || out.format_version == 202 && field_size == 0x68
            || out.format_version == 203 && field_size == 0x88)
        {
            return Err(io::Error::new(
                InvalidData,
                format!(
                    "Unexpected header size 0x{header_size:X} for version {}",
                    out.format_version
                ),
            ));
        }

        for _ in 0..field_count {
            out.fields.push(Field::from_binary_reader(br, &out)?);
        }

        Ok(out)
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        todo!()
    }
}

impl ByteIO<ParamDef> for ParamDef {}
impl FileIO<ParamDef> for ParamDef {}
