use std::io::Cursor;

use crate::io::{BinaryReader, Endian};

pub mod cell;
pub mod format;
pub mod paramdef;
pub mod row;

use format::{FormatFlags1, FormatFlags2};
use paramdef::PARAMDEF;
use row::Row;

pub struct PARAM {
    /// Endianness of the data
    pub endian: Endian,
    /// First set of format flags
    pub format_1: FormatFlags1,
    /// Second set of format flags
    pub format_2: FormatFlags2,
    /// Matches `PARAMDEF` for version `101`, otherwise `0` or `0xFF`
    pub paramdef_format_version: u8,
    pub unk06: u16,
    /// Indicates a revision of the row data structure
    pub paramdef_data_version: u16,
    /// Identifies corresponding `PARAM` and `PARAMDEF`
    pub param_type: String,
    /// Automatically determined based on spacing of row offsets; `None` if `PARAM` had no rows
    pub detected_size: Option<u64>,
    /// The rows of this param; must be loaded with `self.apply_paramdef()` before cells can be used
    pub rows: Vec<Row>,
    /// The currently applied `PARAMDEF`
    pub paramdef: PARAMDEF,
    /// Whether or not rows do not support names. Only known to happen in Chromehounds
    pub unnamed_rows: bool,
    /// Whether or not rows are headerless. Only known to happen in AC: Formula Front
    pub headerless_rows: bool,
    row_reader: BinaryReader<Cursor<Vec<u8>>>,
}
