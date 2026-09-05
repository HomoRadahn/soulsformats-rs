use std::io::{self, Read, Seek, Write};

use crate::{DCX, bnd::binder::{FileFlags, Format}, dcx::compression_info::CompressionInfo, io::{BinaryReader, BinaryWriter, Endian}, util};

/// A generic file in `BND3`, `BND4`, `BXF3`, `BXF4` containers
pub struct BinderFile {
    pub flags: FileFlags,
    pub id: i32,
    pub name: Option<String>,
    pub bytes: Vec<u8>,
    pub compression: CompressionInfo
}

impl BinderFile {
    /// Creates a new `BinderFile`, with `Zlib` compression
    pub fn new(flags: FileFlags, id: i32, name: Option<String>, bytes: Vec<u8>, compression: CompressionInfo) -> Self {
        Self {
            flags, id, name, bytes, compression
        }
    }
}

/// Metadata for a file in a binder container
pub(crate) struct BinderFileHeader {
    pub flags: FileFlags,
    pub id: i32,
    pub name: Option<String>,
    pub compression: CompressionInfo,
    pub compressed_size: i64,
    pub uncompressed_size: i64,
    pub data_offset: i64,
}

impl BinderFileHeader {
    /// Creates a new `BinderFileHeader`, with `Zlib` compression
    pub(crate) fn new(flags: FileFlags, id: i32, name: Option<String>, compressed_size: i64, uncompressed_size: i64, data_offset: i64) -> Self {
        Self {
            flags,
            id,
            name,
            compression: CompressionInfo::Zlib,
            compressed_size,
            uncompressed_size,
            data_offset
        }
    }

    /// Creates a `BinderFileHeader` from `BinderFile`
    pub(crate) fn from_binder_file(file: &BinderFile) -> Self {
        Self {
            flags: file.flags,
            id: file.id,
            name: file.name.clone(),
            compression: file.compression,
            compressed_size: -1,
            uncompressed_size: -1,
            data_offset: -1,
        }
    }

    /// Reads a `BND3` `BinderFileHeader` from `BinaryReader`
    pub(crate) fn read_bnd3_header<R>(br: &mut BinaryReader<R>, format: &Format, bit_endian: Endian) -> io::Result<Self>
    where 
        R: Read + Seek
    {
        let flags = FileFlags::read(br, bit_endian)?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;

        let compressed_size = br.read_i32()? as i64;

        let data_offset = if format.contains(Format::LongOffsets) {
            br.read_i64()?
        }
        else {
            util::try_from_to_io_result(br.read_u32()?)?
        };

        let id = if format.contains(Format::IDs) {
            br.read_i32()?
        }
        else {
            -1
        };

        let name = if format.contains(Format::Names1 | Format::Names2) {
            let name_offset = util::try_from_to_io_result(br.read_i32()?)?;
            Some(br.get_shift_jis(name_offset)?)
        }
        else {
            None
        };

        let uncompressed_size = if format.contains(Format::Compression) {
            br.read_i32()? as i64
        }
        else {
            -1
        };

        Ok(Self::new(flags, id, name, compressed_size, uncompressed_size, data_offset))
    }

    /// Reads a `BND4` `BinderFileHeader` from `BinaryReader`
    pub(crate) fn read_bnd4_header<R>(br: &mut BinaryReader<R>, format: &Format, bit_endian: Endian, unicode: bool) -> io::Result<Self>
    where 
        R: Read + Seek
    {
        let flags = FileFlags::read(br, bit_endian)?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;
        br.assert_i32(&[-1])?;

        let compressed_size = br.read_i32()? as i64;

        let uncompressed_size = if format.contains(Format::Compression) {
            br.read_i32()? as i64
        }
        else {
            -1
        };

        let data_offset = if format.contains(Format::LongOffsets) {
            br.read_i64()?
        }
        else {
            util::try_from_to_io_result(br.read_u32()?)?
        };

        let mut id = if format.contains(Format::IDs) {
            br.read_i32()?
        }
        else {
            -1
        };

        let name = if format.contains(Format::Names1 | Format::Names2) {
            let name_offset = br.read_u32()? as u64;
            if unicode {
                Some(br.get_utf16(name_offset)?)
            }
            else {
                Some(br.get_shift_jis(name_offset)?)
            }
        }
        else {
            None
        };

        if *format == Format::Names1 {
            id = br.read_i32()?;
            br.assert_i32(&[0])?;
        };

        Ok(Self::new(flags, id, name, compressed_size, uncompressed_size, data_offset))
    }

    /// Reads a `BinderFile` from `BinderFileHeader` and `BinaryReader`
    pub(crate) fn read_file_data<R>(&self, br: &mut BinaryReader<R>) -> io::Result<BinderFile>
    where
        R: Read + Seek
    {
        let compressed = br.get_u8_vec(self.data_offset as u64, self.compressed_size as u64)?;

        let (bytes, compression) = if self.flags.contains(FileFlags::Compressed) {
            let dcx = DCX::decompress_bytes(compressed)?;
            (dcx.data, dcx.compression)
        }
        else {
            (compressed, CompressionInfo::Zlib)
        };

        Ok(BinderFile::new(self.flags, self.id, self.name.clone(), bytes, compression))
    }

    pub(crate) fn write_bnd3_header<W>(&self, bw: &mut BinaryWriter<W>, format: &Format, bit_endian: Endian, index: i32) -> io::Result<()>
    where 
        W: Write + Seek
    {
        self.flags.write(bw, bit_endian)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;

        bw.reserve_i32(format!("file_{index}_compressed_size"))?;

        if format.contains(Format::LongOffsets) {
            bw.reserve_i64(format!("file_{index}_data_offset"))?;
        }
        else {
            bw.reserve_i32(format!("file_{index}_data_offset"))?;
        }

        if format.contains(Format::IDs) {
            bw.write_i32(self.id)?;
        }

        if format.contains(Format::Names1 | Format::Names2) {
            bw.reserve_i32(format!("file_{index}_name_offset"))?;
        }
        
        if format.contains(Format::Compression) {
            bw.reserve_i32(format!("file_{index}_uncompressed_size"))?;
        }

        Ok(())
    }

    pub(crate) fn write_bnd4_header<W>(&self, bw: &mut BinaryWriter<W>, format: &Format, bit_endian: Endian, index: i32) -> io::Result<()>
    where 
        W: Write + Seek
    {
        self.flags.write(bw, bit_endian)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        bw.write_i32(-1)?;

        bw.reserve_i64(format!("file_{index}_compressed_size"))?;

        if format.contains(Format::Compression) {
            bw.reserve_i64(format!("file_{index}_uncompressed_size"))?;
        }

        if format.contains(Format::LongOffsets) {
            bw.reserve_i64(format!("file_{index}_data_offset"))?;
        }
        else {
            bw.reserve_i32(format!("file_{index}_data_offset"))?;
        }

        if format.contains(Format::IDs) {
            bw.write_i32(self.id)?;
        }

        if format.contains(Format::Names1 | Format::Names2) {
            bw.reserve_i32(format!("file_{index}_name_offset"))?;
        }

        if *format == Format::Names1 {
            bw.write_i32(self.id)?;
            bw.write_i32(0)?;
        }

        Ok(())
    }

    fn write_file_data<W>(&mut self, bw: &mut BinaryWriter<W>, bytes: Vec<u8>) -> io::Result<()>
    where 
        W: Write + Seek
    {
        if bytes.len() > 0 {
            bw.pad_00(0x10)?;
        }

        self.data_offset = util::try_from_to_io_result(bytes.len())?;
        // self.uncompressed_size = bytes.len();

        Ok(())
    }
}