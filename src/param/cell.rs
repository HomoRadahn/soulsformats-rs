use std::{fmt, io};

use crate::{param::{CellValue, CellValueExt}, paramdef::Field};

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The [`Field`] describing this cell
    pub paramdef_field: Field,
    /// The value held by of this cell
    pub value: CellValue,
}

impl Cell {
    /// Creates a new [`Cell`]
    pub fn new(paramdef_field: Field, value: CellValue) -> Self {
        Self { paramdef_field, value }
    }

    /// Sets payload carried by value of the cell
    pub fn set<T: CellValueExt>(&mut self, value: T) -> io::Result<()> {
        value.set_inner(&mut self.value)
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.paramdef_field.internal_name {
            write!(f, "{:?} {} = {:?}", self.paramdef_field.display_type, name, self.value)
        }
        else {
            write!(f, "{:?} = {:?}", self.paramdef_field.display_type, self.value)
        }
    }
}
