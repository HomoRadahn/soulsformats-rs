use std::io::{self, Read, Seek, Write};

use crate::{
    io::BinaryReader,
    param::{
        format::ParamCellValue,
        paramdef::{
            ParamDef,
            format::{EditFlags, ParamDefType},
        },
    },
};

/// Information about a field present in each row in a param
pub struct Field {
    /// Name to display in the editor
    pub display_name: String,
    /// Type of value to display in the editor
    pub display_type: ParamDefType,
    /// Printf-style format string to apply to the value in the editor
    pub display_format: String,
    /// Default value for new rows
    pub default: ParamCellValue,
    /// Minimum valid value
    pub min: ParamCellValue,
    /// Maximum valid value
    pub max: ParamCellValue,
    /// Amount of increase or decrease per step
    pub increment: ParamCellValue,
    /// Flags determining behavior of the field
    pub edit_flags: EditFlags,
    /// Number of elements for array types; only supported for ArrayU8, StringShiftJIS, and StringUTF16
    pub array_length: usize,
    /// Optional description of the field; may be null
    pub description: Option<String>,
    /// Type of the value in the engine; may be an enum type
    pub internal_type: String,
    /// Name of the value in the engine; not present before version 102
    pub internal_name: String,
    /// Number of bits used by a bitfield; only supported for unsigned types, -1 when not used
    pub bit_size: Option<usize>,
    /// Fields are ordered by this value in the editor; not present before version 104
    pub sort_id: Option<i32>,
    pub unk_b8: Option<String>,
    pub unk_c0: Option<String>,
    pub unk_c8: Option<String>,
    pub first_regulation_version: usize,
    pub removed_regulation_version: usize,
}

impl Default for Field {
    fn default() -> Self {
        Self {
            display_name: "placeholder".to_string(),
            display_type: ParamDefType::f32,
            display_format: Default::default(),
            default: todo!(),
            min: todo!(),
            max: todo!(),
            increment: todo!(),
            edit_flags: todo!(),
            array_length: 1,
            description: Default::default(),
            internal_type: "f32".to_string(),
            internal_name: "placeholder".to_string(),
            bit_size: Default::default(),
            sort_id: Default::default(),
            unk_b8: Default::default(),
            unk_c0: Default::default(),
            unk_c8: Default::default(),
            first_regulation_version: Default::default(),
            removed_regulation_version: Default::default(),
        }
    }
}

impl Field {
    pub fn from_binary_reader<R>(br: &mut BinaryReader<R>, paramdef: &ParamDef) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        todo!()
    }
}
