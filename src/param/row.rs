use std::io;

use crate::{Param, ParamDef, io::BinaryReader, param::{Cell, CellValue}};

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// [`ParamDef`] that describes this row
    pub paramdef: ParamDef,
    /// ID of this row
    pub id: i32,
    /// Name given to this row; no functional significance
    pub name: Option<String>,
    /// Cells contained in this row. Must be loaded with [`ParamDef`] before use
    pub cells: Vec<Cell>,
    pub(crate) data_offset: u64
}

impl Row {
    /// Creates a new [`Row`], with cells having default values set from [`ParamDef`]
    pub fn new(id: i32, name: Option<String>, paramdef: &ParamDef) -> Self {
        let mut cells = Vec::new();
        for field in paramdef.fields.iter() {
            cells.push(Cell::new(field.to_owned(), field.default.to_owned()));
        }

        Self { id, name, cells, paramdef: paramdef.to_owned(), data_offset: 0 }
    }

    /// Gets the [`CellValue`] of a [`Cell`] with the corresponding `internal_name`
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