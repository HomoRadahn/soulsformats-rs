use std::{io, sync::Arc};

use crate::{
    ParamDef,
    param::{Cell, CellValue, layout},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The shared [`ParamDef`] that describes this row. Before applying a definition,
    /// rows read from a file contain a placeholder.
    pub paramdef: Arc<ParamDef>,
    /// ID of this row.
    pub id: i32,
    /// Name given to this row; no functional significance.
    pub name: Option<String>,
    /// Cells in this row, described by `paramdef`.
    pub cells: Vec<Cell>,
    pub(crate) data_offset: u64,
}

impl Row {
    /// Creates a row with one default-valued cell per field in `paramdef`.
    ///
    /// # Panics
    ///
    /// Panics if a field has an invalid default or layout. Use [`Self::try_new`]
    /// to handle invalid definitions.
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
        Self::try_new(id, name, paramdef).expect("invalid PARAMDEF row defaults")
    }

    /// Creates a row with default values, returning an error for an invalid definition.
    pub fn try_new(id: i32, name: Option<String>, paramdef: &ParamDef) -> io::Result<Self> {
        Self::try_new_for_version(id, name, paramdef, u64::MAX)
    }

    /// Creates a row containing fields valid for the specified regulation version.
    pub fn try_new_for_version(
        id: i32,
        name: Option<String>,
        paramdef: &ParamDef,
        version: u64,
    ) -> io::Result<Self> {
        let mut fields = Vec::with_capacity(paramdef.fields.len());
        for field in &paramdef.fields {
            if !paramdef.version_aware || field.is_valid_version(version)? {
                fields.push(Arc::new(field.clone()));
            }
        }
        layout::row_size(&fields)?;
        let cells = fields
            .into_iter()
            .map(|field| {
                let value = layout::default_value(&field)?;
                Ok(Cell {
                    paramdef_field: field,
                    value,
                })
            })
            .collect::<io::Result<_>>()?;
        Ok(Self {
            id,
            name,
            cells,
            paramdef: Arc::new(paramdef.clone()),
            data_offset: 0,
        })
    }

    /// Returns the value of the first cell with the given internal name, if present.
    pub fn get_value_by_internal_name(
        &self,
        internal_name: impl Into<String>,
    ) -> Option<&CellValue> {
        let internal_name = internal_name.into();

        if let Some(cell) = self
            .cells
            .iter()
            .find(|n| n.paramdef_field.internal_name.as_ref() == Some(&internal_name))
        {
            return Some(&cell.value);
        }

        None
    }

    /// Returns the first cell with the given internal name.
    pub fn get_cell(&self, internal_name: &str) -> Option<&Cell> {
        self.cells
            .iter()
            .find(|cell| cell.paramdef_field.internal_name.as_deref() == Some(internal_name))
    }

    /// Returns the first cell with the given internal name for editing.
    pub fn get_cell_mut(&mut self, internal_name: &str) -> Option<&mut Cell> {
        self.cells
            .iter_mut()
            .find(|cell| cell.paramdef_field.internal_name.as_deref() == Some(internal_name))
    }
}
