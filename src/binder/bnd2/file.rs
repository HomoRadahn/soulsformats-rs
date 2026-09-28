use std::io::{self, Read, Seek, Write};

use crate::{
    binder::bnd2::{FileInfoFlags, FilePathMode},
    io::{BinaryReader, BinaryWriter},
    util,
};

/// A file in `Bnd2`
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    /// ID of this `File`, or `None` if it has no ID
    pub id: Option<i32>,
    /// The name of this `File`<br>
    /// Will be set to `id` if name does not exist<br>
    /// Will be a path with a drive letter if `FilePathMode::FullPath` is set<br>
    /// Will need `BaseDirectory` added as the base directory if `FilePathMode::BaseDirectory` is set
    pub name: String,
    /// Raw data contained in the `File`
    pub bytes: Vec<u8>,
}

impl File {
    /// Initializes a new `File` with specified parameters
    pub fn new(id: Option<i32>, name: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self { id, name: name.into(), bytes }
    }
}

impl Default for File {
    fn default() -> Self {
        Self {
            id: None,
            name: Default::default(),
            bytes: Default::default(),
        }
    }
}

pub(crate) struct FileHeader {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) offset: i32,
    pub(crate) size: i32,
}

impl Default for FileHeader {
    fn default() -> Self {
        Self {
            id: -1,
            name: Default::default(),
            offset: -1,
            size: -1,
        }
    }
}

impl FileHeader {
    pub(crate) fn from(file: &File) -> Self {
        Self {
            id: file.id.unwrap_or(-1),
            name: file.name.clone(),
            offset: -1,
            size: -1,
        }
    }

    pub(crate) fn read<R>(
        br: &mut BinaryReader<R>,
        path_mode: FilePathMode,
        info_flags: FileInfoFlags,
    ) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let mut out = Self::default();
        out.id = br.read_i32()?;
        out.offset = br.read_i32()?;
        out.size = br.read_i32()?;

        if info_flags.contains(FileInfoFlags::NameOffset) {
            let name_offset = br.read_i32()?;

            match path_mode {
                FilePathMode::Nameless => out.name = format!("{}", out.id),
                _ => out.name = br.get_shift_jis(name_offset as u64)?,
            }
        }

        Ok(out)
    }

    pub(crate) fn write<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        path_mode: FilePathMode,
        info_flags: FileInfoFlags,
        index: i32,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bw.write_i32(self.id)?;
        bw.reserve_i32(format!("file-offset-{index}"))?;
        bw.reserve_i32(format!("file-size-{index}"))?;

        if info_flags.contains(FileInfoFlags::NameOffset) {
            match path_mode {
                FilePathMode::Nameless => bw.write_i32(0)?,
                _ => bw.reserve_i32(format!("name-offset-{index}"))?,
            }
        }

        Ok(())
    }

    pub(crate) fn read_file_data<R>(&self, br: &mut BinaryReader<R>) -> io::Result<File>
    where
        R: Read + Seek,
    {
        let bytes = br.get_vec_u8(self.offset as u64, self.size as u64)?;
        Ok(File::new(
            (self.id != -1).then_some(self.id),
            self.name.clone(),
            bytes,
        ))
    }

    pub(crate) fn write_file_data<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        index: i32,
        alignment_size: u16,
        bytes: &[u8],
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bw.pad_00(alignment_size as u64)?;
        let offset: i32 = util::convert_num(bw.position()?)?;
        let size: i32 = util::convert_num(bytes.len())?;
        bw.write_vec_u8(bytes.to_vec())?;
        bw.fill_i32(format!("file-offset-{index}"), offset)?;
        bw.fill_i32(format!("file-size-{index}"), size)?;

        Ok(())
    }
}
