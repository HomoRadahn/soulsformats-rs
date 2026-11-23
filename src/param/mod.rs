use std::{
    collections::HashMap,
    io::{self, Cursor, Read, Seek, Write},
    sync::Arc,
};

use crate::{
    ByteIO, FileIO, ParamDef,
    io::{BinaryReader, BinaryWriter, DcxIO, Endian, StreamIO},
    paramdef::Field,
    util,
};

pub mod cell;
pub mod format;
mod layout;
pub mod row;

pub use cell::Cell;
pub use format::{CellValue, CellValueExt, FormatFlags1, FormatFlags2};
pub use row::Row;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// A PARAM file whose row data can be interpreted using a matching [`ParamDef`].
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// Endianness of the data.
    pub endian: Endian,
    /// First set of format flags.
    pub format_1: FormatFlags1,
    /// Second set of format flags.
    pub format_2: FormatFlags2,
    /// PARAMDEF format version byte.
    pub paramdef_format_version: u8,
    pub unk06: u16,
    /// Revision of the row data structure.
    pub paramdef_data_version: u16,
    /// Identifies corresponding PARAM and PARAMDEF files.
    pub param_type: String,
    /// Detected distance between rows, or `None` when no rows are present.
    pub detected_size: Option<u64>,
    /// Row headers; cells are populated after applying a PARAMDEF.
    pub rows: Vec<Row>,
    /// The applied definition. Check [`Self::has_paramdef`] before using it.
    pub paramdef: ParamDef,
    /// Whether row headers omit name offsets.
    pub unnamed_rows: bool,
    /// Whether rows omit headers entirely.
    pub headerless_rows: bool,
    row_reader: BinaryReader<Cursor<Vec<u8>>>,
    has_paramdef: bool,
    regulation_version: Option<u64>,
}

impl Default for Param {
    fn default() -> Self {
        Self {
            endian: Endian::Little,
            format_1: FormatFlags1::None,
            format_2: FormatFlags2::None,
            paramdef_format_version: 0,
            unk06: 0,
            paramdef_data_version: 0,
            param_type: String::new(),
            detected_size: None,
            rows: Vec::new(),
            paramdef: ParamDef::default(),
            unnamed_rows: false,
            headerless_rows: false,
            row_reader: BinaryReader::from_bytes(Vec::new(), Endian::Little, false),
            has_paramdef: false,
            regulation_version: None,
        }
    }
}

impl Param {
    /// Creates an empty PARAM with an applied definition.
    pub fn new(paramdef: &ParamDef) -> io::Result<Self> {
        if paramdef.version_aware {
            return Err(invalid(
                "use a regulation version with a version-aware PARAMDEF",
            ));
        }
        layout::row_size(&Self::fields_for_version(paramdef, u64::MAX)?)?;
        Ok(Self {
            param_type: paramdef.param_type.clone(),
            paramdef_data_version: paramdef.data_version as u16,
            paramdef: paramdef.clone(),
            has_paramdef: true,
            ..Self::default()
        })
    }

    /// Creates an empty PARAM for a specific regulation version.
    pub fn new_for_version(paramdef: &ParamDef, version: u64) -> io::Result<Self> {
        if !paramdef.version_aware {
            return Err(invalid("PARAMDEF is not regulation-version aware"));
        }
        layout::row_size(&Self::fields_for_version(paramdef, version)?)?;
        Ok(Self {
            param_type: paramdef.param_type.clone(),
            paramdef_data_version: paramdef.data_version as u16,
            paramdef: paramdef.clone(),
            has_paramdef: true,
            regulation_version: Some(version),
            ..Self::default()
        })
    }

    /// Returns whether a PARAMDEF has been applied.
    pub fn has_paramdef(&self) -> bool {
        self.has_paramdef
    }

    fn fields_for_version(def: &ParamDef, version: u64) -> io::Result<Vec<Arc<Field>>> {
        let mut fields = Vec::with_capacity(def.fields.len());
        for field in &def.fields {
            if !def.version_aware || field.is_valid_version(version)? {
                fields.push(Arc::new(field.clone()));
            }
        }
        Ok(fields)
    }

    fn apply(&mut self, def: &ParamDef, version: Option<u64>) -> io::Result<()> {
        if version.is_some() && !def.version_aware {
            return Err(invalid("PARAMDEF is not regulation-version aware"));
        }
        let fields = Self::fields_for_version(def, version.unwrap_or(u64::MAX))?;
        let size = layout::row_size(&fields)? as u64;
        let length = self.row_reader.length()?;
        let mut decoded = Vec::with_capacity(self.rows.len());
        for (index, row) in self.rows.iter().enumerate() {
            if row.data_offset == 0 {
                decoded.push(None);
                continue;
            }
            let end = row
                .data_offset
                .checked_add(size)
                .ok_or_else(|| invalid("row offset overflow"))?;
            if end > length {
                return Err(invalid(format!("row {} extends beyond the file", row.id)));
            }
            if let Some(next) = self.rows.get(index + 1)
                && next.data_offset > row.data_offset
                && end > next.data_offset
            {
                return Err(invalid(format!("PARAMDEF exceeds row {} size", row.id)));
            }
            self.row_reader.seek(row.data_offset)?;
            decoded.push(Some(layout::read_cells(&mut self.row_reader, &fields)?));
        }
        let shared_def = Arc::new(def.clone());
        for (row, cells) in self.rows.iter_mut().zip(decoded) {
            if let Some(cells) = cells {
                row.cells = cells;
                row.paramdef = Arc::clone(&shared_def);
            }
        }
        self.paramdef = def.clone();
        self.has_paramdef = true;
        self.regulation_version = version;
        Ok(())
    }

    /// Interprets row data using a PARAMDEF. Existing cells are replaced only on success.
    pub fn apply_paramdef(&mut self, def: &ParamDef) -> io::Result<()> {
        self.apply(def, None)
    }

    /// Applies a version-aware PARAMDEF for a specific regulation version.
    pub fn apply_regulation_versioned_paramdef(
        &mut self,
        def: &ParamDef,
        version: u64,
    ) -> io::Result<()> {
        self.apply(def, Some(version))
    }

    /// Applies a PARAMDEF only if its type, data version, and row size match.
    pub fn apply_paramdef_carefully(&mut self, def: &ParamDef) -> io::Result<bool> {
        self.apply_carefully(def, None)
    }

    fn apply_carefully(&mut self, def: &ParamDef, version: Option<u64>) -> io::Result<bool> {
        if version.is_some() && !def.version_aware {
            return Err(invalid("PARAMDEF is not regulation-version aware"));
        }
        let fields = Self::fields_for_version(def, version.unwrap_or(u64::MAX))?;
        let row_size =
            u64::try_from(layout::row_size(&fields)?).map_err(|_| invalid("row size overflow"))?;
        if self.param_type != def.param_type
            || self.paramdef_data_version != def.data_version as u16
            || self.detected_size.is_some_and(|size| size != row_size)
        {
            return Ok(false);
        }
        self.apply(def, version)?;
        Ok(true)
    }

    /// Applies a PARAMDEF if its type and data version match, ignoring row size.
    pub fn apply_paramdef_somewhat_carefully(&mut self, def: &ParamDef) -> io::Result<bool> {
        self.apply_somewhat_carefully(def, None)
    }

    fn apply_somewhat_carefully(
        &mut self,
        def: &ParamDef,
        version: Option<u64>,
    ) -> io::Result<bool> {
        if version.is_some() && !def.version_aware {
            return Err(invalid("PARAMDEF is not regulation-version aware"));
        }
        if !(self.param_type.is_empty() || self.param_type == def.param_type)
            || (!self.headerless_rows && self.paramdef_data_version != def.data_version as u16)
        {
            return Ok(false);
        }
        self.apply(def, version)?;
        Ok(true)
    }

    /// Applies a version-aware PARAMDEF if its type, data version, and row size match.
    pub fn apply_regulation_versioned_paramdef_carefully(
        &mut self,
        def: &ParamDef,
        version: u64,
    ) -> io::Result<bool> {
        self.apply_carefully(def, Some(version))
    }

    /// Applies a version-aware PARAMDEF if its type and data version match, ignoring row size.
    pub fn apply_regulation_versioned_paramdef_somewhat_carefully(
        &mut self,
        def: &ParamDef,
        version: u64,
    ) -> io::Result<bool> {
        self.apply_somewhat_carefully(def, Some(version))
    }

    /// Returns the first row with the specified ID.
    pub fn find_row(&self, id: i32) -> Option<&Row> {
        self.rows.iter().find(|row| row.id == id)
    }

    /// Returns the first row with the specified ID for editing.
    pub fn find_row_mut(&mut self, id: i32) -> Option<&mut Row> {
        self.rows.iter_mut().find(|row| row.id == id)
    }
}

impl ParamDef {
    /// Calculates the serialized row size using the latest applicable fields.
    pub fn row_size(&self) -> io::Result<usize> {
        self.row_size_for_version(u64::MAX)
    }

    /// Calculates the serialized row size for a regulation version.
    pub fn row_size_for_version(&self, version: u64) -> io::Result<usize> {
        layout::row_size(&Param::fields_for_version(self, version)?)
    }
}

impl StreamIO<Param> for Param {
    fn read<R: Read + Seek>(br: &mut BinaryReader<R>) -> io::Result<Self> {
        let length = br.length()?;
        if length < 0x30 {
            return Err(invalid("PARAM header is truncated"));
        }
        let endian = match br.get_u8(0x2c)? {
            0 => Endian::Little,
            0xff => Endian::Big,
            _ => return Err(invalid("invalid PARAM endianness")),
        };
        br.endian = endian;
        let format_1 = FormatFlags1::try_from(br.get_u8(0x2d)?)?;
        let format_2 = FormatFlags2::try_from(br.get_u8(0x2e)?)?;
        let paramdef_format_version = br.get_u8(0x2f)?;
        let mut row_reader = BinaryReader::from_bytes(br.get_vec_u8(0, length)?, endian, false);
        row_reader.seek(0)?;
        br.seek(0)?;
        let strings_offset = u64::from(br.read_u32()?);
        let extended32 = format_1.contains(FormatFlags1::Flag01 | FormatFlags1::DataOffsets_32bit);
        let extended64 = !extended32 && format_1.contains(FormatFlags1::DataOffsets_64bit);
        let mut data_start = if extended32 || extended64 {
            br.assert_u16(&[0])?;
            0
        } else {
            u64::from(br.read_u16()?)
        };
        let unk06 = br.read_u16()?;
        let paramdef_data_version = br.read_u16()?;
        let row_count = usize::from(br.read_u16()?);
        let mut actual_strings_offset = 0;
        let param_type = if format_1.contains(FormatFlags1::OffsetParamType) {
            br.assert_i32(&[0])?;
            let offset = br.read_i64()?;
            br.assert_pattern(0x14, 0)?;
            if offset < 0 || offset as u64 >= length {
                return Err(invalid("invalid PARAM type offset"));
            }
            actual_strings_offset = offset as u64;
            br.get_ascii(offset as u64)?
        } else {
            br.read_fixed_shift_jis(0x20)?
        };
        br.skip(4)?;
        if extended32 {
            data_start = u64::from(br.read_u32()?);
            br.assert_i32(&[0])?;
            br.assert_i32(&[0])?;
            br.assert_i32(&[0])?;
        } else if extended64 {
            data_start = util::convert_num(br.read_i64()?)?;
            br.assert_i64(&[0])?;
        }
        let rows_start = br.position()?;
        if data_start > length {
            return Err(invalid("PARAM data start is beyond the file"));
        }
        let long_offsets = format_1.contains(FormatFlags1::DataOffsets_64bit);
        let named_header_size = if long_offsets { 24u64 } else { 12 };
        let unnamed_header_size = if long_offsets { 16u64 } else { 8 };
        let mut unnamed_rows = false;
        let mut headerless_rows = false;
        if row_count > 0 {
            let padding = if format_1 == FormatFlags1::Flag01 {
                0x20
            } else {
                0
            };
            if data_start <= rows_start + padding
                || data_start - rows_start - padding < unnamed_header_size
            {
                headerless_rows = true;
                unnamed_rows = true;
            } else {
                let first_offset = if long_offsets {
                    util::convert_num::<i64, u64>(br.get_i64(rows_start + 8)?)?
                } else {
                    u64::from(br.get_u32(rows_start + 4)?)
                };
                let available = first_offset
                    .checked_sub(rows_start + padding)
                    .ok_or_else(|| invalid("invalid first row offset"))?;
                unnamed_rows = available < named_header_size * row_count as u64;
                let header_size = if unnamed_rows {
                    unnamed_header_size
                } else {
                    named_header_size
                };
                if rows_start + header_size > data_start {
                    headerless_rows = true;
                    unnamed_rows = true;
                }
            }
        }
        let placeholder = Arc::new(ParamDef::default());
        let mut rows = Vec::with_capacity(row_count);
        let detected_size;
        if headerless_rows {
            let row_data_start = data_start.max(rows_start);
            let data_end = if strings_offset >= row_data_start
                && strings_offset <= length
                && (strings_offset - row_data_start) % row_count as u64 == 0
            {
                strings_offset
            } else {
                length
            };
            let size = (data_end - row_data_start) / row_count as u64;
            detected_size = Some(size);
            for index in 0..row_count {
                rows.push(Row {
                    paramdef: Arc::clone(&placeholder),
                    id: index as i32,
                    name: None,
                    cells: Vec::new(),
                    data_offset: row_data_start + size * index as u64,
                });
            }
        } else {
            for _ in 0..row_count {
                let id = br.read_i32()?;
                let offset = if long_offsets {
                    br.read_i32()?;
                    util::convert_num(br.read_i64()?)?
                } else {
                    u64::from(br.read_u32()?)
                };
                if offset == 0 || offset > length {
                    return Err(invalid(format!("invalid data offset for row {id}")));
                }
                let name_offset = if unnamed_rows {
                    0
                } else if long_offsets {
                    util::convert_num(br.read_i64()?)?
                } else {
                    u64::from(br.read_u32()?)
                };
                let name = if name_offset == 0 || name_offset == length {
                    None
                } else {
                    if name_offset > length {
                        return Err(invalid(format!("row {id} name is beyond the file")));
                    }
                    if actual_strings_offset == 0 || name_offset < actual_strings_offset {
                        actual_strings_offset = name_offset;
                    }
                    Some(if format_2.contains(FormatFlags2::UnicodeRowNames) {
                        br.get_utf16(name_offset)?
                    } else {
                        br.get_shift_jis(name_offset)?
                    })
                };
                rows.push(Row {
                    paramdef: Arc::clone(&placeholder),
                    id,
                    name,
                    cells: Vec::new(),
                    data_offset: offset,
                });
            }
            detected_size = match rows.as_slice() {
                [] => None,
                [row] => {
                    let end = [actual_strings_offset, strings_offset]
                        .into_iter()
                        .filter(|&offset| {
                            offset >= row.data_offset && offset != 0 && offset <= length
                        })
                        .min()
                        .unwrap_or(length);
                    end.checked_sub(row.data_offset)
                }
                [first, second, ..] => second.data_offset.checked_sub(first.data_offset),
            };
        }
        Ok(Self {
            endian,
            format_1,
            format_2,
            paramdef_format_version,
            unk06,
            paramdef_data_version,
            param_type,
            detected_size,
            rows,
            paramdef: ParamDef::default(),
            unnamed_rows,
            headerless_rows,
            row_reader,
            has_paramdef: false,
            regulation_version: None,
        })
    }

    fn write<W: Write + Seek>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()> {
        if !self.has_paramdef {
            return Err(invalid("a PARAMDEF must be applied before writing"));
        }
        if self.headerless_rows && !self.unnamed_rows {
            return Err(invalid("headerless rows cannot have names"));
        }
        let fields =
            Self::fields_for_version(&self.paramdef, self.regulation_version.unwrap_or(u64::MAX))?;
        layout::row_size(&fields)?;
        for row in &self.rows {
            if row.cells.len() != fields.len()
                || row.cells.iter().zip(&fields).any(|(cell, field)| {
                    cell.paramdef_field.display_type != field.display_type
                        || cell.paramdef_field.bit_size != field.bit_size
                        || cell.paramdef_field.array_length != field.array_length
                        || cell.paramdef_field.internal_name != field.internal_name
                })
            {
                return Err(invalid(format!(
                    "row {} does not match the applied PARAMDEF",
                    row.id
                )));
            }
        }
        for (index, row) in self.rows.iter().enumerate() {
            if self.headerless_rows && row.id != index as i32 {
                return Err(invalid("headerless row IDs must match their positions"));
            }
            if self.unnamed_rows && row.name.is_some() {
                return Err(invalid("unnamed rows cannot store names"));
            }
        }
        bw.endian = self.endian;
        let extended32 = self
            .format_1
            .contains(FormatFlags1::Flag01 | FormatFlags1::DataOffsets_32bit);
        let extended64 = !extended32 && self.format_1.contains(FormatFlags1::DataOffsets_64bit);
        let long_offsets = self.format_1.contains(FormatFlags1::DataOffsets_64bit);
        bw.reserve_u32("strings-offset")?;
        if extended32 || extended64 {
            bw.write_u16(0)?;
        } else {
            bw.reserve_u16("data-start")?;
        }
        bw.write_u16(self.unk06)?;
        bw.write_u16(self.paramdef_data_version)?;
        bw.write_u16(util::convert_num(self.rows.len())?)?;
        if self.format_1.contains(FormatFlags1::OffsetParamType) {
            bw.write_i32(0)?;
            bw.reserve_i64("param-type-offset")?;
            bw.write_pattern(0x14, 0)?;
        } else {
            let padding = if self.headerless_rows || self.format_1.contains(FormatFlags1::Flag01) {
                0x20
            } else {
                0
            };
            bw.write_fix_shift_jis(&self.param_type, 0x20, padding)?;
        }
        bw.write_u8(if self.endian == Endian::Big { 0xff } else { 0 })?;
        bw.write_u8(self.format_1.bits())?;
        bw.write_u8(self.format_2.bits())?;
        bw.write_u8(self.paramdef_format_version)?;
        if extended32 {
            bw.reserve_u32("data-start")?;
            bw.write_pattern(12, 0)?;
        } else if extended64 {
            bw.reserve_i64("data-start")?;
            bw.write_i64(0)?;
        }
        if !self.headerless_rows {
            for (index, row) in self.rows.iter().enumerate() {
                bw.write_i32(row.id)?;
                if long_offsets {
                    bw.write_i32(0)?;
                    bw.reserve_i64(format!("row-{index}"))?;
                    if !self.unnamed_rows {
                        bw.reserve_i64(format!("name-{index}"))?;
                    }
                } else {
                    bw.reserve_u32(format!("row-{index}"))?;
                    if !self.unnamed_rows {
                        bw.reserve_u32(format!("name-{index}"))?;
                    }
                }
            }
        }
        if self.format_1 == FormatFlags1::Flag01 {
            bw.write_pattern(0x20, 0)?;
        }
        let pos = bw.position()?;
        if extended32 {
            bw.fill_u32("data-start", util::convert_num(pos)?)?;
        } else if extended64 {
            bw.fill_i64("data-start", util::convert_num(pos)?)?;
        } else {
            bw.fill_u16("data-start", util::convert_num(pos)?)?;
        }
        for (index, row) in self.rows.iter().enumerate() {
            if !self.headerless_rows {
                let pos = bw.position()?;
                if long_offsets {
                    bw.fill_i64(format!("row-{index}"), util::convert_num(pos)?)?;
                } else {
                    bw.fill_u32(format!("row-{index}"), util::convert_num(pos)?)?;
                }
            }
            layout::write_cells(bw, &row.cells)?;
        }
        let pos = bw.position()?;
        bw.fill_u32("strings-offset", util::convert_num(pos)?)?;
        if self.format_1.contains(FormatFlags1::OffsetParamType) {
            bw.fill_i64("param-type-offset", util::convert_num(pos)?)?;
            bw.write_ascii(&self.param_type, true)?;
        }
        if !self.unnamed_rows && !self.headerless_rows {
            let empty_offset = bw.position()?;
            bw.write_u16(0)?;
            let mut offsets = HashMap::new();
            offsets.insert("", empty_offset);
            for (index, row) in self.rows.iter().enumerate() {
                let offset = if let Some(name) = row.name.as_deref() {
                    if let Some(&offset) = offsets.get(name) {
                        offset
                    } else {
                        let offset = bw.position()?;
                        if self.format_2.contains(FormatFlags2::UnicodeRowNames) {
                            bw.write_utf16(name, true)?;
                        } else {
                            bw.write_shift_jis(name, true)?;
                        }
                        offsets.insert(name, offset);
                        offset
                    }
                } else {
                    0
                };
                if long_offsets {
                    bw.fill_i64(format!("name-{index}"), util::convert_num(offset)?)?;
                } else {
                    bw.fill_u32(format!("name-{index}"), util::convert_num(offset)?)?;
                }
            }
            bw.write_u16(0)?;
        }
        Ok(())
    }
}

impl ByteIO<Param> for Param {}
impl FileIO<Param> for Param {}
impl DcxIO<Param> for Param {}
