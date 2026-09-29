use chrono::{DateTime, Local, TimeZone};

pub use crate::binder::{
    file::File,
    format::{FileFlags, Format},
};
use crate::{
    ByteIO, FileIO,
    binder::{file::BinderFileHeader, format, hashtable},
    io::{BinaryReader, BinaryWriter, Endian, StreamIO},
    util::{self, DateTimeExt},
};
use std::io::{self, ErrorKind::InvalidData, Read, Seek, Write};

/// A general-purpose file container used since DS2
#[derive(Debug, Clone, PartialEq)]
pub struct Bnd4 {
    /// The files contained within this `Bnd4`
    pub files: Vec<File>,
    /// A timestamp or version number, 8 characters maximum
    pub version: String,
    /// Indicates the format of this `Bnd4`
    pub format: Format,
    pub unk_04: bool,
    pub unk_05: bool,
    /// Endian format of the data
    pub endian: Endian,
    /// Ordering of flag bits
    pub bit_endian: Endian,
    /// Whether to encode filenames as UTF-8 or Shift JIS
    pub unicode: bool,
    /// Indicates presence of filename hash table
    pub extended: u8,
}

impl Default for Bnd4 {
    fn default() -> Self {
        Self {
            files: Default::default(),
            version: Local::now().to_bnd_timestamp(),
            format: Format::IDs | Format::Names1 | Format::Names2 | Format::Compression,
            unk_04: Default::default(),
            unk_05: Default::default(),
            endian: Endian::Little,
            bit_endian: Endian::Little,
            unicode: Default::default(),
            extended: 4,
        }
    }
}

impl Bnd4 {
    /// Initializes `Bnd3` with specifies parameters, and rest of parameters set to most common values
    pub fn new<Tz>(date: DateTime<Tz>, files: Vec<File>) -> Self
    where
        Tz: TimeZone,
    {
        let mut out = Self::default();
        out.version = date.to_bnd_timestamp();
        out.files = files;

        out
    }

    fn read_header<R>(&mut self, br: &mut BinaryReader<R>) -> io::Result<Vec<BinderFileHeader>>
    where
        R: Read + Seek,
    {
        br.assert_ascii(&["BND4"])?;
        self.unk_04 = br.read_bool()?;
        self.unk_05 = br.read_bool()?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;

        br.assert_u8(&[0])?;
        self.endian = match br.read_bool()? {
            true => Endian::Big,
            false => Endian::Little,
        };
        self.bit_endian = match !br.read_bool()? {
            true => Endian::Big,
            false => Endian::Little,
        };
        br.assert_u8(&[0])?;

        br.endian = self.endian;

        let file_count = br.read_i32()?;
        br.assert_i64(&[0x40])?; // Header size
        self.version = br.read_fixed_shift_jis(8)?;
        let file_header_size = br.read_i64()?;
        br.read_i64()?; // Headers end (incl. hash table)

        self.unicode = br.read_bool()?;
        self.format = Format::read(br, self.bit_endian)?;
        self.extended = br.assert_u8(&[0, 1, 4, 0x80])?;
        br.assert_u8(&[0])?;

        br.assert_i32(&[0])?;

        if self.extended == 4 {
            let hash_table_offset = br.read_i64()?;
            let pos = br.position()?;
            br.seek(hash_table_offset as u64)?;
            hashtable::assert(br)?;
            br.seek(pos)?;
        } else {
            br.assert_i64(&[0])?;
        }

        if file_header_size != format::get_bnd4_file_header_size(self.format) {
            return Err(io::Error::new(InvalidData, "Invalid file header size"));
        }

        let mut file_headers: Vec<BinderFileHeader> = Vec::with_capacity(file_count as usize);
        for _ in 0..file_count {
            file_headers.push(BinderFileHeader::read_bnd4_header(
                br,
                self.format,
                self.bit_endian,
                self.unicode,
            )?);
        }

        Ok(file_headers)
    }

    fn write_header<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        file_headers: &mut [BinderFileHeader],
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bw.endian = self.endian;

        bw.write_ascii("BND4", false)?;

        bw.write_bool(self.unk_04)?;
        bw.write_bool(self.unk_05)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;

        bw.write_u8(0)?;
        match self.endian {
            Endian::Big => bw.write_bool(true)?,
            Endian::Little => bw.write_bool(false)?,
        }
        match self.bit_endian {
            Endian::Big => bw.write_bool(false)?,
            Endian::Little => bw.write_bool(true)?,
        }
        bw.write_u8(0)?;

        bw.write_i32(util::convert_num(file_headers.len())?)?;
        bw.write_i64(0x40)?;
        bw.write_fix_str(&self.version, 8, 0)?;
        bw.write_i64(format::get_bnd4_file_header_size(self.format))?;
        bw.reserve_i64("headers-end")?;

        bw.write_bool(self.unicode)?;
        self.format.write(bw, self.bit_endian)?;
        bw.write_u8(self.extended)?;
        bw.write_u8(0)?;

        bw.write_i32(0)?;
        bw.reserve_i64("hash-table-offset")?;

        for (index, header) in file_headers.iter().enumerate() {
            header.write_bnd4_header(
                bw,
                self.format,
                self.bit_endian,
                util::convert_num(index)?,
            )?;
        }

        for (index, header) in file_headers.iter().enumerate() {
            header.write_file_name(bw, self.format, util::convert_num(index)?, self.unicode)?;
        }

        if self.extended == 4 {
            bw.pad_00(0x8)?;
            let pos = bw.position()?;
            bw.fill_i64("hash-table-offset", util::convert_num(pos)?)?;
            hashtable::write(bw, file_headers)?;
        } else {
            bw.fill_i64("hash-table-offset", 0)?;
        }

        let pos = bw.position()?;
        bw.fill_i64("headers-end", util::convert_num(pos)?)?;

        Ok(())
    }
}

impl StreamIO<Bnd4> for Bnd4 {
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<Bnd4>
    where
        R: Read + Seek,
    {
        let mut out = Bnd4::default();

        let file_headers = out.read_header(br)?;
        let mut files: Vec<File> = Vec::with_capacity(file_headers.len());

        for header in file_headers {
            files.push(header.read_file_data(br)?);
        }

        out.files = files;
        Ok(out)
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let mut file_headers: Vec<BinderFileHeader> = Vec::with_capacity(self.files.len());

        for file in &self.files {
            file_headers.push(BinderFileHeader::from_binder_file(file));
        }

        self.write_header(bw, &mut file_headers)?;

        for (index, header) in file_headers.iter_mut().enumerate() {
            header.write_bnd4_file_data(
                bw,
                self.format,
                util::convert_num(index)?,
                &self.files[index].bytes,
            )?;
        }

        Ok(())
    }

    fn is<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek,
    {
        let (mut br_dec, _) = util::get_decompressed_binary_reader(br)?;
        let len = br_dec.length()?;
        Ok(len >= 4 && br_dec.get_ascii_len(0, 4)? == "BND4")
    }
}

impl ByteIO<Bnd4> for Bnd4 {}
impl FileIO<Bnd4> for Bnd4 {}
