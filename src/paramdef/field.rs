use std::collections::HashMap;
use std::io::{self, ErrorKind::InvalidData, Read, Seek, Write};
use std::{cmp, fmt};

use crate::io::BinaryWriter;
use crate::param::Cell;
use crate::{
    ParamDef,
    io::BinaryReader,
    param::CellValue,
    paramdef::{self, EditFlags, ParamDefType},
    util,
};

use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
/// Information about a field present in each row in a param
pub struct Field {
    /// Name to display in the editor
    pub display_name: String,
    /// Type of value to display in the editor
    pub display_type: ParamDefType,
    /// Printf-style format string to apply to the value in the editor
    pub display_format: String,
    /// Default value for new rows; may be absent for variable editor value types
    pub default: CellValue,
    /// Minimum valid value; may be absent for variable editor value types
    pub min: CellValue,
    /// Maximum valid value; may be absent for variable editor value types
    pub max: CellValue,
    /// Amount of increase or decrease per step; may be absent for variable editor value types
    pub increment: CellValue,
    /// Flags determining behavior of the field
    pub edit_flags: EditFlags,
    /// Number of elements for array types; only supported for ArrayU8, StringShiftJIS, and StringUTF16
    pub array_length: Option<i32>,
    /// Optional description of the field; may be null
    pub description: Option<String>,
    /// Type of the value in the engine; may be an enum type
    pub internal_type: Option<String>,
    /// Name of the value in the engine; not present before version 102
    pub internal_name: Option<String>,
    /// Number of bits used by a bitfield; only supported for unsigned types, -1 when not used
    pub bit_size: Option<i32>,
    /// Fields are ordered by this value in the editor; not present before version 104
    pub sort_id: Option<i32>,
    pub unk_b8: Option<String>,
    pub unk_c0: Option<String>,
    pub unk_c8: Option<String>,
    pub first_regulation_version: Option<u64>,
    pub removed_regulation_version: Option<u64>,
}

impl Default for Field {
    fn default() -> Self {
        Self::new(ParamDefType::F32, "placeholder")
    }
}

impl Field {
    /// Creates a new `Field`, with specified parameters
    pub fn new(display_type: ParamDefType, internal_name: impl Into<String>) -> Self {
        let internal_name = internal_name.into();
        Self {
            display_name: internal_name.clone(),
            display_type: display_type,
            display_format: paramdef::util::get_default_format(display_type),
            default: paramdef::util::get_default_value(display_type),
            min: paramdef::util::get_default_minimum(display_type),
            max: paramdef::util::get_default_maximum(display_type),
            increment: paramdef::util::get_default_increment(display_type),
            edit_flags: paramdef::util::get_default_edit_flags(display_type),
            array_length: Some(1),
            description: Default::default(),
            internal_type: Some(format!("{:?}", display_type)),
            internal_name: Some(internal_name),
            bit_size: Default::default(),
            sort_id: Default::default(),
            unk_b8: Default::default(),
            unk_c0: Default::default(),
            unk_c8: Default::default(),
            first_regulation_version: Default::default(),
            removed_regulation_version: Default::default(),
        }
    }

    /// Checks if `Field` versions are valid for given regulation version
    pub fn is_valid_version(&self, version: u64) -> io::Result<bool> {
        let temp_first_version = self.first_regulation_version.unwrap_or(0);
        let temp_removed_version = self.removed_regulation_version.unwrap_or(0);

        Ok(version >= temp_first_version
            && (temp_removed_version == 0 || version < temp_removed_version))
    }

    /// Checks if `Field` versions are vaild for specified game version
    pub fn fits_game_version(&self, version: u64) -> io::Result<bool> {
        let temp_first_version = self.first_regulation_version.unwrap_or(0);
        let temp_removed_version = self.removed_regulation_version.unwrap_or(0);

        Ok(version == 0
            || temp_first_version <= version
                && (temp_removed_version == 0 || temp_removed_version > version))
    }

    pub(crate) fn read<R>(br: &mut BinaryReader<R>, paramdef: &ParamDef) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let mut out = Self::default();

        out.display_name =
            if paramdef.format_version >= 202 || (106..200).contains(&paramdef.format_version) {
                let pos = br.read_varint()?;
                br.get_utf16(pos as u64)?
            } else if paramdef.unicode {
                br.read_fixed_utf16(0x40)?
            } else {
                br.read_fixed_shift_jis(0x40)?
            };

        let display_type = br.read_fixed_shift_jis(8)?;
        out.display_type = match display_type.trim() {
            "s8" => ParamDefType::I8,
            "u8" => ParamDefType::U8,
            "s16" => ParamDefType::I16,
            "u16" => ParamDefType::U16,
            "s32" => ParamDefType::I32,
            "u32" => ParamDefType::U32,
            "b32" => ParamDefType::Bool,
            "f32" => ParamDefType::F32,
            "angle32" => ParamDefType::Angle,
            "f64" => ParamDefType::F64,
            "dummy8" => ParamDefType::ArrayU8,
            "fixstr" => ParamDefType::StringShiftJIS,
            "fixstrW" => ParamDefType::StringUTF16,
            unknown => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid ParamDef field type: {unknown}"),
                ));
            }
        };
        out.display_format = br.read_fixed_shift_jis(8)?;

        if paramdef.format_version >= 203 {
            br.assert_pattern(0x10, 0x00)?;
        } else {
            out.default = CellValue::F32(br.read_f32()?);
            out.min = CellValue::F32(br.read_f32()?);
            out.max = CellValue::F32(br.read_f32()?);
            out.increment = CellValue::F32(br.read_f32()?);
        }

        out.edit_flags = EditFlags::try_from(util::convert_num::<i32, u8>(br.read_i32()?)?)?;

        let byte_count = br.read_i32()?;

        if !paramdef::util::is_array_type(out.display_type)
            && byte_count != paramdef::util::get_value_size(out.display_type)
            || paramdef::util::is_array_type(out.display_type)
                && byte_count % paramdef::util::get_value_size(out.display_type) != 0
        {
            return Err(io::Error::new(
                InvalidData,
                format!(
                    "Unexpected byte count {byte_count} for type: {:?}",
                    out.display_type
                ),
            ));
        }

        out.array_length = if paramdef::util::is_array_type(out.display_type) {
            Some(byte_count / paramdef::util::get_value_size(out.display_type))
        } else {
            None
        };

        if paramdef.basic_fields {
            out.internal_name = None;
            out.internal_type = None;
            out.bit_size = None;
            return Ok(out);
        }

        let description_offset = br.read_varint()?;

        out.description = if description_offset != 0 {
            Some(if paramdef.unicode {
                br.get_utf16(description_offset as u64)?
            } else {
                br.get_shift_jis(description_offset as u64)?
            })
        } else {
            None
        };

        out.internal_type =
            if paramdef.format_version >= 202 || (106..200).contains(&paramdef.format_version) {
                let pos = br.read_varint()?;
                Some(br.get_ascii(pos as u64)?.trim().into())
            } else {
                Some(br.read_fixed_shift_jis(0x20)?.trim().into())
            };

        if paramdef.format_version >= 102 {
            out.internal_name = if paramdef.format_version >= 202
                || (106..200).contains(&paramdef.format_version)
            {
                let pos = br.read_varint()?;
                Some(br.get_ascii(pos as u64)?.trim().into())
            } else {
                Some(br.read_fixed_shift_jis(0x20)?.trim().into())
            };

            let re = Regex::new(r"^\s*(?<name>.+?)\s*\:\s*(?<size>\d+)\s*$").unwrap();

            // Call bare .unwrap(), because out.internal_name is set to contain a Some() (line 182)
            if let Some(captures) = re.captures(out.internal_name.clone().unwrap().as_str()) {
                out.internal_name = Some(captures["name"].into());
                out.bit_size = Some(
                    captures["size"]
                        .parse()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                );
            }

            if paramdef::util::is_array_type(out.display_type) {
                let re = Regex::new(r"^\s*(?<name>.+?)\s*\[\s*(?<length>\d+)\s*\]\s*$").unwrap();

                let length = if let Some(captures) =
                    re.captures(out.internal_name.clone().unwrap().as_str())
                {
                    out.internal_name = Some(captures["name"].into());
                    captures["length"]
                        .parse()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                } else {
                    1
                };

                // out.array_length guaranteed to be Some() if is_array_type() returns true (line 142)
                if length != out.array_length.unwrap() {
                    if ![ParamDefType::U8, ParamDefType::ArrayU8].contains(&out.display_type) {
                        return Err(io::Error::new(
                            InvalidData,
                            format!(
                                "Mismatched array length in {:?} with byte count {byte_count}",
                                out.internal_name
                            ),
                        ));
                    }

                    out.array_length = Some(cmp::min(out.array_length.unwrap(), length))
                }
            }
        }

        if paramdef.format_version >= 104 {
            out.sort_id = Some(br.read_i32()?);
        }

        if paramdef.format_version >= 200 {
            br.assert_i32(&[0])?;
            let unk_b8_offset = br.read_i64()?;
            let unk_c0_offset = br.read_i64()?;
            let unk_c8_offset = br.read_i64()?;

            if unk_b8_offset != 0 {
                out.unk_b8 = Some(br.get_ascii(unk_b8_offset as u64)?);
            }

            if unk_c0_offset != 0 {
                out.unk_c0 = Some(br.get_ascii(unk_c0_offset as u64)?);
            }

            if unk_c8_offset != 0 {
                out.unk_c8 = Some(br.get_ascii(unk_c8_offset as u64)?);
            }
        } else if paramdef.format_version >= 106 {
            br.assert_i32(&[0])?;
            br.assert_i32(&[0])?;
            br.assert_i32(&[0])?;
        }

        if paramdef.format_version >= 203 {
            out.default = out.read_variable_value(br)?;
            out.min = out.read_variable_value(br)?;
            out.max = out.read_variable_value(br)?;
            out.increment = out.read_variable_value(br)?;
        }

        Ok(out)
    }

    pub(crate) fn write<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        paramdef: &ParamDef,
        index: i32,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let padding = if paramdef.format_version >= 104 {
            0x00
        } else {
            0x20
        };
        if paramdef.format_version >= 202 || (106..200).contains(&paramdef.format_version) {
            bw.reserve_varint(format!("display-name-offset-{index}"))?;
        } else if paramdef.unicode {
            bw.write_fix_utf16(&self.display_name, 0x40, padding)?;
        } else {
            bw.write_fix_shift_jis(&self.display_name, 0x40, padding)?;
        }

        let display_type_write = match self.display_type {
            ParamDefType::I8 => "s8",
            ParamDefType::U8 => "u8",
            ParamDefType::I16 => "s16",
            ParamDefType::U16 => "u16",
            ParamDefType::I32 => "s32",
            ParamDefType::U32 => "u32",
            ParamDefType::Bool => "b32",
            ParamDefType::F32 => "f32",
            ParamDefType::Angle => "angle32",
            ParamDefType::F64 => "f64",
            ParamDefType::ArrayU8 => "dummy8",
            ParamDefType::StringShiftJIS => "fixstr",
            ParamDefType::StringUTF16 => "fixstrW",
        };

        bw.write_fix_utf16(display_type_write, 8, padding)?;
        bw.write_fix_utf16(&self.display_format, 8, padding)?;

        if paramdef.format_version >= 203 {
            bw.write_pattern(0x10, 0x00)?;
        } else {
            fn cell_value_to_f32(
                cell: &CellValue,
                value: &str,
                paramdef: &ParamDef,
            ) -> io::Result<f32> {
                if let CellValue::F32(val) = cell {
                    Ok(*val)
                } else {
                    Err(io::Error::new(
                        InvalidData,
                        format!(
                            "Invalid type for {value} value CellValue in {} version paramdef",
                            paramdef.format_version
                        ),
                    ))
                }
            }

            bw.write_f32(cell_value_to_f32(&self.default, "default", paramdef)?)?;
            bw.write_f32(cell_value_to_f32(&self.min, "minimum", paramdef)?)?;
            bw.write_f32(cell_value_to_f32(&self.max, "maximum", paramdef)?)?;
            bw.write_f32(cell_value_to_f32(&self.increment, "increment", paramdef)?)?;
        }

        bw.write_i32(self.edit_flags.bits() as i32)?;

        let size = self.array_length.unwrap_or(1);
        bw.write_i32(paramdef::util::get_value_size(self.display_type) * size)?;

        if paramdef.basic_fields {
            return Ok(());
        }

        bw.reserve_varint(format!("description-offset-{index}"))?;

        if paramdef.format_version >= 202 || (106..200).contains(&paramdef.format_version) {
            bw.reserve_varint(format!("internal-type-offset-{index}"))?;
        } else {
            bw.write_fix_shift_jis(
                self.internal_type.clone().unwrap_or_default(),
                0x20,
                padding,
            )?;
        }

        if paramdef.format_version >= 202 || (106..200).contains(&paramdef.format_version) {
            bw.reserve_varint(format!("internal-name-offset-{index}"))?;
        } else {
            bw.write_fix_shift_jis(self.make_internal_name()?, 0x20, padding)?;
        }

        if paramdef.format_version >= 104 {
            if self.sort_id.is_some() {
                bw.write_i32(self.sort_id.unwrap())?;
            } else {
                bw.write_i32(0)?;
            }
        }

        if paramdef.format_version >= 200 {
            bw.write_i32(0)?;
            bw.reserve_i64(format!("unk-b8-offset-{index}"))?;
            bw.reserve_i64(format!("unk-c0-offset-{index}"))?;
            bw.reserve_i64(format!("unk-c8-offset-{index}"))?;
        } else if paramdef.format_version >= 106 {
            bw.write_i32(0)?;
            bw.write_i32(0)?;
            bw.write_i32(0)?;
        }

        if paramdef.format_version >= 203 {
            Self::write_variable_value(&self.default, bw)?;
            Self::write_variable_value(&self.min, bw)?;
            Self::write_variable_value(&self.max, bw)?;
            Self::write_variable_value(&self.increment, bw)?;
        }

        Ok(())
    }

    pub(crate) fn write_strings<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        paramdef: &ParamDef,
        index: i32,
        shared_string_offsets: &mut HashMap<String, i64>,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        if paramdef.format_version >= 202 || (106..200).contains(&paramdef.format_version) {
            let pos = bw.position()?;
            bw.fill_varint(
                format!("display-name-offset-{index}"),
                util::convert_num(pos)?,
            )?;
            bw.write_utf16(&self.display_name, true)?;
        }

        if paramdef.basic_fields {
            return Ok(());
        }

        let mut description_offset = 0;
        if self.description.is_some() {
            description_offset = bw.position()?;
            if paramdef.unicode {
                bw.write_utf16(self.description.as_ref().unwrap(), true)?;
            } else {
                bw.write_shift_jis(self.description.as_ref().unwrap(), true)?;
            }
        }
        bw.fill_varint(
            format!("description-offset-{index}"),
            util::convert_num(description_offset)?,
        )?;

        if paramdef.format_version >= 202 || (106..200).contains(&paramdef.format_version) {
            let pos = bw.position()?;
            bw.fill_varint(
                format!("internal-type-offset-{index}"),
                util::convert_num(pos)?,
            )?;
            bw.write_ascii(self.internal_type.as_ref().unwrap_or(&"".to_string()), true)?;

            let pos = bw.position()?;
            bw.fill_varint(
                format!("internal-name-offset-{index}"),
                util::convert_num(pos)?,
            )?;
            bw.write_ascii(self.make_internal_name()?, true)?;
        }

        if paramdef.format_version >= 200 {
            let val = Self::write_shared_strings(bw, &self.unk_b8, false, shared_string_offsets)?;
            bw.fill_i64(format!("unk-b8-offset-{index}"), val)?;
            let val = Self::write_shared_strings(bw, &self.unk_c0, false, shared_string_offsets)?;
            bw.fill_i64(format!("unk-c0-offset-{index}"), val)?;
            let val = Self::write_shared_strings(bw, &self.unk_c8, true, shared_string_offsets)?;
            bw.fill_i64(format!("unk-c8-offset-{index}"), val)?;
        }

        Ok(())
    }

    fn write_shared_strings<W>(
        bw: &mut BinaryWriter<W>,
        text: &Option<String>,
        unicode: bool,
        shared_string_offsets: &mut HashMap<String, i64>,
    ) -> io::Result<i64>
    where
        W: Write + Seek,
    {
        if text.is_none() {
            return Ok(0);
        }

        if !shared_string_offsets.contains_key(text.as_ref().unwrap().as_str()) {
            shared_string_offsets.insert(text.clone().unwrap(), util::convert_num(bw.position()?)?);
            if unicode {
                bw.write_utf16(text.clone().unwrap(), true)?;
            }
            else {
                bw.write_shift_jis(text.clone().unwrap(), true)?;
            }
        }

        Ok(shared_string_offsets[text.as_ref().unwrap().as_str()])
    }

    fn make_internal_name(&self) -> io::Result<String> {
        if self.internal_name.is_none() {
            return Err(io::Error::new(
                InvalidData,
                "ParamDef field didn't contain internal name, when it should",
            ));
        }

        if self.bit_size.is_some() {
            Ok(format!(
                "{}:{}",
                self.internal_name.as_ref().unwrap(),
                self.bit_size.unwrap()
            ))
        } else if paramdef::util::is_array_type(self.display_type) && self.array_length.is_some() {
            Ok(format!(
                "{}[{}]",
                self.internal_name.as_ref().unwrap(),
                self.array_length.unwrap()
            ))
        } else {
            Ok(self.internal_name.clone().unwrap())
        }
    }

    fn read_variable_value<R>(&self, br: &mut BinaryReader<R>) -> io::Result<CellValue>
    where
        R: Read + Seek,
    {
        let out = match self.display_type {
            ParamDefType::I8 => {
                let raw = br.read_i32()?;
                br.assert_i32(&[0])?;
                CellValue::I8(util::convert_num(raw)?)
            }
            ParamDefType::U8 => {
                let raw = br.read_i32()?;
                br.assert_i32(&[0])?;
                CellValue::U8(util::convert_num(raw)?)
            }
            ParamDefType::I16 => {
                let raw = br.read_i32()?;
                br.assert_i32(&[0])?;
                CellValue::I16(util::convert_num(raw)?)
            }
            ParamDefType::U16 => {
                let raw = br.read_i32()?;
                br.assert_i32(&[0])?;
                CellValue::U16(util::convert_num(raw)?)
            }
            ParamDefType::I32 => {
                let raw = br.read_i32()?;
                br.assert_i32(&[0])?;
                CellValue::I32(util::convert_num(raw)?)
            }
            ParamDefType::U32 => {
                let raw = br.read_i32()?;
                br.assert_i32(&[0])?;
                CellValue::U32(util::convert_num(raw)?)
            }
            ParamDefType::Bool => {
                let raw = br.read_i32()?;
                br.assert_i32(&[0])?;
                CellValue::Bool(raw != 0)
            }
            ParamDefType::F32 => {
                let raw = br.read_f32()?;
                br.assert_i32(&[0])?;
                CellValue::F32(raw)
            }
            ParamDefType::Angle => {
                let raw = br.read_f32()?;
                br.assert_i32(&[0])?;
                CellValue::Angle(raw)
            }
            ParamDefType::F64 => {
                let raw = br.read_f64()?;
                CellValue::F64(raw)
            }
            ParamDefType::ArrayU8 => {
                br.assert_i64(&[0])?;
                CellValue::ArrayU8(Vec::new())
            }
            ParamDefType::StringShiftJIS => {
                br.assert_i64(&[0])?;
                CellValue::StringShiftJIS(String::new())
            }
            ParamDefType::StringUTF16 => {
                br.assert_i64(&[0])?;
                CellValue::StringUTF16(String::new())
            }
        };

        Ok(out)
    }

    fn write_variable_value<W>(field: &CellValue, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        match field {
            CellValue::I8(x) => {
                bw.write_i32(*x as i32)?;
                bw.write_i32(0)?;
            }
            CellValue::U8(x) => {
                bw.write_i32(*x as i32)?;
                bw.write_i32(0)?;
            }
            CellValue::I16(x) => {
                bw.write_i32(*x as i32)?;
                bw.write_i32(0)?;
            }
            CellValue::U16(x) => {
                bw.write_i32(*x as i32)?;
                bw.write_i32(0)?;
            }
            CellValue::I32(x) => {
                bw.write_i32(*x)?;
                bw.write_i32(0)?;
            }
            CellValue::U32(x) => {
                bw.write_i32(util::convert_num(*x)?)?;
                bw.write_i32(0)?;
            }
            CellValue::Bool(x) => {
                if *x {
                    bw.write_i32(1)?;
                } else {
                    bw.write_i32(0)?;
                }
                bw.write_i32(1)?;
            }
            CellValue::F32(x) => {
                bw.write_f32(*x)?;
                bw.write_i32(0)?;
            }
            CellValue::Angle(x) => {
                bw.write_f32(*x)?;
                bw.write_i32(0)?;
            }
            CellValue::F64(x) => bw.write_f64(*x)?,
            CellValue::ArrayU8(_) => bw.write_i64(0)?,
            CellValue::StringShiftJIS(_) => bw.write_i64(0)?,
            CellValue::StringUTF16(_) => bw.write_i64(0)?,
        }

        Ok(())
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let temp_internal_name = self.internal_name.clone().unwrap_or_default();
        if paramdef::util::is_bit_type(self.display_type) && self.bit_size.is_some() {
            write!(
                f,
                "{:?} {}: {}",
                self.display_type,
                &temp_internal_name,
                self.bit_size.unwrap()
            )
        } else if paramdef::util::is_array_type(self.display_type) && self.array_length.is_some() {
            write!(
                f,
                "{:?} {}[{}]",
                self.display_type,
                &temp_internal_name,
                self.array_length.unwrap()
            )
        } else {
            write!(f, "{:?} {}", self.display_type, &temp_internal_name)
        }
    }
}
