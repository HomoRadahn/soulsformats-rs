use std::{
    io::{self, ErrorKind::InvalidData, Read, Seek, Write},
    path::{Path, PathBuf},
};

use crate::{
    ByteIO, FileIO,
    io::{BinaryReader, BinaryWriter, Endian, StreamIO},
    util,
};

pub mod file;
pub mod format;

pub use format::*;
pub use file::*;

/// Generic binder archive use in games: Metal Wolf Chaos, A.C.E. 2, AC: FF (PSP), AC: NB, AC: LR (PSP+PS2)
#[derive(Debug, Clone, PartialEq)]
pub struct BND2 {
    /// Header Info Flags
    pub header_info_flags: HeaderInfoFlags,
    /// File Info Flags
    pub file_info_flags: FileInfoFlags,
    pub unk_06: u8,
    pub unk_07: u8,
    /// Version of `BND2` - usually `202` or `211`
    pub file_version: i32,
    /// Alignment of each `File`
    pub alignment_size: u16,
    /// File Path Mode
    pub file_path_mode: FilePathMode,
    pub unk_1b: u8,
    /// Base directory of all files - used only if `FilePathMode::BaseDirectory` is set
    pub base_directory: String,
    /// Files contained in this `BND2`
    pub files: Vec<File>,
}

impl BND2 {
    /// Creates an empty `BND2`
    pub fn empty() -> Self {
        Self {
            header_info_flags: HeaderInfoFlags::all(),
            file_info_flags: FileInfoFlags::all(),
            unk_06: 0x00,
            unk_07: 0x00,
            file_version: 211,
            alignment_size: 2048,
            file_path_mode: FilePathMode::FileName,
            unk_1b: 0,
            base_directory: String::new(),
            files: Vec::new(),
        }
    }

    /// Creates `BND2` with specified version
    pub fn with_version(version: i32) -> Self {
        Self {
            header_info_flags: HeaderInfoFlags::empty(),
            file_info_flags: FileInfoFlags::empty(),
            unk_06: 0x00,
            unk_07: 0x00,
            file_version: version,
            alignment_size: 2048,
            file_path_mode: FilePathMode::FileName,
            unk_1b: 0,
            base_directory: String::new(),
            files: Vec::new(),
        }
    }

    /// Creates `BND2` with specified `FilePathMode`
    pub fn with_path_mode(file_path_mode: FilePathMode) -> Self {
        Self {
            header_info_flags: HeaderInfoFlags::empty(),
            file_info_flags: FileInfoFlags::empty(),
            unk_06: 0x00,
            unk_07: 0x00,
            file_version: 211,
            alignment_size: 2048,
            file_path_mode,
            unk_1b: 0,
            base_directory: String::new(),
            files: Vec::new(),
        }
    }

    pub fn with_version_path_mode(version: i32, file_path_mode: FilePathMode) -> Self {
        Self {
            header_info_flags: HeaderInfoFlags::empty(),
            file_info_flags: FileInfoFlags::empty(),
            unk_06: 0x00,
            unk_07: 0x00,
            file_version: version,
            alignment_size: 2048,
            file_path_mode,
            unk_1b: 0,
            base_directory: String::new(),
            files: Vec::new(),
        }
    }

    fn read_header<R>(&mut self, br: &mut BinaryReader<R>) -> io::Result<Vec<FileHeader>>
    where
        R: Read + Seek,
    {
        br.endian = Endian::Little;

        br.assert_ascii(&["BND\0"])?;
        self.header_info_flags = HeaderInfoFlags::try_from(br.read_u8()?)?;
        self.file_info_flags = FileInfoFlags::try_from(br.read_u8()?)?;
        self.unk_06 = br.read_u8()?;
        self.unk_07 = br.read_u8()?;
        self.file_version = br.read_i32()?;
        br.skip(4)?;
        let file_count = br.read_i32()?;
        let base_dir_offset = br.read_i32()?;
        self.alignment_size = br.read_u16()?;
        self.file_path_mode = br.read_enum_u8::<FilePathMode>()?;
        self.unk_1b = br.assert_u8(&[0, 1])?;
        br.assert_u32(&[0])?;

        if self.file_info_flags.contains(FileInfoFlags::NameOffset) {
            br.assert_u32(&[0])?;
        }

        match self.file_path_mode {
            FilePathMode::BaseDirectory
                if self.file_info_flags.contains(FileInfoFlags::NameOffset) =>
            {
                self.base_directory = br.get_shift_jis(base_dir_offset as u64)?
            }
            _ => self.base_directory = String::new(),
        }

        let mut file_headers = Vec::with_capacity(file_count as usize);

        for _ in 0..file_count {
            file_headers.push(FileHeader::read(
                br,
                self.file_path_mode,
                self.file_info_flags,
            )?);
        }

        Ok(file_headers)
    }

    fn write_header<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        file_headers: &[FileHeader],
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bw.endian = Endian::Little;

        bw.write_ascii("BND", true)?;
        bw.write_u8(self.header_info_flags.bits())?;
        bw.write_u8(self.file_info_flags.bits())?;
        bw.write_u8(self.unk_06)?;
        bw.write_u8(self.unk_07)?;
        bw.write_i32(self.file_version)?;
        bw.reserve_i32("file-size")?;
        bw.write_i32(util::convert_num(file_headers.len())?)?;
        bw.reserve_i32("base-dir-offset")?;
        bw.write_u16(self.alignment_size)?;
        bw.write_u8(self.file_path_mode as u8)?;
        bw.write_u8(self.unk_1b)?;
        bw.write_u32(0)?;

        if self.file_info_flags.contains(FileInfoFlags::NameOffset) {
            bw.write_u32(0)?;
        }

        for (index, header) in file_headers.iter().enumerate() {
            header.write(
                bw,
                self.file_path_mode,
                self.file_info_flags,
                util::convert_num(index)?,
            )?;
        }

        if self.file_info_flags.contains(FileInfoFlags::NameOffset) {
            self.write_file_names(bw, file_headers)?;
        } else {
            bw.fill_i32("base-dir-offset", 0)?;
        }

        Ok(())
    }

    fn write_file_names<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        file_headers: &[FileHeader],
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        match self.file_path_mode {
            FilePathMode::BaseDirectory => {
                let pos = bw.position()?;
                bw.fill_i32("base-dir-offset", util::convert_num(pos)?)?;
                bw.write_shift_jis(&self.base_directory, true)?;
            }
            _ => bw.fill_i32("base-dir-offset", 0)?,
        }

        match self.file_path_mode {
            FilePathMode::Nameless => Ok(()),
            _ => {
                for (index, header) in file_headers.iter().enumerate() {
                    let pos = bw.position()?;
                    bw.fill_i32(format!("name-offset-{index}"), util::convert_num(pos)?)?;
                    let mut name = PathBuf::from(&header.name);

                    if self.file_path_mode == FilePathMode::FullPath && !name.is_absolute() {
                        name = Path::new(r"K:\").join(name);
                    }

                    bw.write_shift_jis(
                        name.to_str().ok_or(io::Error::new(
                            InvalidData,
                            "Unable to cast path back to Shift-JIS",
                        ))?,
                        true,
                    )?;
                }
                Ok(())
            }
        }
    }
}

impl StreamIO<BND2> for BND2 {
    fn is<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek,
    {
        if br.length()? < 32 {
            return Ok(false);
        }

        let magic = br.read_ascii_len(4)?;
        br.skip(4)?;
        let file_version = br.read_i32()?;
        br.skip(8)?;
        let base_dir_offset = br.read_i32()?;
        br.skip(2)?;
        let file_path_mode = br.read_u8()?;
        let unk_1b = br.read_u8()?;
        let unk_1c = br.read_u32()?;

        let valid_names_offset = match file_path_mode {
            0..=2 => base_dir_offset <= util::convert_num(br.length()?)? && base_dir_offset == 0,
            3 => base_dir_offset <= util::convert_num(br.length()?)?,
            _ => return Ok(false), // Invalid FilePathMode (reason for not casting `u8` to enum)
        };

        Ok(magic == "BND\0"
            && (202..=211).contains(&file_version)
            && valid_names_offset
            && (unk_1b == 0 || unk_1b == 1)
            && unk_1c == 0)
    }

    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<BND2>
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
        let mut file_headers = Vec::with_capacity(self.files.len());
        for file in &self.files {
            file_headers.push(FileHeader::from(file));
        }

        self.write_header(bw, &file_headers)?;

        for (index, header) in file_headers.iter().enumerate() {
            header.write_file_data(
                bw,
                util::convert_num(index)?,
                self.alignment_size,
                &self.files[index].bytes,
            )?;
        }
        let pos = bw.position()?;
        bw.fill_i32("file-size", util::convert_num(pos)?)?;

        Ok(())
    }
}

impl ByteIO<BND2> for BND2 {}
impl FileIO<BND2> for BND2 {}
