use bitflags::bitflags;
use std::{
    io::{self, ErrorKind::InvalidData, Read, Seek, Write},
    path::{Path, PathBuf},
};

use crate::{
    ByteIO, FileIO,
    io::{BinaryReader, BinaryWriter, Endian, StreamIO},
    util,
};

/// Generic binder archive use in games: Metal Wolf Chaos, A.C.E. 2, AC: FF (PSP), AC: NB, AC: LR (PSP+PS2)
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
    pub files: Vec<Binder2File>,
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

    fn read_header<R>(&mut self, br: &mut BinaryReader<R>) -> io::Result<Vec<Binder2FileHeader>>
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
            file_headers.push(Binder2FileHeader::read(
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
        file_headers: &Vec<Binder2FileHeader>,
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
        bw.write_i32(util::try_from_to_io_result(file_headers.len())?)?;
        bw.reserve_i32("base-dir-offset")?;
        bw.write_u16(self.alignment_size)?;
        bw.write_u8(self.file_path_mode as u8)?;
        bw.write_u8(self.unk_1b)?;
        bw.write_u32(0)?;

        if self.file_info_flags.contains(FileInfoFlags::NameOffset) {
            bw.write_u32(0)?;
        }

        for i in 0..file_headers.len() {
            file_headers[i].write(
                bw,
                self.file_path_mode,
                self.file_info_flags,
                util::try_from_to_io_result(i)?,
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
        file_headers: &Vec<Binder2FileHeader>,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        match self.file_path_mode {
            FilePathMode::BaseDirectory => {
                let pos = bw.position()?;
                bw.fill_i32("base-dir-offset", util::try_from_to_io_result(pos)?)?;
                bw.write_shift_jis(&self.base_directory, true)?;
            }
            _ => bw.fill_i32("base-dir-offset", 0)?,
        }

        match self.file_path_mode {
            FilePathMode::Nameless => Ok(()),
            _ => {
                for i in 0..file_headers.len() {
                    let pos = bw.position()?;
                    bw.fill_i32(
                        format!("name-offset-{i}"),
                        util::try_from_to_io_result(pos)?,
                    )?;
                    let mut name = PathBuf::from(&file_headers[i].name);
                    match self.file_path_mode {
                        FilePathMode::FullPath => {
                            if !name.is_absolute() {
                                name = Path::new(r"K:\").join(name);
                            }
                        }
                        _ => (),
                    }

                    bw.write_shift_jis(
                        name.to_str().ok_or(io::Error::new(
                            InvalidData,
                            "Unable to cast path back to Shift-JIS",
                        ))?,
                        true,
                    )?;
                }
                return Ok(());
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
            0 | 1 | 2 => {
                base_dir_offset <= util::try_from_to_io_result(br.length()?)?
                    && base_dir_offset == 0
            }
            3 => base_dir_offset <= util::try_from_to_io_result(br.length()?)?,
            _ => return Ok(false), // Invalid FilePathMode (reason for not casting `u8` to enum)
        };

        let valid_magic = magic == "BND\0";
        let valid_file_version = file_version >= 202 && file_version <= 211;
        let valid_unk_1b = unk_1b == 0 || unk_1b == 1;
        let valid_unk_1c = unk_1c == 0;

        Ok(valid_magic && valid_file_version && valid_names_offset && valid_unk_1b && valid_unk_1c)
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
            file_headers.push(Binder2FileHeader::from(file));
        }

        self.write_header(bw, &file_headers)?;

        for i in 0..self.files.len() {
            file_headers[i].write_file_data(
                bw,
                util::try_from_to_io_result(i)?,
                self.alignment_size,
                &self.files[i].bytes,
            )?;
        }
        let pos = bw.position()?;
        bw.fill_i32("file-size", util::try_from_to_io_result(pos)?)?;

        Ok(())
    }
}

impl ByteIO<BND2> for BND2 {}
impl FileIO<BND2> for BND2 {}

/// A file in `BND2`
pub struct Binder2File {
    /// ID of this `Binder2File`
    pub id: i32,
    /// The name of this `Binder2File`<br>
    /// Will be set to `id` if name does not exist<br>
    /// Will be a path with a drive letter if `FilePathMode::FullPath` is set<br>
    /// Will need `BaseDirectory` added as the base directory if `FilePathMode::BaseDirectory` is set
    pub name: String,
    /// Raw data contained in the `Binder2File`
    pub bytes: Vec<u8>,
}

impl Binder2File {
    /// Initializes a new `Binder2File` with specified parameters
    pub fn new(id: i32, name: String, bytes: Vec<u8>) -> Self {
        Self { id, name, bytes }
    }

    /// Creates an empty `Binder2File`
    pub fn empty() -> Self {
        Self {
            id: -1,
            name: String::new(),
            bytes: Vec::new(),
        }
    }
}

struct Binder2FileHeader {
    id: i32,
    name: String,
    offset: i32,
    size: i32,
}

impl Binder2FileHeader {
    fn empty() -> Self {
        Self {
            id: -1,
            name: String::new(),
            offset: -1,
            size: -1,
        }
    }

    fn from(file: &Binder2File) -> Self {
        Self {
            id: file.id,
            name: file.name.clone(),
            offset: -1,
            size: -1,
        }
    }

    fn read<R>(
        br: &mut BinaryReader<R>,
        path_mode: FilePathMode,
        info_flags: FileInfoFlags,
    ) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let mut out = Self::empty();
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

    fn write<W>(
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

    fn read_file_data<R>(&self, br: &mut BinaryReader<R>) -> io::Result<Binder2File>
    where
        R: Read + Seek,
    {
        let bytes = br.get_u8_vec(self.offset as u64, self.size as u64)?;
        Ok(Binder2File::new(self.id, self.name.clone(), bytes))
    }

    fn write_file_data<W>(
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
        let offset: i32 = util::try_from_to_io_result(bw.position()?)?;
        let size: i32 = util::try_from_to_io_result(bytes.len())?;
        bw.write_u8_vec(bytes.to_vec())?;
        bw.fill_i32(format!("file-offset-{index}"), offset)?;
        bw.fill_i32(format!("file-size-{index}"), size)?;

        Ok(())
    }
}

#[derive(Clone, Copy)]
/// An enum for the different supported file path modes
pub enum FilePathMode {
    /// Files in this BND have no name
    Nameless = 0,
    /// Files in this BND only have file names
    FileName = 1,
    /// All files use a full file path
    FullPath = 2,
    /// Add a base directory all paths start from, then write the rest of the path as each file name
    BaseDirectory = 3,
}

impl TryFrom<u8> for FilePathMode {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Nameless),
            1 => Ok(Self::FileName),
            2 => Ok(Self::FullPath),
            3 => Ok(Self::BaseDirectory),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid enum value",
            )),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    /// Header Info flags describing what features are enabled
    pub struct HeaderInfoFlags: u8 {
        const HeaderItem = 0b00000001;
        const Endian = 0b00000010;
        const FileVersion = 0b00000100;
        const FileSize = 0b00001000;
        const FileNum = 0b00010000;
        const BaseDirOffset = 0b00100000;
        const AlignmentSize = 0b01000000;
        const Option = 0b10000000;
    }
}

impl TryFrom<u8> for HeaderInfoFlags {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid format flags"))
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    /// File Info flags describing what features are enabled
    pub struct FileInfoFlags: u8 {
        const ID = 0b00000001;
        const Offset = 0b00000010;
        const Size = 0b00000100;
        const NameOffset = 0b00001000;
        const Flag5 = 0b00010000;
        const Flag6 = 0b00100000;
        const Flag7 = 0b01000000;
        const Flag8 = 0b10000000;
    }
}

impl TryFrom<u8> for FileInfoFlags {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid format flags"))
    }
}
