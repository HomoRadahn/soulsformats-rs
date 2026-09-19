use std::io::{self, Read, Seek, Write};
use bitflags::bitflags;

use crate::{ByteIO, FileIO, dcx::compression_info::CompressionInfo, io::{BinaryReader, BinaryWriter, StreamIO}};

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
    pub files: Vec<File>,
    /// `DCX` compression info
    pub compression: CompressionInfo
}

impl BND2 {
    /// Creates `BND2`
    pub fn new(compression: CompressionInfo) -> Self {
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
            compression
        }
    }

    /// Creates `BND2` with specified version
    pub fn with_version(version: i32, compression: CompressionInfo) -> Self {
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
            compression
        }
    }

    /// Creates `BND2` with specified `FilePathMode`
    pub fn with_path_mode(file_path_mode: FilePathMode, compression: CompressionInfo) -> Self {
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
            compression
        }
    }

    pub fn with_version_path_mode(version: i32, file_path_mode: FilePathMode, compression: CompressionInfo) -> Self {
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
            compression
        }
    }
}

impl StreamIO<BND2> for BND2 {
    fn is<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek,
    {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Is function not implemented for type: {}", stringify!(T)),
        ))
    }
    
    fn get_compression(&self) -> CompressionInfo {
        self.compression
    }
    
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<BND2>
    where
        R: Read + Seek {
        todo!()
    }
    
    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek {
        todo!()
    }
}

impl ByteIO<BND2> for BND2 {}
impl FileIO<BND2> for BND2 {}

pub struct File {

}

/// `BND2` - An enum for the different supported file path modes
pub enum FilePathMode {
    /// Files in this BND have no name
    Nameless = 0,
    /// Files in this BND only have file names
    FileName = 1,
    /// All files use a full file path
    FullPath = 2,
    /// Add a base directory all paths start from, then write the rest of the path as each file name
    BaseDirectory = 3
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
    /// `BND2` - Header Info flags describing what features are enabled
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

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    /// `BND2` - File Info flags describing what features are enabled
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