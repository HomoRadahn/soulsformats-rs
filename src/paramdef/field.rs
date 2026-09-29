use std::io::{self, ErrorKind::InvalidData, Read, Seek, Write};

use crate::{
    io::BinaryReader, param::{CellValue, self},
    ParamDef, paramdef::{EditFlags, ParamDefType},
    util
};

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
    pub fn new(display_type: ParamDefType, internal_name: impl Into<String>) -> Self {
        let internal_name = internal_name.into();
        Self {
            display_name: internal_name.clone(),
            display_type: display_type,
            display_format: param::util::get_default_format(display_type),
            default: param::util::get_default_value(display_type),
            min: param::util::get_default_minimum(display_type),
            max: param::util::get_default_maximum(display_type),
            increment: param::util::get_default_increment(display_type),
            edit_flags: param::util::get_default_edit_flags(display_type),
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

    pub fn read<R>(br: &mut BinaryReader<R>, paramdef: &ParamDef) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let mut out = Self::default();

        out.display_name = if paramdef.format_version >= 202 || (106..200).contains(&paramdef.format_version) {
            let pos = br.read_varint()?;
            br.get_utf16(pos as u64)?
        }
        else if paramdef.unicode {
            br.read_fixed_utf16(0x40)?
        }
        else {
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
        }
        else {
            out.default = CellValue::F32(br.read_f32()?);
            out.min = CellValue::F32(br.read_f32()?);
            out.max = CellValue::F32(br.read_f32()?);
            out.increment = CellValue::F32(br.read_f32()?);
        }

        out.edit_flags = EditFlags::try_from(util::convert_num::<i32, u8>(br.read_i32()?)?)?;

        let byte_count = br.read_i32()?;
        
        if !param::util::is_array_type(out.display_type) && byte_count != param::util::get_value_size(out.display_type)
        || param::util::is_array_type(out.display_type) && byte_count % param::util::get_value_size(out.display_type) != 0
        {
            return Err(io::Error::new(InvalidData, format!("Unexpected byte count {byte_count} for type: {:?}", out.display_type)));
        }

        out.array_length = if param::util::is_array_type(out.display_type) {
            Some(byte_count / param::util::get_value_size(out.display_type))
        }
        else {
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
            Some(
                if paramdef.unicode {
                    br.get_utf16(description_offset as u64)?
                }
                else {
                    br.get_shift_jis(description_offset as u64)?
                }
            )
        }
        else {
            None
        };

        out.internal_type = if paramdef.format_version >= 202 
        || paramdef.format_version >= 106 && paramdef.format_version < 200
        {
            let pos = br.read_varint()?;
            Some(br.get_ascii(pos as u64)?.trim().into())
        }
        else {
            Some(br.read_fixed_shift_jis(0x20)?.trim().into())
        };

        

        todo!();

        Ok(out)
    }
}
