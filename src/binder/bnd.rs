use std::io::{self, Read, Seek, Write};

use crate::{
    io::{BinaryReader, BinaryWriter, StreamIO},
    ByteIO, FileIO, util,
};

/// A file in a `BND` container.
pub struct Binder1File {
    pub id: i32,
    pub name: String,
    pub bytes: Vec<u8>,
}

impl Binder1File {
    pub fn new(id: i32, name: String, bytes: Vec<u8>) -> Self {
        Self { id, name, bytes }
    }
}

struct Binder1FileHeader {
    id: i32,
    name: String,
    offset: u32,
    size: u32,
}

impl Binder1FileHeader {
    fn from(file: &Binder1File) -> Self {
        Self {
            id: file.id,
            name: file.name.clone(),
            offset: 0,
            size: 0,
        }
    }

    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let id = br.read_i32()?;
        let offset = br.read_u32()?;
        let size = br.read_u32()?;
        let name_offset = br.read_u32()?;
        let name = if name_offset != 0 {
            br.get_shift_jis(name_offset as u64)?
        } else {
            format!("file_{id}")
        };

        Ok(Self {
            id,
            name,
            offset,
            size,
        })
    }

    fn read_file_data<R>(&self, br: &mut BinaryReader<R>) -> io::Result<Binder1File>
    where
        R: Read + Seek,
    {
        Ok(Binder1File::new(
            self.id,
            self.name.clone(),
            br.get_u8_vec(self.offset as u64, self.size as u64)?,
        ))
    }
}

/// BND file, used in old titles
pub struct BND {
    /// `BND` version
    pub internal_version: i32,
    /// First `BND` format definition
    pub format0: u16,
    /// The second BND format definition.
    /// Bit 0 determines if filenames exist.
    /// Bit 1 determines if `root_file_path` exists
    pub format1: u16,
    /// Files in the `BND`
    pub files: Vec<Binder1File>,
    /// Root file path. Not all `BND` have this
    pub root_file_path: Option<String>,
}

impl BND {
    /// Initializes an empty `BND`
    pub fn empty() -> Self {
        Self {
            internal_version: -1,
            format0: 0,
            format1: 0,
            files: Vec::new(),
            root_file_path: None,
        }
    }

    fn read_header<R>(&mut self, br: &mut BinaryReader<R>) -> io::Result<Vec<Binder1FileHeader>>
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
            file_headers.push(Binder1FileHeader::read(br)?);
        }
        Ok(file_headers)
    }
}

impl StreamIO<BND> for BND {
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<BND>
    where
        R: Read + Seek,
    {
        let mut out = Self::empty();
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
        let file_headers: Vec<_> = self
            .files
            .iter()
            .map(Binder1FileHeader::from)
            .collect();

        bw.write_ascii("BND", true)?;
        bw.write_u16(0xFFFF)?;
        bw.write_u16(0)?;
        bw.write_i32(self.internal_version)?;
        bw.reserve_i32("file-size")?;
        bw.write_i32(util::try_from_to_io_result(self.files.len())?)?;
        bw.reserve_i32("root-file-path")?;
        bw.write_u16(self.format0)?;
        bw.write_u16(self.format1)?;
        bw.write_u32(0)?;

        for (index, (header, file)) in file_headers.iter().zip(&self.files).enumerate() {
            bw.write_i32(header.id)?;
            bw.reserve_i32(format!("file-offset-{index}"))?;
            bw.write_i32(util::try_from_to_io_result(file.bytes.len())?)?;
            bw.reserve_i32(format!("file-name-{index}"))?;
        }

        match &self.root_file_path {
            Some(path) => {
                let position = bw.position()?;
                bw.fill_i32("root-file-path", util::try_from_to_io_result(position)?)?;
                bw.write_shift_jis(path, true)?;
            }
            None => bw.fill_i32("root-file-path", 0)?,
        }

        for (index, header) in file_headers.iter().enumerate() {
            let position = bw.position()?;
            bw.fill_i32(
                format!("file-name-{index}"),
                util::try_from_to_io_result(position)?,
            )?;
            bw.write_shift_jis(&header.name, true)?;
        }
        bw.pad_00(0x10)?;

        for (index, file) in self.files.iter().enumerate() {
            let position = bw.position()?;
            bw.fill_i32(
                format!("file-offset-{index}"),
                util::try_from_to_io_result(position)?,
            )?;
            bw.write_bytes(&file.bytes)?;
            if index + 1 != self.files.len() {
                bw.pad_00(0x10)?;
            }
        }

        let position = bw.position()?;
        bw.fill_i32("file-size", util::try_from_to_io_result(position)?)?;
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

impl ByteIO<BND> for BND {}
impl FileIO<BND> for BND {}
