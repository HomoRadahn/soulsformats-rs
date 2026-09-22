use crate::{
    binder::{DateTime, file::BinderFileHeader},
    io::{BinaryReader, BinaryWriter, ByteIO, Endian, FileIO, StreamIO},
    util,
};
use std::io::{self, Read, Seek, Write};

pub use crate::binder::{
    file::File,
    format::{FileFlags, Format},
};

/// A general-purpose file container used before DS2
#[derive(Debug, Clone, PartialEq)]
pub struct BND3 {
    /// The files contained within this `BND3`
    pub files: Vec<File>,
    /// A timestamp or version number, 8 characters maximum
    pub version: String,
    /// Indicates the format of the `BND3`
    pub format: Format,
    /// Endian format of the data
    pub endian: Endian,
    /// Ordering of flag bits
    pub bit_endian: Endian,
    pub unk_18: i32,
    /// Whether or not to write the file headers end value or 0
    pub write_file_headers_end: bool,
}

impl BND3 {
    /// Initializes `BND3` with specifies parameters, and rest of parameters set to most common values
    pub fn new(date: DateTime, files: Vec<File>) -> Self {
        let mut out = Self::empty();
        out.version = date.to_bnd_timestamp();
        out.files = files;

        out
    }

    /// Creates an empty `BND3` formatted for DS1
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
            endian: Endian::Little,
            bit_endian: Endian::Little,
            unk_18: 0,
            write_file_headers_end: false,
        }
    }

    fn read_header<R>(&mut self, br: &mut BinaryReader<R>) -> io::Result<Vec<BinderFileHeader>>
    where
        R: Read + Seek,
    {
        br.assert_ascii(&["BND3"])?;
        self.version = br.read_fix_str(8)?;

        self.bit_endian = match br.get_bool(0xE)? {
            true => Endian::Big,
            false => Endian::Little,
        };

        self.format = Format::read(br, self.bit_endian)?;
        self.endian = match br.read_bool()? {
            true => Endian::Big,
            false => Endian::Little,
        };
        match self.bit_endian {
            Endian::Big => br.assert_bool(&[true])?,
            Endian::Little => br.assert_bool(&[false])?,
        };
        br.assert_u8(&[0])?;

        br.endian = match self.endian {
            Endian::Big => Endian::Big,
            Endian::Little if self.format.contains(Format::BigEndian) => Endian::Big,
            Endian::Little => Endian::Little,
        };

        let file_count = br.read_i32()?;
        self.write_file_headers_end = br.read_i32()? > 0;
        self.unk_18 = br.assert_i32(&[0, i32::MIN])?;
        br.assert_i32(&[0])?;

        let mut file_headers: Vec<BinderFileHeader> = Vec::with_capacity(file_count as usize);
        for _ in 0..file_count {
            file_headers.push(BinderFileHeader::read_bnd3_header(
                br,
                self.format,
                self.bit_endian,
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
        bw.endian = match self.endian {
            Endian::Big => Endian::Big,
            Endian::Little if self.format.contains(Format::BigEndian) => Endian::Big,
            Endian::Little => Endian::Little,
        };

        bw.write_ascii("BND3", false)?;
        bw.write_fix_str(&self.version, 8, 0)?;
        self.format.write(bw, self.bit_endian)?;
        bw.write_bool(match self.endian {
            Endian::Big => true,
            Endian::Little => false,
        })?;
        bw.write_bool(match self.bit_endian {
            Endian::Big => true,
            Endian::Little => false,
        })?;
        bw.write_u8(0)?;

        bw.write_i32(util::convert_num(file_headers.len())?)?;
        bw.reserve_i32("file-headers-end")?;
        bw.write_i32(self.unk_18)?;
        bw.write_i32(0)?;

        for (index, header) in file_headers.iter().enumerate() {
            header.write_bnd3_header(
                bw,
                self.format,
                self.bit_endian,
                util::convert_num(index)?,
            )?;
        }

        for (index, header) in file_headers.iter().enumerate() {
            header.write_file_name(bw, self.format, util::convert_num(index)?, false)?;
        }

        if self.write_file_headers_end {
            let pos = util::convert_num::<u64, i32>(bw.position()?)?;
            bw.fill_i32("file-headers-end", pos)?;
        } else {
            bw.fill_i32("file-headers-end", 0)?;
        }

        Ok(())
    }
}

impl StreamIO<BND3> for BND3 {
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<BND3>
    where
        R: Read + Seek,
    {
        let mut bnd = BND3::empty();

        let file_headers = bnd.read_header(br)?;
        let mut files: Vec<File> = Vec::with_capacity(file_headers.len());

        for header in file_headers {
            files.push(header.read_file_data(br)?);
        }

        bnd.files = files;

        Ok(bnd)
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
            header.write_bnd3_file_data(
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
        Ok(len >= 4 && br_dec.get_ascii_len(0, 4)? == "BND3")
    }
}

impl ByteIO<BND3> for BND3 {}
impl FileIO<BND3> for BND3 {}
