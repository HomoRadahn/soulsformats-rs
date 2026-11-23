use std::io;

use crate::{Param, ParamDef, io::BinaryReader, param::{Cell, CellValue}};

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// [`ParamDef`] that describes this row.
    pub paramdef: ParamDef,
    /// ID of this row.
    pub id: i32,
    /// Name given to this row; no functional significance.
    pub name: Option<String>,
    /// Cells in this row, described by `paramdef`.
    pub cells: Vec<Cell>,
    pub(crate) data_offset: u64
}

impl Row {
    /// Creates a row with one default-valued cell per field in `paramdef`.
    ///
    /// ```
    /// use soulsformats_rs::{ParamDef, param::{CellValue, Row}, paramdef::{Field, ParamDefType}};
    ///
    /// let mut definition = ParamDef::default();
    /// definition.fields.push(Field::new(ParamDefType::I32, "health"));
    /// let mut row = Row::new(1, None, &definition);
    /// row.cells[0].set(100_i32)?;
    /// assert_eq!(row.get_value_by_internal_name("health"), Some(&CellValue::I32(100)));
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn new(id: i32, name: Option<String>, paramdef: &ParamDef) -> Self {
        let cells = paramdef.fields.iter()
            .map(|field| Cell::new(field.clone(), field.default.clone()))
            .collect();

        Self { id, name, cells, paramdef: paramdef.to_owned(), data_offset: 0 }
    }

    /// Returns the value of the first cell with the given internal name, if present.
    pub fn get_value_by_internal_name(&self, internal_name: impl Into<String>) -> Option<&CellValue> {
        let internal_name = internal_name.into();

        if let Some(cell) = self.cells.iter().find(|n| n.paramdef_field.internal_name.as_ref() == Some(&internal_name)) {
            return Some(&cell.value);
        }

        None
    }

    pub(crate) fn read<R>(br: &mut BinaryReader<R>, param: &Param, actual_string_offset: u64) -> io::Result<Self> {
        todo!()
    }
}