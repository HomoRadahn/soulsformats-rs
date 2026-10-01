use crate::param::CellValue;
use crate::paramdef::{EditFlags, ParamDefType};

pub(crate) fn get_default_format(param_def_type: ParamDefType) -> String {
    match param_def_type {
        ParamDefType::I8 => "%d",
        ParamDefType::U8 => "%d",
        ParamDefType::I16 => "%d",
        ParamDefType::U16 => "%d",
        ParamDefType::I32 => "%d",
        ParamDefType::U32 => "%d",
        ParamDefType::Bool => "%d",
        ParamDefType::F32 => "%f",
        ParamDefType::Angle => "%f",
        ParamDefType::F64 => "%f",
        ParamDefType::ArrayU8 => "",
        ParamDefType::StringShiftJIS => "%d",
        ParamDefType::StringUTF16 => "%d",
    }
    .to_string()
}

pub(crate) fn get_default_value(param_def_type: ParamDefType) -> CellValue {
    match param_def_type {
        ParamDefType::I8 => CellValue::I8(0),
        ParamDefType::U8 => CellValue::U8(0),
        ParamDefType::I16 => CellValue::I16(0),
        ParamDefType::U16 => CellValue::U16(0),
        ParamDefType::I32 => CellValue::I32(0),
        ParamDefType::U32 => CellValue::U32(0),
        ParamDefType::Bool => CellValue::Bool(false),
        ParamDefType::F32 => CellValue::F32(0.0),
        ParamDefType::Angle => CellValue::Angle(0.0),
        ParamDefType::F64 => CellValue::F64(0.0),
        ParamDefType::ArrayU8 => CellValue::ArrayU8(Vec::new()),
        ParamDefType::StringShiftJIS => CellValue::StringShiftJIS(String::new()),
        ParamDefType::StringUTF16 => CellValue::StringUTF16(String::new()),
    }
}

pub(crate) fn get_default_minimum(param_def_type: ParamDefType) -> CellValue {
    match param_def_type {
        ParamDefType::I8 => CellValue::I8(i8::MIN),
        ParamDefType::U8 => CellValue::U8(u8::MIN),
        ParamDefType::I16 => CellValue::I16(i16::MIN),
        ParamDefType::U16 => CellValue::U16(u16::MIN),
        ParamDefType::I32 => CellValue::I32(-2147483520),
        ParamDefType::U32 => CellValue::U32(u32::MIN),
        ParamDefType::Bool => CellValue::Bool(false),
        ParamDefType::F32 => CellValue::F32(f32::MIN),
        ParamDefType::Angle => CellValue::Angle(f32::MIN),
        ParamDefType::F64 => CellValue::F64(f64::MIN),
        ParamDefType::ArrayU8 => CellValue::ArrayU8(Vec::new()),
        ParamDefType::StringShiftJIS => CellValue::StringShiftJIS(String::new()),
        ParamDefType::StringUTF16 => CellValue::StringUTF16(String::new()),
    }
}

pub(crate) fn get_default_maximum(param_def_type: ParamDefType) -> CellValue {
    match param_def_type {
        ParamDefType::I8 => CellValue::I8(i8::MAX),
        ParamDefType::U8 => CellValue::U8(u8::MAX),
        ParamDefType::I16 => CellValue::I16(i16::MAX),
        ParamDefType::U16 => CellValue::U16(u16::MAX),
        ParamDefType::I32 => CellValue::I32(2147483520),
        ParamDefType::U32 => CellValue::U32(4294967040),
        ParamDefType::Bool => CellValue::Bool(true),
        ParamDefType::F32 => CellValue::F32(f32::MAX),
        ParamDefType::Angle => CellValue::Angle(f32::MAX),
        ParamDefType::F64 => CellValue::F64(f64::MAX),
        ParamDefType::ArrayU8 => CellValue::ArrayU8(Vec::new()),
        ParamDefType::StringShiftJIS => CellValue::StringShiftJIS(String::new()),
        ParamDefType::StringUTF16 => CellValue::StringUTF16(String::new()),
    }
}

pub(crate) fn get_default_increment(param_def_type: ParamDefType) -> CellValue {
    match param_def_type {
        ParamDefType::I8 => CellValue::I8(1),
        ParamDefType::U8 => CellValue::U8(1),
        ParamDefType::I16 => CellValue::I16(1),
        ParamDefType::U16 => CellValue::U16(1),
        ParamDefType::I32 => CellValue::I32(1),
        ParamDefType::U32 => CellValue::U32(1),
        ParamDefType::Bool => CellValue::Bool(true),
        ParamDefType::F32 => CellValue::F32(0.01),
        ParamDefType::Angle => CellValue::Angle(0.01),
        ParamDefType::F64 => CellValue::F64(0.01),
        ParamDefType::ArrayU8 => CellValue::ArrayU8(Vec::new()),
        ParamDefType::StringShiftJIS => CellValue::StringShiftJIS(String::new()),
        ParamDefType::StringUTF16 => CellValue::StringUTF16(String::new()),
    }
}

pub(crate) fn get_default_edit_flags(param_def_type: ParamDefType) -> EditFlags {
    match param_def_type {
        ParamDefType::ArrayU8 => EditFlags::None,
        _ => EditFlags::Wrap,
    }
}

pub(crate) fn is_array_type(param_def_type: ParamDefType) -> bool {
    match param_def_type {
        ParamDefType::U8
        | ParamDefType::ArrayU8
        | ParamDefType::StringShiftJIS
        | ParamDefType::StringUTF16 => true,
        _ => false,
    }
}

pub(crate) fn is_bit_type(param_def_type: ParamDefType) -> bool {
    match param_def_type {
        ParamDefType::I8
        | ParamDefType::U8
        | ParamDefType::I16
        | ParamDefType::U16
        | ParamDefType::I32
        | ParamDefType::U32
        | ParamDefType::ArrayU8 => true,
        _ => false,
    }
}

pub(crate) fn get_value_size(param_def_type: ParamDefType) -> i32 {
    match param_def_type {
        ParamDefType::I8 => 1,
        ParamDefType::U8 => 1,
        ParamDefType::I16 => 2,
        ParamDefType::U16 => 2,
        ParamDefType::I32 => 4,
        ParamDefType::U32 => 4,
        ParamDefType::Bool => 4,
        ParamDefType::F32 => 4,
        ParamDefType::Angle => 4,
        ParamDefType::F64 => 8,
        ParamDefType::ArrayU8 => 1,
        ParamDefType::StringShiftJIS => 1,
        ParamDefType::StringUTF16 => 2,
    }
}
