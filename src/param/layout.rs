use std::{
    io::{self, Read, Seek, Write},
    sync::Arc,
};

use crate::{
    io::{BinaryReader, BinaryWriter},
    param::{Cell, CellValue},
    paramdef::{Field, ParamDefType},
};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn bit_width(field: &Field) -> io::Result<Option<(u32, u32)>> {
    let Some(size) = field.bit_size.filter(|&size| size != -1) else {
        return Ok(None);
    };
    let width = match field.display_type {
        ParamDefType::I8 | ParamDefType::U8 | ParamDefType::ArrayU8 => 8,
        ParamDefType::I16 | ParamDefType::U16 => 16,
        ParamDefType::I32 | ParamDefType::U32 => 32,
        _ => return Err(invalid("bit size on a non-integer field")),
    };
    if size <= 0 || size > width {
        return Err(invalid(format!(
            "invalid bit size {size} for {:?}",
            field.display_type
        )));
    }
    Ok(Some((width as u32, size as u32)))
}

fn array_length(field: &Field) -> io::Result<usize> {
    let length = usize::try_from(field.array_length.unwrap_or(1))
        .map_err(|_| invalid("negative field array length"))?;
    if field.display_type == ParamDefType::U8 && length == 0 {
        return Err(invalid("u8 array length must be nonzero"));
    }
    Ok(length)
}

pub(crate) fn default_value(field: &Field) -> io::Result<CellValue> {
    let legacy = match &field.default {
        CellValue::F32(value) => Some(f64::from(*value)),
        _ => None,
    };
    macro_rules! integer {
        ($ty:ty, $variant:ident) => {{
            match (&field.default, legacy) {
                (CellValue::$variant(value), _) => CellValue::$variant(*value),
                (_, Some(value))
                    if value.is_finite()
                        && value.round_ties_even() >= <$ty>::MIN as f64
                        && value.round_ties_even() <= <$ty>::MAX as f64 =>
                {
                    CellValue::$variant(value.round_ties_even() as $ty)
                }
                _ => {
                    return Err(invalid(format!(
                        "invalid default for {:?}",
                        field.internal_name
                    )));
                }
            }
        }};
    }
    let value = match field.display_type {
        ParamDefType::I8 => integer!(i8, I8),
        ParamDefType::U8 => {
            let CellValue::U8(value) = integer!(u8, U8) else {
                unreachable!()
            };
            if bit_width(field)?.is_none() && array_length(field)? > 1 {
                CellValue::ArrayU8(vec![value; array_length(field)?])
            } else {
                CellValue::U8(value)
            }
        }
        ParamDefType::I16 => integer!(i16, I16),
        ParamDefType::U16 => integer!(u16, U16),
        ParamDefType::I32 => integer!(i32, I32),
        ParamDefType::U32 => integer!(u32, U32),
        ParamDefType::Bool => match &field.default {
            CellValue::Bool(value) => CellValue::Bool(*value),
            CellValue::I32(value) => CellValue::I32(*value),
            CellValue::F32(_) => {
                let CellValue::I32(value) = integer!(i32, I32) else {
                    unreachable!()
                };
                match value {
                    0 => CellValue::Bool(false),
                    1 => CellValue::Bool(true),
                    value => CellValue::I32(value),
                }
            }
            _ => return Err(invalid("invalid boolean default")),
        },
        ParamDefType::F32 => match &field.default {
            CellValue::F32(value) => CellValue::F32(*value),
            _ => return Err(invalid("invalid f32 default")),
        },
        ParamDefType::Angle => match &field.default {
            CellValue::Angle(value) | CellValue::F32(value) => CellValue::Angle(*value),
            _ => return Err(invalid("invalid angle default")),
        },
        ParamDefType::F64 => match &field.default {
            CellValue::F64(value) => CellValue::F64(*value),
            CellValue::F32(value) => CellValue::F64(f64::from(*value)),
            _ => return Err(invalid("invalid f64 default")),
        },
        ParamDefType::ArrayU8 => {
            if bit_width(field)?.is_some() {
                CellValue::U8(0)
            } else {
                CellValue::ArrayU8(vec![0; array_length(field)?])
            }
        }
        ParamDefType::StringShiftJIS => CellValue::StringShiftJIS(String::new()),
        ParamDefType::StringUTF16 => CellValue::StringUTF16(String::new()),
    };
    if let Some((_, bits)) = bit_width(field)? {
        checked_bit_value(field, &value, bits)?;
    }
    Ok(value)
}

fn checked_bit_value(field: &Field, value: &CellValue, bits: u32) -> io::Result<u32> {
    let value = match (field.display_type, value) {
        (ParamDefType::I8, CellValue::I8(v)) => *v as i32 as u32,
        (ParamDefType::U8 | ParamDefType::ArrayU8, CellValue::U8(v)) => u32::from(*v),
        (ParamDefType::I16, CellValue::I16(v)) => *v as i32 as u32,
        (ParamDefType::U16, CellValue::U16(v)) => u32::from(*v),
        (ParamDefType::I32, CellValue::I32(v)) => *v as u32,
        (ParamDefType::U32, CellValue::U32(v)) => *v,
        _ => return Err(invalid("cell value does not match bitfield type")),
    };
    let mask = if bits == 32 {
        u32::MAX
    } else {
        (1u32 << bits) - 1
    };
    let fits = if matches!(
        field.display_type,
        ParamDefType::I8 | ParamDefType::I16 | ParamDefType::I32
    ) {
        let signed = value as i32 as i64;
        let limit = 1i64 << (bits - 1);
        signed >= -limit && signed < limit
    } else {
        value <= mask
    };
    if !fits {
        return Err(invalid(format!(
            "value does not fit bitfield {:?}",
            field.internal_name
        )));
    }
    Ok(value & mask)
}

pub(crate) fn row_size(fields: &[Arc<Field>]) -> io::Result<usize> {
    let mut total = 0usize;
    let mut bit_group: Option<(u32, u32)> = None;
    for field in fields {
        let size = if let Some((width, bits)) = bit_width(field)? {
            if let Some((group_width, used)) = bit_group {
                if width == group_width && used + bits <= width {
                    bit_group = Some((width, used + bits));
                    0
                } else {
                    bit_group = Some((width, bits));
                    (width / 8) as usize
                }
            } else {
                bit_group = Some((width, bits));
                (width / 8) as usize
            }
        } else {
            bit_group = None;
            match field.display_type {
                ParamDefType::U8 | ParamDefType::ArrayU8 | ParamDefType::StringShiftJIS => {
                    array_length(field)?
                }
                ParamDefType::StringUTF16 => array_length(field)?
                    .checked_mul(2)
                    .ok_or_else(|| invalid("field size overflow"))?,
                ParamDefType::I8 => 1,
                ParamDefType::I16 | ParamDefType::U16 => 2,
                ParamDefType::I32
                | ParamDefType::U32
                | ParamDefType::Bool
                | ParamDefType::F32
                | ParamDefType::Angle => 4,
                ParamDefType::F64 => 8,
            }
        };
        total = total
            .checked_add(size)
            .ok_or_else(|| invalid("row size overflow"))?;
    }
    Ok(total)
}

fn read_bits<R: Read + Seek>(br: &mut BinaryReader<R>, width: u32) -> io::Result<u32> {
    match width {
        8 => Ok(u32::from(br.read_u8()?)),
        16 => Ok(u32::from(br.read_u16()?)),
        32 => br.read_u32(),
        _ => Err(invalid("invalid bitfield width")),
    }
}

fn check_unused_bits(group: Option<(u32, u32, u32)>) -> io::Result<()> {
    if let Some((_, used, value)) = group
        && used < 32
        && value >> used != 0
    {
        return Err(invalid("nonzero unused bits in a parameter bitfield"));
    }
    Ok(())
}

pub(crate) fn read_cells<R: Read + Seek>(
    br: &mut BinaryReader<R>,
    fields: &[Arc<Field>],
) -> io::Result<Vec<Cell>> {
    let mut cells = Vec::with_capacity(fields.len());
    let mut group: Option<(u32, u32, u32)> = None;
    for field in fields {
        let value = if let Some((width, bits)) = bit_width(field)? {
            if !matches!(group, Some((w, used, _)) if w == width && used + bits <= width) {
                check_unused_bits(group)?;
                group = Some((width, 0, read_bits(br, width)?));
            }
            let (_, used, raw) = group.expect("bit group was initialized");
            let mask = if bits == 32 {
                u32::MAX
            } else {
                (1u32 << bits) - 1
            };
            let value = (raw >> used) & mask;
            group = Some((width, used + bits, raw));
            let signed = ((value << (32 - bits)) as i32) >> (32 - bits);
            match field.display_type {
                ParamDefType::I8 => CellValue::I8(signed as i8),
                ParamDefType::U8 | ParamDefType::ArrayU8 => CellValue::U8(value as u8),
                ParamDefType::I16 => CellValue::I16(signed as i16),
                ParamDefType::U16 => CellValue::U16(value as u16),
                ParamDefType::I32 => CellValue::I32(signed),
                ParamDefType::U32 => CellValue::U32(value),
                _ => return Err(invalid("invalid bitfield type")),
            }
        } else {
            check_unused_bits(group.take())?;
            match field.display_type {
                ParamDefType::I8 => CellValue::I8(br.read_i8()?),
                ParamDefType::U8 if array_length(field)? > 1 => {
                    CellValue::ArrayU8(br.read_vec_u8(array_length(field)? as u64)?)
                }
                ParamDefType::U8 => CellValue::U8(br.read_u8()?),
                ParamDefType::I16 => CellValue::I16(br.read_i16()?),
                ParamDefType::U16 => CellValue::U16(br.read_u16()?),
                ParamDefType::I32 => CellValue::I32(br.read_i32()?),
                ParamDefType::U32 => CellValue::U32(br.read_u32()?),
                ParamDefType::Bool => match br.read_i32()? {
                    0 => CellValue::Bool(false),
                    1 => CellValue::Bool(true),
                    raw => CellValue::I32(raw),
                },
                ParamDefType::F32 => CellValue::F32(br.read_f32()?),
                ParamDefType::Angle => CellValue::Angle(br.read_f32()?),
                ParamDefType::F64 => CellValue::F64(br.read_f64()?),
                ParamDefType::ArrayU8 => {
                    CellValue::ArrayU8(br.read_vec_u8(array_length(field)? as u64)?)
                }
                ParamDefType::StringShiftJIS => {
                    CellValue::StringShiftJIS(br.read_fixed_shift_jis(array_length(field)?)?)
                }
                ParamDefType::StringUTF16 => CellValue::StringUTF16(
                    br.read_fixed_utf16(
                        array_length(field)?
                            .checked_mul(2)
                            .ok_or_else(|| invalid("field size overflow"))?,
                    )?,
                ),
            }
        };
        cells.push(Cell {
            paramdef_field: Arc::clone(field),
            value,
        });
    }
    check_unused_bits(group)?;
    Ok(cells)
}

fn write_bits<W: Write + Seek>(
    bw: &mut BinaryWriter<W>,
    group: Option<(u32, u32, u32)>,
) -> io::Result<()> {
    if let Some((width, _, value)) = group {
        match width {
            8 => bw.write_u8(value as u8)?,
            16 => bw.write_u16(value as u16)?,
            32 => bw.write_u32(value)?,
            _ => return Err(invalid("invalid bitfield width")),
        }
    }
    Ok(())
}

pub(crate) fn write_cells<W: Write + Seek>(
    bw: &mut BinaryWriter<W>,
    cells: &[Cell],
) -> io::Result<()> {
    let mut group: Option<(u32, u32, u32)> = None;
    for cell in cells {
        let field = &cell.paramdef_field;
        if let Some((width, bits)) = bit_width(field)? {
            if !matches!(group, Some((w, used, _)) if w == width && used + bits <= width) {
                write_bits(bw, group.take())?;
                group = Some((width, 0, 0));
            }
            let (_, used, raw) = group.expect("bit group was initialized");
            let value = checked_bit_value(field, &cell.value, bits)?;
            group = Some((width, used + bits, raw | (value << used)));
            continue;
        }
        write_bits(bw, group.take())?;
        match (field.display_type, &cell.value) {
            (ParamDefType::I8, CellValue::I8(v)) => bw.write_i8(*v)?,
            (ParamDefType::U8, CellValue::U8(v)) if array_length(field)? <= 1 => bw.write_u8(*v)?,
            (ParamDefType::U8 | ParamDefType::ArrayU8, CellValue::ArrayU8(v))
                if v.len() == array_length(field)? =>
            {
                bw.write_bytes(v)?
            }
            (ParamDefType::I16, CellValue::I16(v)) => bw.write_i16(*v)?,
            (ParamDefType::U16, CellValue::U16(v)) => bw.write_u16(*v)?,
            (ParamDefType::I32, CellValue::I32(v)) => bw.write_i32(*v)?,
            (ParamDefType::U32, CellValue::U32(v)) => bw.write_u32(*v)?,
            (ParamDefType::Bool, CellValue::Bool(v)) => bw.write_i32(i32::from(*v))?,
            (ParamDefType::Bool, CellValue::I32(v)) => bw.write_i32(*v)?,
            (ParamDefType::F32, CellValue::F32(v)) => bw.write_f32(*v)?,
            (ParamDefType::Angle, CellValue::Angle(v)) => bw.write_f32(*v)?,
            (ParamDefType::F64, CellValue::F64(v)) => bw.write_f64(*v)?,
            (ParamDefType::StringShiftJIS, CellValue::StringShiftJIS(v)) => {
                bw.write_fix_shift_jis(v, array_length(field)?, 0)?
            }
            (ParamDefType::StringUTF16, CellValue::StringUTF16(v)) => bw.write_fix_utf16(
                v,
                array_length(field)?
                    .checked_mul(2)
                    .ok_or_else(|| invalid("field size overflow"))?,
                0,
            )?,
            _ => {
                return Err(invalid(format!(
                    "cell value or array length does not match field {:?}",
                    field.internal_name
                )));
            }
        }
    }
    write_bits(bw, group)
}
