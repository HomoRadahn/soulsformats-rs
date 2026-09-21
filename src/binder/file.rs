use core::fmt;
use std::io::{self, Read, Seek, Write};

use crate::{
    DCX,
    binder::format::{FileFlags, Format},
    dcx::compression_info::CompressionInfo,
    io::{BinaryReader, BinaryWriter, Endian},
    util,
};

/// A generic file in `BND3`, `BND4`, `BXF3`, `BXF4` containers
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    /// Flags of this `File`
    pub flags: FileFlags,
    /// ID of this `File`
    pub id: i32,
    /// Name of this `File`
    pub name: String,
    /// Bytes contained in this `File`
    pub bytes: Vec<u8>,
    /// Compression of this `File`, different from DCX compression
    pub compression: CompressionInfo,
}

impl File {
    /// Creates a new `File`, with zlib compression
    pub fn new(
        flags: FileFlags,
        id: i32,
        name: impl Into<String>,
        bytes: Vec<u8>,
        compression: CompressionInfo,
    ) -> Self {
        Self {
            flags,
            id,
            name: name.into(),
            bytes,
            compression,
        }
    }
}

pub(crate) struct BinderFileHeader {
    pub flags: FileFlags,
    pub id: i32,
    pub name: String,
    pub compression: CompressionInfo,
    pub compressed_size: i64,
    pub uncompressed_size: i64,
    pub data_offset: i64,
}

impl BinderFileHeader {
    pub(crate) fn new(
        flags: FileFlags,
        id: i32,
        name: String,
        compressed_size: i64,
        uncompressed_size: i64,
        data_offset: i64,
    ) -> Self {
        Self {
            flags,
            id,
            name,
            compression: CompressionInfo::Zlib,
            compressed_size,
            uncompressed_size,
            data_offset,
        }
    }

    pub(crate) fn from_binder_file(file: &File) -> Self {
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

    pub(crate) fn read_bnd3_header<R>(
        br: &mut BinaryReader<R>,
        format: Format,
        bit_endian: Endian,
    ) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let flags = FileFlags::read(br, bit_endian)?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;

        let compressed_size = br.read_i32()? as i64;

        let data_offset = if format.contains(Format::LongOffsets) {
            br.read_i64()?
        } else {
            util::convert_num(br.read_u32()?)?
        };

        let id = if format.contains(Format::IDs) {
            br.read_i32()?
        } else {
            -1
        };

        let name = if format.contains(Format::Names1 | Format::Names2) {
            let name_offset = util::convert_num(br.read_i32()?)?;
            br.get_shift_jis(name_offset)?
        } else {
            "".to_string()
        };

        let uncompressed_size = if format.contains(Format::Compression) {
            br.read_i32()? as i64
        } else {
            -1
        };

        Ok(Self::new(
            flags,
            id,
            name,
            compressed_size,
            uncompressed_size,
            data_offset,
        ))
    }

    pub(crate) fn read_bnd4_header<R>(
        br: &mut BinaryReader<R>,
        format: Format,
        bit_endian: Endian,
        unicode: bool,
    ) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let flags = FileFlags::read(br, bit_endian)?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;
        br.assert_i32(&[-1])?;

        let compressed_size = br.read_i64()?;

        let uncompressed_size = if format.contains(Format::Compression) {
            br.read_i64()?
        } else {
            -1
        };

        let data_offset = if format.contains(Format::LongOffsets) {
            br.read_i64()?
        } else {
            util::convert_num(br.read_u32()?)?
        };

        let mut id = if format.contains(Format::IDs) {
            br.read_i32()?
        } else {
            -1
        };

        let name = if format.contains(Format::Names1 | Format::Names2) {
            let name_offset = br.read_u32()? as u64;
            if unicode {
                br.get_utf16(name_offset)?
            } else {
                br.get_shift_jis(name_offset)?
            }
        } else {
            "".to_string()
        };

        if format == Format::Names1 {
            id = br.read_i32()?;
            br.assert_i32(&[0])?;
        };

        Ok(Self::new(
            flags,
            id,
            name,
            compressed_size,
            uncompressed_size,
            data_offset,
        ))
    }

    pub(crate) fn read_file_data<R>(&self, br: &mut BinaryReader<R>) -> io::Result<File>
    where
        R: Read + Seek,
    {
        let compressed = br.get_vec_u8(self.data_offset as u64, self.compressed_size as u64)?;

        let (bytes, compression) = if self.flags.contains(FileFlags::Compressed) {
            let dcx = DCX::from_bytes(compressed)?;
            (dcx.data, dcx.compression)
        } else {
            (compressed, CompressionInfo::Zlib)
        };

        Ok(File::new(
            self.flags,
            self.id,
            self.name.clone(),
            bytes,
            compression,
        ))
    }

    pub(crate) fn write_bnd3_header<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        format: Format,
        bit_endian: Endian,
        index: i32,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        self.flags.write(bw, bit_endian)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;

        bw.reserve_i32(format!("file_{index}_compressed_size"))?;

        if format.contains(Format::LongOffsets) {
            bw.reserve_i64(format!("file_{index}_data_offset"))?;
        } else {
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

    pub(crate) fn write_bnd4_header<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        format: Format,
        bit_endian: Endian,
        index: i32,
    ) -> io::Result<()>
    where
        W: Write + Seek,
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
        } else {
            bw.reserve_i32(format!("file_{index}_data_offset"))?;
        }

        if format.contains(Format::IDs) {
            bw.write_i32(self.id)?;
        }

        if format.contains(Format::Names1 | Format::Names2) {
            bw.reserve_i32(format!("file_{index}_name_offset"))?;
        }

        if format == Format::Names1 {
            bw.write_i32(self.id)?;
            bw.write_i32(0)?;
        }

        Ok(())
    }

    fn write_file_data<W>(&mut self, bw: &mut BinaryWriter<W>, bytes: &[u8]) -> io::Result<()>
    where
        W: Write + Seek,
    {
        if !bytes.is_empty() {
            bw.pad_00(0x10)?;
        }

        self.data_offset = util::convert_num(bw.position()?)?;
        self.uncompressed_size = util::convert_num(bytes.len())?;

        if self.flags.contains(FileFlags::Compressed) {
            let compressed = DCX::new(bytes.to_vec(), self.compression).to_bytes()?;
            self.compressed_size = util::convert_num(compressed.len())?;
            bw.write_vec_u8(compressed)?;
        } else {
            self.compressed_size = util::convert_num(bytes.len())?;
            bw.write_bytes(bytes)?;
        }

        Ok(())
    }

    pub(crate) fn write_bnd3_file_data<W>(
        &mut self,
        bw: &mut BinaryWriter<W>,
        format: Format,
        index: i32,
        bytes: &[u8],
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        self.write_file_data(bw, bytes)?;

        bw.fill_i32(
            format!("file_{index}_compressed_size"),
            util::convert_num(self.compressed_size)?,
        )?;

        if format.contains(Format::Compression) {
            bw.fill_i32(
                format!("file_{index}_uncompressed_size"),
                util::convert_num(self.uncompressed_size)?,
            )?;
        }

        if format.contains(Format::LongOffsets) {
            bw.fill_i64(format!("file_{index}_data_offset"), self.data_offset)?;
        } else {
            bw.fill_u32(
                format!("file_{index}_data_offset"),
                util::convert_num(self.data_offset)?,
            )?;
        }

        Ok(())
    }

    pub(crate) fn write_bxf3_file_data<WH, WD>(
        &mut self,
        bw_header: &mut BinaryWriter<WH>,
        bw_data: &mut BinaryWriter<WD>,
        format: Format,
        index: i32,
        bytes: &[u8],
    ) -> io::Result<()>
    where
        WH: Write + Seek,
        WD: Write + Seek,
    {
        self.write_file_data(bw_data, bytes)?;

        bw_header.fill_i32(
            format!("file_{index}_compressed_size"),
            util::convert_num(self.compressed_size)?,
        )?;

        if format.contains(Format::Compression) {
            bw_header.fill_i32(
                format!("file_{index}_uncompressed_size"),
                util::convert_num(self.uncompressed_size)?,
            )?;
        }

        if format.contains(Format::LongOffsets) {
            bw_header.fill_i64(format!("file_{index}_data_offset"), self.data_offset)?;
        } else {
            bw_header.fill_u32(
                format!("file_{index}_data_offset"),
                util::convert_num(self.data_offset)?,
            )?;
        }

        Ok(())
    }

    pub(crate) fn write_bnd4_file_data<W>(
        &mut self,
        bw: &mut BinaryWriter<W>,
        format: Format,
        index: i32,
        bytes: &[u8],
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        self.write_file_data(bw, bytes)?;

        bw.fill_i64(
            format!("file_{index}_compressed_size"),
            self.compressed_size,
        )?;

        if format.contains(Format::Compression) {
            bw.fill_i64(
                format!("file_{index}_uncompressed_size"),
                self.uncompressed_size,
            )?;
        }

        if format.contains(Format::LongOffsets) {
            bw.fill_i64(format!("file_{index}_data_offset"), self.data_offset)?;
        } else {
            bw.fill_u32(
                format!("file_{index}_data_offset"),
                util::convert_num(self.data_offset)?,
            )?;
        }

        Ok(())
    }

    pub(crate) fn write_bxf4_file_data<WH, WD>(
        &mut self,
        bw_header: &mut BinaryWriter<WH>,
        bw_data: &mut BinaryWriter<WD>,
        format: Format,
        index: i32,
        bytes: &[u8],
    ) -> io::Result<()>
    where
        WH: Write + Seek,
        WD: Write + Seek,
    {
        self.write_file_data(bw_data, bytes)?;

        bw_header.fill_i64(
            format!("file_{index}_compressed_size"),
            self.compressed_size,
        )?;

        if format.contains(Format::Compression) {
            bw_header.fill_i64(
                format!("file_{index}_uncompressed_size"),
                self.uncompressed_size,
            )?;
        }

        if format.contains(Format::LongOffsets) {
            bw_header.fill_i64(format!("file_{index}_data_offset"), self.data_offset)?;
        } else {
            bw_header.fill_u32(
                format!("file_{index}_data_offset"),
                util::convert_num(self.data_offset)?,
            )?;
        }

        Ok(())
    }

    pub(crate) fn write_file_name<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        format: Format,
        index: i32,
        unicode: bool,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        // Calling bare unwrap(), since name can only be none if format doesn't have names
        if format.contains(Format::Names1 | Format::Names2) {
            let pos = bw.position()?;
            bw.fill_i32(format!("file_{index}_name_offset"), util::convert_num(pos)?)?;
            match unicode {
                true => bw.write_utf16(&self.name, true)?,
                false => bw.write_shift_jis(&self.name, true)?,
            };
        }

        Ok(())
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ID: {} | Name: {} | Length: {} | Flags: {}",
            self.id,
            self.name,
            self.bytes.len(),
            self.flags.bits()
        )
    }
}
