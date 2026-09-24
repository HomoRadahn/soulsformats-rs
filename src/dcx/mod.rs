use std::io::{self, Read, Seek, Write};

use crate::{
    dcx::{compression_info::*, zstd_helper::ZstdHelper},
    io::{BinaryReader, BinaryWriter, Endian},
    oodle, util,
};
pub mod compression_info;
mod deflate_helper;
mod zlib_helper;
mod zstd_helper;

#[derive(Debug, Clone, PartialEq)]
pub struct DCX {
    pub data: Vec<u8>,
    pub compression: CompressionInfo,
}

impl DCX {
    /// Creates a `DCX` from `Vec<u8>` and `CompressionInfo`
    pub fn new(data: Vec<u8>, compression: CompressionInfo) -> Self {
        Self { data, compression }
    }

    /// Checks if the provided `BinaryReader` contains a valid `DCX`
    pub fn is<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek,
    {
        if br.length()? < 4 {
            return Ok(false);
        }

        let magic = br.get_ascii_len(0, 4)?;

        Ok(magic == "DCP\0" || magic == "DCX\0")
    }

    /// Checks whether provided `Vec<u8>` is a valid `DCX`
    pub fn is_bytes(bytes: Vec<u8>) -> io::Result<bool> {
        let mut br = BinaryReader::from_bytes(bytes, Endian::Big, false);
        DCX::is(&mut br)
    }

    /// Checks whether provided file is a valid `DCX`
    pub fn is_file(path: impl Into<String>) -> io::Result<bool> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Big, false)?;
        DCX::is(&mut br)
    }

    /// Decompress `DCX` from provided `Vec<u8>`
    pub fn from_bytes(data: Vec<u8>) -> io::Result<Self> {
        let br = BinaryReader::from_bytes(data, Endian::Big, false);
        let (decompressed, compression) = DCX::decompress(br)?;
        Ok(Self {
            data: decompressed,
            compression,
        })
    }

    /// Decompress `DCX` from provided file
    pub fn from_file(path: impl Into<String>) -> io::Result<Self> {
        let br = BinaryReader::from_file(path.into(), Endian::Big, false)?;
        let (decompressed, compression) = DCX::decompress(br)?;
        Ok(Self {
            data: decompressed,
            compression,
        })
    }

    /// Compress `DCX` to specified file
    pub fn to_file(&self, path: impl Into<String>) -> io::Result<()> {
        let mut bw = BinaryWriter::to_file(path.into(), Endian::Big, false)?;
        let data = &self.data;
        DCX::compress(&mut bw, data, self.compression)?;
        Ok(())
    }

    /// Compress `DCX` to `Vec<u8>`
    pub fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Big, false);
        let data = &self.data;
        DCX::compress(&mut bw, data, self.compression)?;
        bw.close_bytes()
    }
}

/// Decompression Internal Functions
impl DCX {
    /// Decompress `DCX` from the provided `BinaryReader`
    pub fn decompress<R>(mut br: BinaryReader<R>) -> io::Result<(Vec<u8>, CompressionInfo)>
    where
        R: Read + Seek,
    {
        let mut compression: CompressionInfo = CompressionInfo::Unknown;

        let magic = br.read_ascii_len(4)?;
        if magic == "DCP\0" {
            let format = br.get_ascii_len(4, 4)?;
            match format.as_str() {
                "DFLT" => compression = CompressionInfo::DcpDflt,
                "EDGE" => compression = CompressionInfo::DcpEdge,
                other => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unrecognized DCP format: {}", other),
                    ));
                }
            }
        } else if magic == "DCX\0" {
            let format = br.get_ascii_len(0x28, 4)?;

            match format.as_str() {
                "DFLT" => {
                    let unk_04 = br.get_i32(0x4)?;
                    let unk_10 = br.get_i32(0x10)?;
                    let unk_14 = br.get_i32(0x14)?;
                    let unk_30 = br.get_i32(0x30)?;
                    let unk_38 = br.get_i32(0x38)?;
                    compression = CompressionInfo::DcxDflt(DcxDfltArgs::new(
                        unk_04, unk_10, unk_14, unk_30, unk_38,
                    ));
                }
                "EDGE" => compression = CompressionInfo::DcxEdge,
                "KRAK" => {
                    let compression_level = br.get_u8(0x30)?;
                    compression = CompressionInfo::DcxKrak(DcxKrakArgs {
                        compression_level,
                        oodle_compressor: ::oodle::OodleCompressor::Kraken,
                    });
                }
                "ZSTD" => {
                    let zstd_compression_level = br.get_u8(0x30)?;
                    compression = CompressionInfo::DcxZstd(zstd_compression_level);
                }
                other => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unrecognized DCX format: {}", other),
                    ));
                }
            }
        } else {
            let b0 = br.get_u8(0)?;
            let b1 = br.get_u8(1)?;

            if b0 == 0x78 && (b1 == 0x01 || b1 == 0x5E || b1 == 0x9C || b1 == 0xDA) {
                compression = CompressionInfo::Zlib;
            }
        }

        br.seek(0)?;

        let data = match compression {
            CompressionInfo::Zlib => {
                let size = br.length()?;
                zlib_helper::read_zlib(&mut br, size)?
            }
            CompressionInfo::DcpDflt => DCX::decompress_dcp_dflt(br)?,
            CompressionInfo::DcpEdge => DCX::decompress_dcp_edge(br)?,
            CompressionInfo::DcxEdge => DCX::decompress_dcx_edge(br)?,
            CompressionInfo::DcxDflt(args) => DCX::decompress_dcx_dflt(br, args)?,
            CompressionInfo::DcxKrak(args) => DCX::decompress_dcx_krak(br, args)?,
            CompressionInfo::DcxZstd(level) => DCX::decompress_dcx_zstd(br, level)?,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unrecognized DCX format",
                ));
            }
        };

        Ok((data, compression))
    }

    fn decompress_dcp_dflt<R>(mut br: BinaryReader<R>) -> io::Result<Vec<u8>>
    where
        R: Read + Seek,
    {
        br.assert_ascii(&["DCP\0"])?;
        br.assert_ascii(&["DFLT"])?;
        br.assert_i32(&[0x20])?;
        br.assert_i32(&[0x9000000])?;
        br.assert_i32(&[0])?;
        br.assert_i32(&[0])?;
        br.assert_i32(&[0])?;
        br.assert_i32(&[0x00010100])?;
        br.assert_ascii(&["DCS\0"])?;
        // uncompressed size
        br.read_i32()?;
        let compressed = br.read_i32()?;

        let output = zlib_helper::read_zlib(&mut br, util::convert_num(compressed)?)?;

        br.assert_ascii(&["DCA\0"])?;
        br.assert_i32(&[8])?;

        Ok(output)
    }

    fn decompress_dcx_dflt<R>(mut br: BinaryReader<R>, args: DcxDfltArgs) -> io::Result<Vec<u8>>
    where
        R: Read + Seek,
    {
        br.assert_ascii(&["DCX\0"])?;
        br.assert_i32(&[args.unk_04])?;
        br.assert_i32(&[0x18])?;
        br.assert_i32(&[0x24])?;
        br.assert_i32(&[args.unk_10])?;
        br.assert_i32(&[args.unk_14])?;

        br.assert_ascii(&["DCS\0"])?;
        // uncompressed size
        br.read_i32()?;
        // compressed size
        br.read_i32()?;
        br.assert_ascii(&["DCP\0"])?;
        br.assert_ascii(&["DFLT"])?;
        br.assert_i32(&[0x20])?;
        br.assert_i32(&[args.unk_30])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[args.unk_38])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[0x00010100])?;

        br.assert_ascii(&["DCA\0"])?;
        // Compressed Header Length
        br.read_i32()?;

        let len = br.length()?;
        let pos = br.position()?;
        zlib_helper::read_zlib(&mut br, len - pos)
    }

    fn decompress_dcp_edge<R>(mut br: BinaryReader<R>) -> io::Result<Vec<u8>>
    where
        R: Read + Seek,
    {
        br.assert_ascii(&["DCP\0"])?;
        br.assert_ascii(&["EDGE"])?;
        br.assert_i32(&[0x20])?;
        br.assert_i32(&[0x9000000])?;
        br.assert_i32(&[0x10000])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[0x00100100])?;

        br.assert_ascii(&["DCS\0"])?;
        let uncompressed = br.read_i32()?;
        let compressed = br.read_i32()?;
        br.assert_i32(&[0])?;
        let data_start = br.position()?;
        br.skip(compressed as i64)?;

        br.assert_ascii(&["DCA\0"])?;
        br.read_i32()?;
        br.assert_ascii(&["EgdT"])?;
        br.assert_i32(&[0x00010000])?;
        br.assert_i32(&[0x20])?;
        br.assert_i32(&[0x10])?;
        br.assert_i32(&[0x10000])?;
        let egdt_size = br.read_i32()?;
        let chunk_count = br.read_i32()?;
        br.assert_i32(&[0x100000])?;

        if egdt_size != (0x20 + chunk_count * 0x10) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected EdgT size in EDGE DCP",
            ));
        }

        let mut output: Vec<u8> = Vec::with_capacity(uncompressed as usize);

        for _ in 0..chunk_count {
            br.assert_i32(&[0])?;
            let offset = br.read_i32()?;
            let size = br.read_i32()? as usize;
            let compressed = br.assert_i32(&[0, 1])? == 1;

            let mut chunk = br.get_vec_u8(
                data_start + util::convert_num::<i32, u64>(offset)?,
                util::convert_num(size)?,
            )?;

            if compressed {
                let mut data = deflate_helper::decompress_deflate_bytes(&chunk[..])?;
                output.append(&mut data);
            } else {
                output.append(&mut chunk);
            }
        }

        Ok(output)
    }

    fn decompress_dcx_edge<R>(mut br: BinaryReader<R>) -> io::Result<Vec<u8>>
    where
        R: Read + Seek,
    {
        br.assert_ascii(&["DCX\0"])?;
        br.assert_i32(&[0x10000])?;
        br.assert_i32(&[0x18])?;
        br.assert_i32(&[0x24])?;
        br.assert_i32(&[0x24])?;
        let unk_1 = br.read_i32()?;

        br.assert_ascii(&["DCS\0"])?;
        let uncompressed = br.read_i32()?;
        br.read_i32()?;

        br.assert_ascii(&["DCP\0"])?;
        br.assert_ascii(&["EDGE"])?;
        br.assert_i32(&[0x20])?;
        br.assert_i32(&[0x9000000])?;
        br.assert_i32(&[0x10000])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[0x00100100])?;

        let dca_start = br.position()?;
        br.assert_ascii(&["DCA\0"])?;
        let dca_size = br.read_i32()?;

        br.assert_ascii(&["EgdT"])?;
        br.assert_i32(&[0x00010100])?;
        br.assert_i32(&[0x24])?;
        br.assert_i32(&[0x10])?;
        br.assert_i32(&[0x10000])?;

        // Uncompressed size of last block
        br.assert_i32(&[uncompressed % 0x10000, 0x10000])?;
        let egdt_size = br.read_i32()?;
        let chunk_count = br.read_i32()?;
        br.assert_i32(&[0x100000])?;

        if unk_1 != (0x50 + chunk_count * 0x10) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected unk_1 size in EDGE DCX",
            ));
        }

        if egdt_size != 0x24 + chunk_count * 0x10 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected EgdT size in EDGE DCX",
            ));
        }

        let mut output: Vec<u8> = Vec::with_capacity(uncompressed as usize);

        for _ in 0..chunk_count {
            br.assert_i32(&[0])?;
            let offset = br.read_i32()?;
            let size = br.read_i32()?;
            let compressed = br.assert_i32(&[0, 1])? == 1;

            let mut chunk = br.get_vec_u8(
                dca_start
                    + util::convert_num::<i32, u64>(dca_size)?
                    + util::convert_num::<i32, u64>(offset)?,
                util::convert_num(size)?,
            )?;

            if compressed {
                let mut data = deflate_helper::decompress_deflate_bytes(&chunk[..])?;
                output.append(&mut data);
            } else {
                output.append(&mut chunk);
            }
        }

        Ok(output)
    }

    fn decompress_dcx_krak<R>(mut br: BinaryReader<R>, args: DcxKrakArgs) -> io::Result<Vec<u8>>
    where
        R: Read + Seek,
    {
        br.assert_ascii(&["DCX\0"])?;
        br.assert_i32(&[0x11000])?;
        br.assert_i32(&[0x18])?;
        br.assert_i32(&[0x24])?;
        br.assert_i32(&[0x44])?;
        br.assert_i32(&[0x4C])?;
        br.assert_ascii(&["DCS\0"])?;
        let uncompressed_size = util::convert_num(br.read_u32()?)?;
        let compressed_size = util::convert_num::<u32, usize>(br.read_u32()?)?;
        br.assert_ascii(&["DCP\0"])?;
        br.assert_ascii(&["KRAK"])?;
        br.assert_i32(&[0x20])?;
        br.assert_u8(&[args.compression_level])?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;
        br.assert_i32(&[0])?;
        br.assert_i32(&[0])?;
        br.assert_i32(&[0])?;
        br.assert_i32(&[0x10100])?;
        br.assert_ascii(&["DCA\0"])?;
        br.assert_i32(&[8])?;

        let compressed = br.read_vec_u8(compressed_size as u64)?;
        oodle::decompress(&compressed, uncompressed_size)
    }

    fn decompress_dcx_zstd<R>(mut br: BinaryReader<R>, compression_level: u8) -> io::Result<Vec<u8>>
    where
        R: Read + Seek,
    {
        br.assert_ascii(&["DCX\0"])?;
        br.assert_i32(&[0x11000])?;
        br.assert_i32(&[0x18])?;
        br.assert_i32(&[0x24])?;
        br.assert_i32(&[0x44])?;
        br.assert_i32(&[0x4C])?;

        br.assert_ascii(&["DCS\0"])?;
        // uncompressed size
        br.read_i32()?;
        // compressed size
        let compressed = br.read_i32()?;

        br.assert_ascii(&["DCP\0"])?;
        br.assert_ascii(&["ZSTD"])?;
        br.assert_i32(&[0x20])?;
        br.assert_u8(&[compression_level])?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[0x010100])?;

        br.assert_ascii(&["DCA\0"])?;
        br.assert_i32(&[8])?;

        ZstdHelper::read_zstd(&mut br, util::convert_num(compressed)?)
    }
}

/// Compression Internal Functions
impl DCX {
    /// Compress `DCX` to provided `BinaryWriter`
    pub fn compress<W>(
        bw: &mut BinaryWriter<W>,
        data: &Vec<u8>,
        compression: CompressionInfo,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        match compression {
            CompressionInfo::Zlib => DCX::compress_zlib(bw, data)?,
            CompressionInfo::DcpEdge => DCX::compress_dcp_edge(bw, data)?,
            CompressionInfo::DcpDflt => DCX::compress_dcp_dflt(bw, data)?,
            CompressionInfo::DcxEdge => DCX::compress_dcx_edge(bw, data)?,
            CompressionInfo::DcxDflt(args) => DCX::compress_dcx_dflt(bw, data, args)?,
            CompressionInfo::DcxKrak(args) => DCX::compress_dcx_krak(bw, data, args)?,
            CompressionInfo::DcxZstd(level) => DCX::compress_dcx_zstd(bw, data, level)?,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unrecognized DCX format",
                ));
            }
        };

        Ok(())
    }

    fn compress_zlib<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        zlib_helper::write_zlib(bw, 0xDA, data)?;
        Ok(())
    }

    fn compress_dcp_dflt<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bw.write_ascii("DCP", true)?;
        bw.write_ascii("DFLT", false)?;
        bw.write_i32(0x20)?;
        bw.write_i32(0x9000000)?;
        bw.write_i32(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0x00010100)?;

        bw.write_ascii("DCS", true)?;
        bw.write_i32(util::convert_num(data.len())?)?;
        bw.reserve_i32("compressed_size")?;

        let compressed_size = zlib_helper::write_zlib(bw, 0xDA, data)?;

        bw.fill_i32("compressed_size", compressed_size)?;

        bw.write_ascii("DCA", true)?;
        bw.write_i32(8)?;
        bw.finalize()?;

        Ok(())
    }

    fn compress_dcx_dflt<W>(
        bw: &mut BinaryWriter<W>,
        data: &Vec<u8>,
        args: DcxDfltArgs,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bw.write_ascii("DCX", true)?;

        bw.write_i32(args.unk_04)?;

        bw.write_i32(0x18)?;
        bw.write_i32(0x24)?;

        bw.write_i32(args.unk_10)?;
        bw.write_i32(args.unk_14)?;

        bw.write_ascii("DCS", true)?;
        bw.write_i32(util::convert_num(data.len())?)?;
        bw.reserve_i32("compressed_size")?;
        bw.write_ascii("DCP", true)?;
        bw.write_ascii("DFLT", false)?;
        bw.write_i32(0x20)?;

        bw.write_i32(args.unk_30)?;
        bw.write_i32(0)?;
        bw.write_i32(args.unk_38)?;

        bw.write_i32(0)?;
        bw.write_i32(0x00010100)?;
        bw.write_ascii("DCA", true)?;
        bw.write_i32(8)?;

        let compressed_start = bw.position()?;

        zlib_helper::write_zlib(bw, 0xDA, data)?;

        let pos = bw.position()?;
        bw.fill_i32(
            "compressed_size",
            util::convert_num(pos - compressed_start)?,
        )?;

        bw.finalize()?;

        Ok(())
    }

    #[allow(unused)]
    fn compress_dcx_krak<W>(
        bw: &mut BinaryWriter<W>,
        data: &[u8],
        args: DcxKrakArgs,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let compressed = oodle::compress(data, args.oodle_compressor, args.compression_level)?;

        bw.write_ascii("DCX", true)?;
        bw.write_i32(0x11000)?;
        bw.write_i32(0x18)?;
        bw.write_i32(0x24)?;
        bw.write_i32(0x44)?;
        bw.write_i32(0x4C)?;
        bw.write_ascii("DCS", true)?;
        bw.write_u32(util::convert_num(data.len())?)?;
        bw.write_u32(util::convert_num(compressed.len())?)?;
        bw.write_ascii("DCP", true)?;
        bw.write_ascii("KRAK", false)?;
        bw.write_i32(0x20)?;
        bw.write_u8(args.compression_level)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0x10100)?;
        bw.write_ascii("DCA", true)?;
        bw.write_i32(8)?;
        bw.write_vec_u8(compressed)?;
        bw.pad_00(0x10)?;
        Ok(())
    }

    fn compress_dcp_edge<W>(bw: &mut BinaryWriter<W>, data: &[u8]) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let mut chunk_count = data.len() / 0x10000;
        let chunk_remainder = data.len() % 0x10000;
        if chunk_remainder > 0 {
            chunk_count += 1;
        }

        bw.write_ascii("DCP", true)?;
        bw.write_ascii("EDGE", false)?;
        bw.write_i32(0x20)?;
        bw.write_u8(9)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        bw.write_i32(0x10000)?;
        bw.write_i32(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0x100100)?;

        bw.write_ascii("DCS", true)?;
        bw.write_i32(util::convert_num(data.len())?)?;
        bw.reserve_i32("compressed_size")?;
        bw.write_i32(0)?;

        let data_start = bw.position()?;
        let mut chunk_headers: Vec<EdgeChunk> = Vec::new();
        for index in 0..chunk_count {
            let mut chunk_size = 0x10000;

            if index == chunk_count - 1 && chunk_remainder > 0 {
                chunk_size = chunk_remainder;
            }

            let chunk_offset = index * 0x10000;

            let input = &data[chunk_offset..(chunk_offset + chunk_size)];
            let compressed = deflate_helper::compress_deflate_bytes(input)?;

            let (chunk, is_compressed) = if compressed.len() < chunk_size {
                (compressed, true)
            } else {
                (input.to_vec(), false)
            };

            let comp_chunk_offset: i32 = util::convert_num(bw.position()? - data_start)?;
            let comp_chunk_size = chunk.len();
            bw.write_vec_u8(chunk)?;
            bw.pad_00(0x10)?;

            chunk_headers.push(EdgeChunk {
                compressed_offset: comp_chunk_offset,
                compressed_length: util::convert_num(comp_chunk_size)?,
                is_compressed,
            });
        }

        let pos = bw.position()?;
        bw.fill_i32("compressed_size", util::convert_num(pos - data_start)?)?;

        let dca_start = bw.position()?;
        bw.write_ascii("DCA", true)?;
        bw.reserve_i32("dca_size")?;

        let egdt_start = bw.position()?;

        bw.write_ascii("EgdT", false)?;
        bw.write_i32(0x10000)?;
        bw.write_i32(0x20)?;
        bw.write_i32(0x10)?;
        bw.write_i32(0x10000)?;
        bw.reserve_i32("egdt_size")?;
        bw.write_i32(util::convert_num(chunk_count)?)?;
        bw.write_i32(0x100000)?;

        for index in chunk_headers {
            bw.write_i32(0)?;
            bw.write_i32(index.compressed_offset)?;
            bw.write_i32(index.compressed_length)?;
            match index.is_compressed {
                true => bw.write_i32(1)?,
                false => bw.write_i32(0)?,
            };
        }
        let pos = bw.position()?;
        bw.fill_i32("egdt_size", util::convert_num(pos - egdt_start)?)?;
        bw.fill_i32("dca_size", util::convert_num(pos - dca_start)?)?;
        bw.finalize()?;

        Ok(())
    }

    fn compress_dcx_edge<W>(bw: &mut BinaryWriter<W>, data: &[u8]) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let mut chunk_count = data.len() / 0x10000;
        let chunk_remainder = data.len() % 0x10000;
        if chunk_remainder > 0 {
            chunk_count += 1;
        }

        bw.write_ascii("DCX", true)?;
        bw.write_i32(0x10000)?;
        bw.write_i32(0x18)?;
        bw.write_i32(0x24)?;
        bw.write_i32(0x24)?;
        bw.write_i32(util::convert_num(0x50 + chunk_count * 0x10)?)?;

        bw.write_ascii("DCS", true)?;
        bw.write_i32(util::convert_num(data.len())?)?;
        bw.reserve_i32("compressed_size")?;

        bw.write_ascii("DCP", true)?;
        bw.write_ascii("EDGE", false)?;
        bw.write_i32(0x20)?;
        bw.write_i32(0x9000000)?;
        bw.write_i32(0x10000)?;
        bw.write_i32(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0x00100100)?;

        let dca_start = bw.position()?;
        bw.write_ascii("DCA", true)?;
        bw.reserve_i32("dca_size")?;
        let egdt_start = bw.position()?;
        bw.write_ascii("EgdT", false)?;
        bw.write_i32(0x00010100)?;
        bw.write_i32(0x24)?;
        bw.write_i32(0x10)?;
        bw.write_i32(0x10000)?;
        bw.write_i32(util::convert_num(chunk_remainder)?)?;
        bw.reserve_i32("egdt_size")?;
        bw.write_i32(util::convert_num(chunk_count)?)?;
        bw.write_i32(0x100000)?;

        for index in 0..chunk_count {
            bw.write_i32(0)?;
            bw.reserve_i32(format!("chunk_{index}_offset"))?;
            bw.reserve_i32(format!("chunk_{index}_size"))?;
            bw.reserve_i32(format!("chunk_{index}_compressed"))?;
        }

        let pos = bw.position()?;
        bw.fill_i32("dca_size", util::convert_num(pos - dca_start)?)?;
        bw.fill_i32("egdt_size", util::convert_num(pos - egdt_start)?)?;

        let data_start = bw.position()?;

        let mut compressed_size: i32 = 0;
        for index in 0..chunk_count {
            let mut chunk_size = 0x10000;

            if index == chunk_count - 1 && chunk_remainder > 0 {
                chunk_size = chunk_remainder;
            }

            let chunk_offset = index * 0x10000;

            let input = &data[chunk_offset..(chunk_offset + chunk_size)];
            let compressed = deflate_helper::compress_deflate_bytes(input)?;

            let (chunk, is_compressed) = if compressed.len() < chunk_size {
                (compressed, true)
            } else {
                (input.to_vec(), false)
            };

            match is_compressed {
                true => bw.fill_i32(format!("chunk_{index}_compressed"), 1)?,
                false => bw.fill_i32(format!("chunk_{index}_compressed"), 0)?,
            };

            compressed_size += util::convert_num::<usize, i32>(chunk.len())?;
            let pos = bw.position()?;
            bw.fill_i32(
                format!("chunk_{index}_offset"),
                util::convert_num(pos - data_start)?,
            )?;
            bw.fill_i32(
                format!("chunk_{index}_size"),
                util::convert_num(chunk.len())?,
            )?;
            bw.write_vec_u8(chunk)?;
            bw.pad_00(0x10)?;
        }

        bw.fill_i32("compressed_size", compressed_size)?;

        bw.finalize()?;

        Ok(())
    }

    fn compress_dcx_zstd<W>(
        bw: &mut BinaryWriter<W>,
        data: &[u8],
        compression_level: u8,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let compressed = ZstdHelper::write_zstd(data, compression_level)?;

        bw.write_ascii("DCX", true)?;
        bw.write_i32(0x11000)?;
        bw.write_i32(0x18)?;
        bw.write_i32(0x24)?;
        bw.write_i32(0x44)?;
        bw.write_i32(0x4C)?;
        bw.write_ascii("DCS", true)?;
        bw.write_u32(util::convert_num(data.len())?)?;
        bw.write_u32(util::convert_num(compressed.len())?)?;
        bw.write_ascii("DCP", true)?;
        bw.write_ascii("ZSTD", false)?;
        bw.write_i32(0x20)?;
        bw.write_u8(compression_level)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        bw.write_u8(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0x10100)?;
        bw.write_ascii("DCA", true)?;
        bw.write_i32(8)?;
        bw.write_vec_u8(compressed)?;
        bw.pad_00(0x10)?;

        bw.finalize()?;

        Ok(())
    }
}
