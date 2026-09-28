use crate::io::BinaryReader;
use std::io::{self, Read, Seek};

/// A file in a `Bnd` container.
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub bytes: Vec<u8>,
}

impl File {
    /// Creates a new `File`
    pub fn new(id: Option<i32>, name: Option<impl Into<String>>, bytes: Vec<u8>) -> Self {
        let name = match name {
            Some(text) => Some(text.into()),
            None => None
        };
        Self {
            id,
            name: name,
            bytes,
        }
    }
}

pub struct FileHeader {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub offset: u32,
    pub size: u32,
}

impl FileHeader {
    pub fn from(file: &File) -> Self {
        Self {
            id: file.id,
            name: file.name.clone(),
            offset: 0,
            size: 0,
        }
    }

    pub fn read<R>(br: &mut BinaryReader<R>) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let id = br.read_i32()?;
        let offset = br.read_u32()?;
        let size = br.read_u32()?;
        let name_offset = br.read_u32()?;
        let name = if name_offset != 0 {
            Some(br.get_shift_jis(name_offset as u64)?)
        } else {
            None
        };

        Ok(Self {
            id: (id != -1).then_some(id),
            name,
            offset,
            size,
        })
    }

    pub fn read_file_data<R>(&self, br: &mut BinaryReader<R>) -> io::Result<File>
    where
        R: Read + Seek,
    {
        Ok(File::new(
            self.id,
            self.name.clone(),
            br.get_vec_u8(self.offset as u64, self.size as u64)?,
        ))
    }
}
