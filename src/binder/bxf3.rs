use std::{
    fs,
    io::{self, Read, Seek, Write},
};

use crate::{
    binder::{
        BinderFile, DateTime,
        file::BinderFileHeader,
        format::{self, Format},
    },
    io::{BinaryReader, BinaryWriter, Endian},
    util,
};

/// A general-purpose split header and data binder, used in older FromSoftware games.<br>
/// Header `.bhd`<br>
/// Data `.bdt`
pub struct BXF3 {
    /// Files contained in this `BXF3`
    pub files: Vec<BinderFile>,
    /// A timestamp or version number, 8 characters maximum
    pub version: String,
    /// Formats of this `BXF3`
    pub format: Format,
    /// Endian format of the data
    pub endian: Endian,
    /// Ordering of flag bits
    pub bit_endian: Endian,
}

impl BXF3 {
    /// Initializes `BXF3` with specifies parameters, and rest of parameters set to most common values
    pub fn new(date: DateTime, files: Vec<BinderFile>) -> Self {
        let mut out = Self::empty();
        out.version = date.to_bnd_timestamp();
        out.files = files;

        out
    }
    /// Creates an empty `BXF3` formatted for DS1.
    pub fn empty() -> Self {
        Self {
            files: Vec::new(),
            version: format::DateTime {
                year: 2026,
                month: 1,
                day: 1,
                hour: 20,
                minute: 0,
            }
            .to_bnd_timestamp(),
            format: Format::IDs | Format::Names1 | Format::Names2 | Format::Compression,
            endian: Endian::Little,
            bit_endian: Endian::Little,
        }
    }

    fn read_bdf_header<R>(bdt: &mut BinaryReader<R>) -> io::Result<()>
    where
        R: Read + Seek,
    {
        bdt.assert_ascii(&["BDF3"])?;
        bdt.read_fix_str(8)?;
        bdt.assert_i32(&[0])?;

        Ok(())
    }

    fn read_bhf_header<R>(&mut self, bhd: &mut BinaryReader<R>) -> io::Result<Vec<BinderFileHeader>>
    where
        R: Read + Seek,
    {
        bhd.assert_ascii(&["BHF3"])?;
        self.version = bhd.read_fix_str(8)?;

        self.bit_endian = match bhd.get_bool(0xE)? {
            true => Endian::Big,
            false => Endian::Little,
        };

        self.format = Format::read(bhd, self.bit_endian)?;

        self.endian = match bhd.read_bool()? {
            true => Endian::Big,
            false => Endian::Little,
        };
        bhd.assert_bool(match self.bit_endian {
            Endian::Big => &[true],
            Endian::Little => &[false],
        })?;
        bhd.assert_u8(&[0])?;

        bhd.endian = match self.endian {
            Endian::Big => Endian::Big,
            Endian::Little if self.format.contains(Format::BigEndian) => Endian::Big,
            Endian::Little => Endian::Little,
        };

        let file_count = bhd.read_i32()?;
        bhd.assert_i32(&[0])?;
        bhd.assert_i32(&[0])?;
        bhd.assert_i32(&[0])?;

        let mut file_headers: Vec<BinderFileHeader> = Vec::with_capacity(file_count as usize);

        for _ in 0..file_count {
            file_headers.push(BinderFileHeader::read_bnd3_header(
                bhd,
                self.format,
                self.bit_endian,
            )?);
        }

        Ok(file_headers)
    }

    /// Reads `BXF3` from two `BinaryReaders`. Only accepts decompressed data
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
}

impl BXF3 {
    /// Writes `BXF3` to two separate `BinaryWriters`. Doesn't compress data
    pub fn write<WH, WD>(
        &self,
        bhd: &mut BinaryWriter<WH>,
        bdt: &mut BinaryWriter<WD>,
    ) -> io::Result<()>
    where
        WH: Write + Seek,
        WD: Write + Seek,
    {
        let mut file_headers = Vec::with_capacity(self.files.len());

        for file in &self.files {
            file_headers.push(BinderFileHeader::from_binder_file(&file));
        }

        self.write_bdf_header(bdt)?;
        self.write_bhf_header(bhd, &file_headers)?;

        for i in 0..self.files.len() {
            file_headers[i].write_bxf3_file_data(
                bhd,
                bdt,
                self.format,
                util::try_from_to_io_result(i)?,
                &self.files[i].bytes,
            )?;
        }

        Ok(())
    }

    fn write_bdf_header<W>(&self, bdt: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bdt.write_ascii("BDF3", false)?;
        bdt.write_fix_str(&self.version, 8, 0)?;
        bdt.write_i32(0)?;

        Ok(())
    }

    fn write_bhf_header<W>(
        &self,
        bhd: &mut BinaryWriter<W>,
        file_headers: &[BinderFileHeader],
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bhd.endian = match self.endian {
            Endian::Big => Endian::Big,
            Endian::Little if self.format.contains(Format::BigEndian) => Endian::Big,
            Endian::Little => Endian::Little,
        };

        bhd.write_ascii("BHF3", false)?;
        bhd.write_fix_str(&self.version, 8, 0)?;

        self.format.write(bhd, self.bit_endian)?;
        bhd.write_u8(0)?;
        bhd.write_u8(0)?;
        bhd.write_u8(0)?;

        bhd.write_i32(util::try_from_to_io_result(file_headers.len())?)?;
        bhd.write_i32(0)?;
        bhd.write_i32(0)?;
        bhd.write_i32(0)?;

        for i in 0..file_headers.len() {
            file_headers[i].write_bnd3_header(
                bhd,
                self.format,
                self.bit_endian,
                util::try_from_to_io_result(i)?,
            )?;
        }

        for i in 0..file_headers.len() {
            file_headers[i].write_file_name(
                bhd,
                self.format,
                util::try_from_to_io_result(i)?,
                false,
            )?;
        }

        Ok(())
    }

    fn preprocess_to_bytes(&self) -> io::Result<(Vec<u8>, Vec<u8>)> {
        let mut bhd = BinaryWriter::to_bytes(Endian::Little, false);
        let mut bdt = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bhd, &mut bdt)?;
        Ok((bhd.close_bytes()?, bdt.close_bytes()?))
    }

    /// Writes `BXF3` to two files, compressing as necessary
    pub fn to_files(
        &self,
        bhd_path: impl Into<String>,
        bdt_path: impl Into<String>,
    ) -> io::Result<()> {
        let (bhd, bdt) = self.preprocess_to_bytes()?;

        fs::write(bhd_path.into(), bhd)?;
        fs::write(bdt_path.into(), bdt)?;

        Ok(())
    }

    /// Writes `BXF3` - `bhd` to file and `bdt` to bytes
    pub fn to_bhd_file_bdt_bytes(&self, bhd_path: impl Into<String>) -> io::Result<Vec<u8>> {
        let (bhd, bdt) = self.preprocess_to_bytes()?;
        fs::write(bhd_path.into(), bhd)?;

        Ok(bdt)
    }

    /// Writes `BXF3` - `bhd` to file and `bdt` to bytes
    pub fn to_bhd_bytes_bdt_file(&self, bdt_path: impl Into<String>) -> io::Result<Vec<u8>> {
        let (bhd, bdt) = self.preprocess_to_bytes()?;
        fs::write(bdt_path.into(), bdt)?;

        Ok(bhd)
    }

    /// Writes `BXF3` to two `Vec<u8>`, compressing as necessary
    pub fn to_bytes(&self) -> io::Result<(Vec<u8>, Vec<u8>)> {
        let (bhd, bdt) = self.preprocess_to_bytes()?;

        Ok((bhd, bdt))
    }
}

impl BXF3 {
    /// Checks if the provided `BinaryReader` appears to contain a valid header
    pub fn is_header<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek,
    {
        let (mut decompressed, _) = util::get_decompressed_binary_reader(br)?;
        let remaining = decompressed.remaining()?;
        let pos = decompressed.position()?;
        Ok(remaining >= 4 && decompressed.get_ascii_len(pos, 4)? == "BHF3")
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
        let remaining = decompressed.remaining()?;
        let pos = decompressed.position()?;
        Ok(remaining >= 4 && decompressed.get_ascii_len(pos, 4)? == "BDF3")
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
