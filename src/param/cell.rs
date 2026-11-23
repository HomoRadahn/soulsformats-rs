use std::{fmt, io, sync::Arc};

use crate::{
    param::{CellValue, CellValueExt},
    paramdef::Field,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The shared [`Field`] describing this cell.
    pub paramdef_field: Arc<Field>,
    /// The value held by this cell.
    pub value: CellValue,
}

impl Cell {
    /// Creates a new [`Cell`]
    pub fn new(paramdef_field: Field, value: CellValue) -> Self {
        Self {
            paramdef_field: Arc::new(paramdef_field),
            value,
        }
    }

    /// Updates the value's payload, returning an error if its variant is incompatible.
    pub fn set<T: CellValueExt>(&mut self, value: T) -> io::Result<()> {
        value.set_inner(&mut self.value)
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.paramdef_field.internal_name {
            write!(
                f,
                "{:?} {} = {:?}",
                self.paramdef_field.display_type, name, self.value
            )
        } else {
            write!(
                f,
                "{:?} = {:?}",
                self.paramdef_field.display_type, self.value
            )
        }
    }
}
