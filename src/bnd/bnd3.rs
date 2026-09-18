use crate::{
    bnd::{
        BinderFile,
        binder::{self, Format},
        file::BinderFileHeader,
    },
    dcx::compression_info::CompressionInfo,
    io::{BinaryReader, BinaryWriter, ByteIO, Endian, FileIO, StreamIO},
    util,
};
use std::io::{self, Read, Seek, Write};

/// A general-purpose file container used before DS2
pub struct BND3 {
    /// The files contained within this BND3
    pub files: Vec<BinderFile>,
    /// A timestamp or version number, 8 characters maximum
    pub version: String,
    /// Indicates the format of the BND3
    pub format: Format,
    /// Endian format to write in
    pub endian: Endian,
    /// Ordering of flag bits
    pub bit_endian: Endian,
    pub unk18: i32,
    /// Whether or not to write the file headers end value or 0
    pub write_file_headers_end: bool,
    /// DCX Compression info
    pub compression: CompressionInfo,
}

impl BND3 {
    /// Creates an empty BND3 formatted for DS1
    pub fn new(compression: CompressionInfo) -> Self {
        Self {
            files: Vec::new(),
            version: binder::DateTime {
                year: 2026,
                month: 1,
                day: 1,
                hour: 20,
                minute: 0,
            }
            .to_bnd_timestamp(),
            format: Format::IDs | Format::Names1 | Format::Names2 | Format::Compression,
            endian: Endian::Big,
            bit_endian: Endian::Big,
            unk18: 0,
            write_file_headers_end: false,
            compression: compression,
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

        match self.endian {
            Endian::Big => br.set_endian(Endian::Big),
            Endian::Little if self.format.contains(Format::BigEndian) => br.set_endian(Endian::Big),
            Endian::Little => br.set_endian(Endian::Little),
        };

        let file_count = br.read_i32()?;
        self.write_file_headers_end = br.read_i32()? > 0;
        self.unk18 = br.assert_i32(&[0, i32::MIN])?;
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
}

impl StreamIO<BND3> for BND3 {
    fn get_compression(&self) -> CompressionInfo {
        self.compression
    }

    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<BND3>
    where
        R: Read + Seek,
    {
        let (mut br_dec, compression) = util::get_decompressed_binary_reader(br)?;
        let mut bnd = BND3::new(compression);

        let file_headers = bnd.read_header(&mut br_dec)?;
        let mut files: Vec<BinderFile> = Vec::with_capacity(file_headers.len());

        for header in file_headers {
            files.push(header.read_file_data(&mut br_dec)?);
        }

        bnd.files = files;

        Ok(bnd)
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        todo!()
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
