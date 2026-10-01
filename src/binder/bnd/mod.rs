use std::io::{self, Read, Seek, Write};

use crate::{
    ByteIO, FileIO,
    io::{BinaryReader, BinaryWriter, StreamIO},
    util,
};

mod file;
pub use file::File;
use file::FileHeader;

/// Bnd file, used in old titles
#[derive(Debug, Clone, PartialEq)]
pub struct Bnd {
    /// `Bnd` version
    pub internal_version: i32,
    /// First `Bnd` format definition
    pub format0: u16,
    /// The second Bnd format definition.
    /// Bit 0 determines if filenames exist.
    /// Bit 1 determines if `root_file_path` exists
    pub format1: u16,
    /// Files in the `Bnd`
    pub files: Vec<File>,
    /// Root file path. Not all `Bnd` have this
    pub root_file_path: Option<String>,
}

impl Default for Bnd {
    fn default() -> Self {
        Self {
            internal_version: -1,
            format0: Default::default(),
            format1: Default::default(),
            files: Default::default(),
            root_file_path: Default::default(),
        }
    }
}

impl Bnd {
    fn read_header<R>(&mut self, br: &mut BinaryReader<R>) -> io::Result<Vec<FileHeader>>
    where
        R: Read + Seek,
    {
        br.assert_ascii(&["BND\0"])?;
        br.assert_u16(&[0xFFFF])?;
        br.assert_u16(&[0])?;
        self.internal_version = br.read_i32()?;
        br.read_u32()?;
        let file_count = br.read_i32()?;
        let root_file_path_offset = br.read_i32()?;
        self.root_file_path = if root_file_path_offset != 0 {
            Some(br.get_ascii(root_file_path_offset as u64)?)
        } else {
            None
        };

        self.format0 = br.read_u16()?;
        self.format1 = br.read_u16()?;
        br.assert_i32(&[0])?;

        let mut file_headers = Vec::with_capacity(file_count as usize);
        for _ in 0..file_count {
            file_headers.push(FileHeader::read(br)?);
        }
        Ok(file_headers)
    }
}

impl StreamIO<Bnd> for Bnd {
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<Bnd>
    where
        R: Read + Seek,
    {
        let mut out = Self::default();
        let file_headers = out.read_header(br)?;
        for header in file_headers {
            out.files.push(header.read_file_data(br)?);
        }
        Ok(out)
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let file_headers: Vec<_> = self.files.iter().map(FileHeader::from).collect();

        bw.write_ascii("BND", true)?;
        bw.write_u16(0xFFFF)?;
        bw.write_u16(0)?;
        bw.write_i32(self.internal_version)?;
        bw.reserve_i32("file-size")?;
        bw.write_i32(util::convert_num(self.files.len())?)?;
        bw.reserve_i32("root-file-path")?;
        bw.write_u16(self.format0)?;
        bw.write_u16(self.format1)?;
        bw.write_u32(0)?;

        for (index, (header, file)) in file_headers.iter().zip(&self.files).enumerate() {
            if let Some(id) = header.id {
                bw.write_i32(id)?;
            } else {
                bw.write_i32(-1)?;
            }

            bw.reserve_i32(format!("file-offset-{index}"))?;
            bw.write_i32(util::convert_num(file.bytes.len())?)?;
            bw.reserve_i32(format!("file-name-{index}"))?;
        }

        match &self.root_file_path {
            Some(path) => {
                let position = bw.position()?;
                bw.fill_i32("root-file-path", util::convert_num(position)?)?;
                bw.write_shift_jis(path, true)?;
            }
            None => bw.fill_i32("root-file-path", 0)?,
        }

        for (index, header) in file_headers.iter().enumerate() {
            if let Some(name) = &header.name {
                let position = bw.position()?;
                bw.fill_i32(format!("file-name-{index}"), util::convert_num(position)?)?;
                bw.write_shift_jis(name, true)?;
            } else {
                bw.fill_i32(format!("file-name-{index}"), 0)?;
            }
        }
        bw.pad_00(0x10)?;

        for (index, file) in self.files.iter().enumerate() {
            let position = bw.position()?;
            bw.fill_i32(format!("file-offset-{index}"), util::convert_num(position)?)?;
            bw.write_bytes(&file.bytes)?;
            if index + 1 != self.files.len() {
                bw.pad_00(0x10)?;
            }
        }

        let position = bw.position()?;
        bw.fill_i32("file-size", util::convert_num(position)?)?;
        Ok(())
    }

    fn is<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek,
    {
        if br.length()? < 4 {
            return Ok(false);
        }
        Ok(br.get_ascii_len(0, 4)? == "BND\0")
    }
}

impl ByteIO<Bnd> for Bnd {}
impl FileIO<Bnd> for Bnd {}
