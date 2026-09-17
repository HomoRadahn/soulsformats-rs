mod binder;
mod file;
mod hashtable;

pub use file::BinderFile;

use crate::{SoulsFile, bnd::{binder::*, file::BinderFileHeader}, dcx::compression_info::CompressionInfo, format::SoulsFileInternal, io::{BinaryReader, BinaryWriter, Endian}, util};
use std::io::{self, ErrorKind::InvalidData, Read, Seek, Write};

/// A general-purpose file container used since DS2. Extension: .*bnd
pub struct BND4 {
    /// The files contained within this BND4
    pub files: Vec<BinderFile>,
    /// A timestamp or version number, 8 characters maximum
    pub version: String,
    /// Indicates the format of this BND4
    pub format: binder::Format,
    pub unk04: bool,
    pub unk05: bool,
    /// Endian format to write in
    pub endian: Endian,
    /// Controls ordering of flag bits
    pub bit_endian: Endian,
    /// Whether to encode filenames as UTF-8 or Shift JIS
    pub unicode: bool,
    /// Indicates presence of filename hash table
    pub extended: u8,
    /// DCX Compression info
    pub compression: CompressionInfo
}

impl BND4 {
    /// Creates an empty BND4 formatted for DS3
    pub fn new(compression: CompressionInfo) -> Self {
        Self {
            files: Vec::new(),
            version: DateTime { year: 2026, month: 1, day: 1, hour: 20, minute: 0 }.to_bnd_timestamp(),
            format: Format::IDs | Format::Names1 | Format::Names2 | Format::Compression,
            unk04: false,
            unk05: false,
            endian: Endian::Little,
            bit_endian: Endian::Little,
            unicode: true,
            extended: 4,
            compression
        }
    }

    fn read_header<R>(br: &mut BinaryReader<R>, bnd: &mut BND4) -> io::Result<Vec<BinderFileHeader>>
    where
        R: Read + Seek
    {
        br.assert_ascii(&["BND4"])?;
        bnd.unk04 = br.read_bool()?;
        bnd.unk05 = br.read_bool()?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;

        br.assert_u8(&[0])?;
        bnd.endian = match br.read_bool()? {
            true => Endian::Big,
            false => Endian::Little,
        };
        bnd.bit_endian = match !br.read_bool()? {
            true => Endian::Big,
            false => Endian::Little,
        };
        br.assert_u8(&[0])?;

        br.set_endian(bnd.endian);

        let file_count = br.read_i32()?;
        br.assert_i64(&[0x40])?; // Header size
        bnd.version = br.read_fix_str(8)?;
        let file_header_size = br.read_i64()?;
        br.read_i64()?; // Headers end (incl. hash table)

        bnd.unicode = br.read_bool()?;
        bnd.format = Format::read(br, bnd.bit_endian)?;
        bnd.extended = br.assert_u8(&[0, 1, 4, 0x80])?;
        br.assert_u8(&[0])?;

        br.assert_i32(&[0])?;

        if bnd.extended == 4 {
            let hash_table_offset = br.read_i64()?;
            let pos = br.position()?;
            br.seek(hash_table_offset as u64)?;
            hashtable::assert(br)?;
            br.seek(pos)?;
        }
        else {
            br.assert_i64(&[0])?;
        }

        if file_header_size != get_bnd4_file_header_size(bnd.format) {
            return Err(io::Error::new(InvalidData, "Invalid file header size"));
        }

        let mut file_headers: Vec<BinderFileHeader> = Vec::with_capacity(file_count as usize);
        for _ in 0..file_count {
            file_headers.push(BinderFileHeader::read_bnd4_header(br, bnd.format, bnd.bit_endian, bnd.unicode)?);
        }

        Ok(file_headers)

    }

    fn write_header<W>(&self, bw: &mut BinaryWriter<W>, file_headers: &mut Vec<BinderFileHeader>) -> io::Result<()>
    where 
        W: Write + Seek
    {
        bw.set_endian(self.endian);

        bw.write_ascii("BND4", false)?;

        bw.write_bool(self.unk04)?;
        bw.write_bool(self.unk05)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        
        bw.write_u8(0)?;
        match self.endian {
            Endian::Big => bw.write_bool(true)?,
            Endian::Little => bw.write_bool(false)?
        }
        match self.bit_endian {
            Endian::Big => bw.write_bool(false)?,
            Endian::Little => bw.write_bool(true)?
        }
        bw.write_u8(0)?;

        bw.write_i32(util::try_from_to_io_result(file_headers.len())?)?;
        bw.write_i64(0x40)?;
        bw.write_fix_str(self.version.clone(), 8, 0)?;
        bw.write_i64(binder::get_bnd4_file_header_size(self.format))?;
        bw.reserve_i64("headers-end")?;

        bw.write_bool(self.unicode)?;
        self.format.write(bw, self.bit_endian)?;
        bw.write_u8(self.extended)?;
        bw.write_u8(0)?;

        bw.write_i32(0)?;
        bw.reserve_i64("hash-table-offset")?;

        for i in 0..file_headers.len() {
            file_headers[i].write_bnd4_header(bw, self.format, self.bit_endian, util::try_from_to_io_result(i)?)?;
        }

        for i in 0..self.files.len() {
            file_headers[i].write_bnd4_file_data(bw, self.format, i as i32, self.files[i].bytes.clone())?;
        }

        for i in 0..file_headers.len() {
            file_headers[i].write_file_name(bw, self.format, util::try_from_to_io_result(i)?, self.unicode)?;
        }

        if self.extended == 4 {
            bw.pad_00(0x8)?;
            let pos = bw.position()?;
            bw.fill_i64("hash-table-offset", util::try_from_to_io_result(pos)?)?;
            hashtable::write(bw, &file_headers)?;
        }
        else {
            bw.fill_i64("hash-table-offset", 0)?;
        }

        let pos = bw.position()?;
        bw.fill_i64("headers-end", util::try_from_to_io_result(pos)?)?;

        Ok(())
    }
}

impl SoulsFileInternal<BND4> for BND4 {
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<BND4>
    where
        R: Read + Seek
    {
        let (mut reader, compression) = util::get_decompressed_binary_reader(br)?;
        let mut bnd = BND4::new(compression);
        
        let file_headers = BND4::read_header(&mut reader, &mut bnd)?;
        let mut files: Vec<BinderFile> = Vec::with_capacity(file_headers.len());

        for header in file_headers {
            files.push(header.read_file_data(&mut reader)?);
        }

        bnd.files = files;
        Ok(bnd)
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek
    {
        let mut file_headers: Vec<BinderFileHeader> = Vec::with_capacity(self.files.len());

        for file in &self.files {
            file_headers.push(BinderFileHeader::from_binder_file(&file));
        }

        BND4::write_header(&self, bw, &mut file_headers)?;

        Ok(())
    }

    fn is<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek
    {
        let len = br.length()?;
        Ok(len >= 4 && br.get_ascii_len(0, 4)? == "BND4")    
    }
    
    fn get_compression(&self) -> CompressionInfo {
        self.compression
    }
}

impl SoulsFile<BND4> for BND4 {}