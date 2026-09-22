use std::{
    fs,
    io::{self, ErrorKind::InvalidData, Read, Seek, Write},
};

use crate::{
    binder::{DateTime, file::BinderFileHeader, format, hashtable},
    io::{BinaryReader, BinaryWriter, Endian},
    util,
};

pub use crate::binder::{
    file::File,
    format::{FileFlags, Format},
};

/// A general-purpose split header and data binder, used in newer FromSoftware games.
/// Header `.bhd`; data `.bdt`.
#[derive(Debug, Clone, PartialEq)]
pub struct BXF4 {
    /// Files contained within this `BXF4`
    pub files: Vec<File>,
    /// A timestamp or version number, 8 characters maximum
    pub version: String,
    /// Format of this `BXF4`
    pub format: Format,
    pub unk_04: bool,
    pub unk_05: bool,
    /// Endianness of the data
    pub endian: Endian,
    /// Ordering of flag bits
    pub bit_endian: Endian,
    /// Whether to write strings in UTF-16
    pub unicode: bool,
    /// Indicates the presence of a filename hash table
    pub extended: u8,
}

impl BXF4 {
    /// Initializes `BXF3` with specifies parameters, and rest of parameters set to most common values
    pub fn new(date: DateTime, files: Vec<File>) -> Self {
        let mut out = Self::empty();
        out.version = date.to_bnd_timestamp();
        out.files = files;

        out
    }

    /// Creates and empty `BXF4` formatted for DS3
    pub fn empty() -> Self {
        Self {
            files: Vec::new(),
            version: DateTime {
                year: 2026,
                month: 1,
                day: 1,
                hour: 20,
                minute: 0,
            }
            .to_bnd_timestamp(),
            format: Format::IDs | Format::Names1 | Format::Names2 | Format::Compression,
            unk_04: false,
            unk_05: false,
            endian: Endian::Little,
            bit_endian: Endian::Little,
            unicode: true,
            extended: 4,
        }
    }

    fn read_bdf_header<R>(bdt: &mut BinaryReader<R>) -> io::Result<()>
    where
        R: Read + Seek,
    {
        bdt.assert_ascii(&["BDF4"])?;
        bdt.read_bool()?;
        bdt.read_bool()?;
        bdt.assert_u8(&[0])?;
        bdt.assert_u8(&[0])?;
        bdt.assert_u8(&[0])?;
        bdt.endian = if bdt.read_bool()? {
            Endian::Big
        } else {
            Endian::Little
        };
        bdt.read_bool()?;
        bdt.assert_u8(&[0])?;
        bdt.assert_i32(&[0])?;
        bdt.assert_i64(&[0x30, 0x40])?;
        bdt.read_fix_str(8)?;
        bdt.assert_i64(&[0])?;
        bdt.assert_i64(&[0])?;
        Ok(())
    }

    fn read_bhf_header<R>(&mut self, bhd: &mut BinaryReader<R>) -> io::Result<Vec<BinderFileHeader>>
    where
        R: Read + Seek,
    {
        bhd.assert_ascii(&["BHF4"])?;

        self.unk_04 = bhd.read_bool()?;
        self.unk_05 = bhd.read_bool()?;
        bhd.assert_u8(&[0])?;
        bhd.assert_u8(&[0])?;

        bhd.assert_u8(&[0])?;
        self.endian = if bhd.read_bool()? {
            Endian::Big
        } else {
            Endian::Little
        };
        self.bit_endian = if !bhd.read_bool()? {
            Endian::Big
        } else {
            Endian::Little
        };
        bhd.assert_u8(&[0])?;

        bhd.endian = self.endian;

        let file_count = bhd.read_i32()?;
        bhd.assert_i64(&[0x40])?;
        self.version = bhd.read_fix_str(8)?;
        let file_header_size = bhd.read_i64()?;
        bhd.assert_i64(&[0])?;

        self.unicode = bhd.read_bool()?;
        self.format = Format::read(bhd, self.bit_endian)?;
        self.extended = bhd.assert_u8(&[0, 4])?;
        if self.extended != 0 && self.extended != 4 {
            return Err(io::Error::new(InvalidData, "invalid BXF4 extended value"));
        }
        bhd.assert_u8(&[0])?;
        bhd.assert_i32(&[0])?;

        if self.extended == 4 {
            let hash_table_offset = bhd.read_i64()?;
            let position = bhd.position()?;
            bhd.seek(hash_table_offset as u64)?;
            hashtable::assert(bhd)?;
            bhd.seek(position)?;
        } else {
            bhd.assert_i64(&[0])?;
        }

        if file_header_size != format::get_bnd4_file_header_size(self.format) {
            return Err(io::Error::new(InvalidData, "Invalid file header size"));
        }
        let mut file_headers = Vec::with_capacity(util::convert_num(file_count)?);
        for _ in 0..file_count {
            file_headers.push(BinderFileHeader::read_bnd4_header(
                bhd,
                self.format,
                self.bit_endian,
                self.unicode,
            )?);
        }
        Ok(file_headers)
    }

    /// Read `BXF4` from two given `BinaryReaders`. Only accepts decompressed data
    pub fn read<RH, RD>(bhd: &mut BinaryReader<RH>, bdt: &mut BinaryReader<RD>) -> io::Result<Self>
    where
        RH: Read + Seek,
        RD: Read + Seek,
    {
        let mut bxf = Self::empty();
        Self::read_bdf_header(bdt)?;
        let file_headers = bxf.read_bhf_header(bhd)?;
        for header in file_headers {
            bxf.files.push(header.read_file_data(bdt)?);
        }
        Ok(bxf)
    }

    /// Reads `BXF3` from two files, decompressing as necessary
    pub fn from_files(
        bhd_path: impl Into<String>,
        bdt_path: impl Into<String>,
    ) -> io::Result<Self> {
        let mut bhd = BinaryReader::from_file(bhd_path.into(), Endian::Little, false)?;
        let mut bdt = BinaryReader::from_file(bdt_path.into(), Endian::Little, false)?;
        Self::read(&mut bhd, &mut bdt)
    }

    /// Reads `BXF3` from a `bhd` file and `bdt` bytes, decompressing as necessary
    pub fn from_bhd_file_bdt_bytes(
        bhd_path: impl Into<String>,
        bdt_bytes: Vec<u8>,
    ) -> io::Result<Self> {
        let mut bhd = BinaryReader::from_file(bhd_path.into(), Endian::Little, false)?;
        let mut bdt = BinaryReader::from_bytes(bdt_bytes, Endian::Little, false);
        Self::read(&mut bhd, &mut bdt)
    }

    /// Reads `BXF3` from `bhd` bytes and `bdt` file, decompressing as necessary
    pub fn from_bhd_bytes_bdt_file(
        bhd_bytes: Vec<u8>,
        bdt_path: impl Into<String>,
    ) -> io::Result<Self> {
        let mut bhd = BinaryReader::from_bytes(bhd_bytes, Endian::Little, false);
        let mut bdt = BinaryReader::from_file(bdt_path.into(), Endian::Little, false)?;

        Self::read(&mut bhd, &mut bdt)
    }

    /// Reads `BXF3` from two `Vec<u8>`, decompressing as necessary
    pub fn from_bytes(bhd_bytes: Vec<u8>, bdt_bytes: Vec<u8>) -> io::Result<Self> {
        let mut bhd = BinaryReader::from_bytes(bhd_bytes, Endian::Little, false);
        let mut bdt = BinaryReader::from_bytes(bdt_bytes, Endian::Little, false);

        Self::read(&mut bhd, &mut bdt)
    }

    /// Writes `BXF4` to two `BinaryWriters`. Doesn't compress data
    pub fn write<WH, WD>(
        &self,
        bhd: &mut BinaryWriter<WH>,
        bdt: &mut BinaryWriter<WD>,
    ) -> io::Result<()>
    where
        WH: Write + Seek,
        WD: Write + Seek,
    {
        let mut file_headers: Vec<BinderFileHeader> = self
            .files
            .iter()
            .map(BinderFileHeader::from_binder_file)
            .collect();
        self.write_bdf_header(bdt)?;
        self.write_bhf_header(bhd, &mut file_headers)?;
        for (index, (header, file)) in file_headers.iter_mut().zip(&self.files).enumerate() {
            header.write_bxf4_file_data(
                bhd,
                bdt,
                self.format,
                util::convert_num(index)?,
                &file.bytes,
            )?;
        }
        Ok(())
    }

    fn write_bdf_header<W>(&self, bdt: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bdt.endian = self.endian;
        bdt.write_ascii("BDF4", false)?;
        bdt.write_bool(self.unk_04)?;
        bdt.write_bool(self.unk_05)?;
        bdt.write_u8(0)?;
        bdt.write_u8(0)?;
        bdt.write_u8(0)?;
        bdt.write_bool(self.endian == Endian::Big)?;
        bdt.write_bool(self.bit_endian == Endian::Little)?;
        bdt.write_u8(0)?;
        bdt.write_i32(0)?;
        bdt.write_i64(0x30)?;
        bdt.write_fix_str(&self.version, 8, 0)?;
        bdt.write_i64(0)?;
        bdt.write_i64(0)
    }

    fn write_bhf_header<W>(
        &self,
        bhd: &mut BinaryWriter<W>,
        file_headers: &mut [BinderFileHeader],
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bhd.endian = self.endian;

        bhd.write_ascii("BHF4", false)?;

        bhd.write_bool(self.unk_04)?;
        bhd.write_bool(self.unk_05)?;
        bhd.write_u8(0)?;
        bhd.write_u8(0)?;

        bhd.write_u8(0)?;
        bhd.write_bool(self.endian == Endian::Big)?;
        bhd.write_bool(self.bit_endian == Endian::Little)?;
        bhd.write_u8(0)?;

        bhd.write_i32(util::convert_num(file_headers.len())?)?;
        bhd.write_i64(0x40)?;
        bhd.write_fix_str(&self.version, 8, 0)?;
        bhd.write_i64(format::get_bnd4_file_header_size(self.format))?;
        bhd.write_i64(0)?;

        bhd.write_bool(self.unicode)?;
        self.format.write(bhd, self.bit_endian)?;
        bhd.write_u8(self.extended)?;
        bhd.write_u8(0)?;

        bhd.write_i32(0)?;
        bhd.reserve_i64("hash-table-offset")?;

        for (index, header) in file_headers.iter().enumerate() {
            header.write_bnd4_header(
                bhd,
                self.format,
                self.bit_endian,
                util::convert_num(index)?,
            )?;
        }
        for (index, header) in file_headers.iter().enumerate() {
            header.write_file_name(bhd, self.format, util::convert_num(index)?, self.unicode)?;
        }
        if self.extended == 4 {
            bhd.pad_00(0x8)?;
            let position = bhd.position()?;
            hashtable::write(bhd, file_headers)?;
            bhd.fill_i64("hash-table-offset", util::convert_num(position)?)?;
        } else {
            bhd.fill_i64("hash-table-offset", 0)?;
        }
        Ok(())
    }

    fn preprocess_to_dcx(&self) -> io::Result<(Vec<u8>, Vec<u8>)> {
        let mut bhd = BinaryWriter::to_bytes(Endian::Little, false);
        let mut bdt = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bhd, &mut bdt)?;

        Ok((bhd.close_bytes()?, bdt.close_bytes()?))
    }

    /// Writes `BXF4` to two files, compressing as necessary
    pub fn to_files(
        &self,
        bhd_path: impl Into<String>,
        bdt_path: impl Into<String>,
    ) -> io::Result<()> {
        let (bhd, bdt) = self.preprocess_to_dcx()?;

        fs::write(bhd_path.into(), bhd)?;
        fs::write(bdt_path.into(), bdt)?;

        Ok(())
    }

    /// Writes `BXF4` - `bhd` to file and `bdt` to bytes
    pub fn to_bhd_file_bdt_bytes(&self, bhd_path: impl Into<String>) -> io::Result<Vec<u8>> {
        let (bhd, bdt) = self.preprocess_to_dcx()?;
        fs::write(bhd_path.into(), bhd)?;

        Ok(bdt)
    }

    /// Writes `BXF4` - `bhd` to file and `bdt` to bytes
    pub fn to_bhd_bytes_bdt_file(&self, bdt_path: impl Into<String>) -> io::Result<Vec<u8>> {
        let (bhd, bdt) = self.preprocess_to_dcx()?;
        fs::write(bdt_path.into(), bdt)?;

        Ok(bhd)
    }

    /// Writes `BXF4` to two `Vec<u8>`, compressing as necessary
    pub fn to_bytes(&self) -> io::Result<(Vec<u8>, Vec<u8>)> {
        let (bhd, bdt) = self.preprocess_to_dcx()?;

        Ok((bhd, bdt))
    }

    /// Checks if the provided `BinaryReader` appears to contain a valid header
    pub fn is_header<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek,
    {
        let (mut decompressed, _) = util::get_decompressed_binary_reader(br)?;
        Ok(decompressed.remaining()? >= 4 && decompressed.get_ascii_len(0, 4)? == "BHF4")
    }

    /// Checks if the given bytes appear to contain a valid header
    pub fn is_header_bytes(data: Vec<u8>) -> io::Result<bool> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        Self::is_header(&mut br)
    }

    /// Checks if the given bytes appear to contain a valid header
    pub fn is_header_file(path: impl Into<String>) -> io::Result<bool> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Little, false)?;
        Self::is_header(&mut br)
    }

    /// Checks if the provided `BinaryReader` appears to contain valid data
    pub fn is_data<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek,
    {
        let (mut decompressed, _) = util::get_decompressed_binary_reader(br)?;
        Ok(decompressed.remaining()? >= 4 && decompressed.get_ascii_len(0, 4)? == "BDF4")
    }

    /// Checks if the given bytes appear to contain valid data
    pub fn is_data_bytes(data: Vec<u8>) -> io::Result<bool> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        Self::is_data(&mut br)
    }

    /// Checks if the given bytes appear to contain valid data
    pub fn is_data_file(path: impl Into<String>) -> io::Result<bool> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Little, false)?;
        Self::is_data(&mut br)
    }
}
