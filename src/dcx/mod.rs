use std::io::{self, Read, Seek, Write};

use crate::{dcx::{compression_info::*, deflate_helper::DeflateHelper, zlib_helper::ZlibHelper, zstd_helper::ZstdHelper}, io::{BinaryReader, BinaryWriter, Endian}};
pub mod compression_info;
mod zlib_helper;
mod zstd_helper;
mod deflate_helper;


pub struct DCX {
    pub decompressed: Vec<u8>,
    pub compression: Box<dyn CompressionInfo>
}

impl DCX {
    /// Creates a new DCX from a decompressed vector of bytes and compression info
    pub fn new(decompressed: Vec<u8>, compression: Box<dyn CompressionInfo>) -> Self {
        Self { decompressed, compression }
    }
    fn is_base<R>(mut br: BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek
    {
        if br.length()? < 4 {
            return Ok(false);
        }

        let magic = br.get_ascii_len(0, 4)?;

        Ok(magic == "DCP\0" || magic == "DCX\0")
    }

    /// Checks whether provided bytes are a valid dcx
    pub fn is_bytes(bytes: Vec<u8>) -> io::Result<bool> {
        let br = BinaryReader::from_bytes(bytes, Endian::Big, true);
        DCX::is_base(br)
    }
    
    /// Checks whether provided file is a valid dcx
    pub fn is_file(path: String) -> io::Result<bool> {
        let br = BinaryReader::from_file(path, Endian::Big, true)?;
        DCX::is_base(br)
    }

    /// Decompress DCX from provided bytes
    pub fn decompress_bytes(data: Vec<u8>) -> io::Result<Self> {
        let br = BinaryReader::from_bytes(data, Endian::Big, true);
        let (decompressed, compression) = DCX::decompress(br)?;
        Ok(Self { decompressed, compression })
    }

    /// Decompress DCX from provided file
    pub fn decompress_file(path: String) -> io::Result<Self> {
        let br = BinaryReader::from_file(path, Endian::Big, true)?;
        let (decompressed, compression) = DCX::decompress(br)?;
        Ok(Self { decompressed, compression })
    }

    pub fn compress_to_file(&self, path: String) -> io::Result<()> {
        let mut bw = BinaryWriter::to_file(path, Endian::Big, true)?;
        let data = &self.decompressed;
        DCX::compress(&mut bw, data, &self.compression)?;
        Ok(())
    }

    pub fn compress_to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Big, true);
        let data = &self.decompressed;
        DCX::compress(&mut bw, data, &self.compression)?;
        bw.close_bytes()

    }
}

/// Decompression Internal Functions
impl DCX {
    fn decompress<R>(mut br: BinaryReader<R>) -> io::Result<(Vec<u8>, Box<dyn CompressionInfo>)>
    where
        R: Read + Seek
    {
        let mut compression: Box<dyn CompressionInfo> = Box::new(UnkCompressionInfo);

        let magic = br.read_ascii_len(4)?;
        if magic == "DCP\0" {
            let format = br.get_ascii_len(4, 4)?;
            match format.as_str() {
                "DFLT" => compression = Box::new(DcpDfltCompressionInfo),
                "EDGE" => compression = Box::new(DcpEdgeCompressionInfo),
                other => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unrecognized DCP format: {}", other)))
            }
        }
        else if magic == "DCX\0" {
            let format = br.get_ascii_len(0x28, 4)?;

            match format.as_str() {
                "DFLT" => {
                    let unk04 = br.get_i32(0x4)?;
                    let unk10 = br.get_i32(0x10)?;
                    let unk14 = br.get_i32(0x14)?;
                    let unk30 = br.get_i32(0x30)?;
                    let unk38 = br.get_i32(0x38)?;
                    compression = Box::new(DcxDfltCompressionInfo::new(unk04, unk10, unk14, unk30, unk38));
                },
                "EDGE" => compression = Box::new(DcxEdgeCompressionInfo),
                "KRAK" => unimplemented!(),
                "ZSTD" => {
                    let zstd_compression_level = br.get_u8(0x30)?;
                    compression = Box::new(DcxZstdCompressionInfo::new(zstd_compression_level));
                },
                other => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unrecognized DCX format: {}", other)))
            }
        }
        else {
            let b0 = br.get_u8(0)?;
            let b1 = br.get_u8(1)?;

            if b0 == 0x78 && (b1 == 0x01 || b1 == 0x5E || b1 == 0x9C || b1 == 0xDA) {
                compression = Box::new(ZlibCompressionInfo);
            }
        }

        br.seek(0)?;

        let data = match compression.get_type() {
            Type::Zlib => {
                let size = br.length()?;
                ZlibHelper::read_zlib(&mut br, size)?
            },
            Type::DcpDflt => DCX::decompress_dcp_dflt(br)?,
            Type::DcpEdge => DCX::decompress_dcp_edge(br)?,
            Type::DcxEdge => DCX::decompress_dcx_edge(br)?,
            Type::DcxDflt => DCX::decompress_dcx_dflt(br, &compression)?,
            Type::DcxKrak => DCX::decompress_dcx_krak(br, &compression)?,
            Type::DcxZstd => DCX::decompress_dcx_zstd(br, &compression)?,
            _ => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unrecognized DCX format"))
        };

        Ok((data, compression))
    }

    fn decompress_dcp_dflt<R>(mut br: BinaryReader<R>) -> io::Result<Vec<u8>>
    where 
        R: Read + Seek
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
        
        let output = ZlibHelper::read_zlib(&mut br, u64::try_from(compressed).unwrap())?;
        
        br.assert_ascii(&["DCA\0"])?;
        br.assert_i32(&[8])?;

        return Ok(output);
    }

    fn decompress_dcx_dflt<R>(mut br: BinaryReader<R>, compression: &Box<dyn CompressionInfo>) -> io::Result<Vec<u8>>
    where 
        R: Read + Seek
    {
        let args = compression.get_dcx_dflt_args()?;
        br.assert_ascii(&["DCX\0"])?;
        br.assert_i32(&[args.unk04])?;
        br.assert_i32(&[0x18])?;
        br.assert_i32(&[0x24])?;
        br.assert_i32(&[args.unk10])?;
        br.assert_i32(&[args.unk14])?;

        br.assert_ascii(&["DCS\0"])?;
        // uncompressed size
        br.read_i32()?;
        // compressed size
        br.read_i32()?;
        br.assert_ascii(&["DCP\0"])?;
        br.assert_ascii(&["DFLT"])?;
        br.assert_i32(&[0x20])?;
        br.assert_i32(&[args.unk30])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[args.unk38])?;
        br.assert_i32(&[0x0])?;
        br.assert_i32(&[0x00010100])?;

        br.assert_ascii(&["DCA\0"])?;
        // Compressed Header Length
        br.read_i32()?;
        
        let len = br.length()?;
        let pos = br.position()?;
        ZlibHelper::read_zlib(&mut br, len - pos)
    }

    fn decompress_dcp_edge<R>(mut br: BinaryReader<R>) -> io::Result<Vec<u8>>
    where 
        R: Read + Seek
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
            return Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected EdgT size in EDGE DCP"));
        }


        let mut output: Vec<u8> = Vec::with_capacity(uncompressed as usize);

        for _ in 0..chunk_count {
            br.assert_i32(&[0])?;
            let offset = br.read_i32()? as usize;
            let size = br.read_i32()? as usize;
            let compressed = br.assert_i32(&[0, 1])? == 1;

            let mut chunk = br.get_u8_vec(data_start + u64::try_from(offset).unwrap(), u64::try_from(size).unwrap())?;

            if compressed {
                let mut data = DeflateHelper::decompress_deflate_bytes(&chunk[..])?;
                output.append(&mut data);
            } else {
                output.append(&mut chunk);
            }
        }
        
        return Ok(output);
    }

    fn decompress_dcx_edge<R>(mut br: BinaryReader<R>) -> io::Result<Vec<u8>>
    where 
        R: Read + Seek
    {
        br.assert_ascii(&["DCX\0"])?;
        br.assert_i32(&[0x10000])?;
        br.assert_i32(&[0x18])?;
        br.assert_i32(&[0x24])?;
        br.assert_i32(&[0x24])?;
        let unk1 = br.read_i32()?;
        
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

        if unk1 != (0x50 + chunk_count * 0x10) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected unk1 size in EDGE DCX"));
        }
        
        if egdt_size != 0x24 + chunk_count * 0x10 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected EgdT size in EDGE DCX"));
        }

        let mut output: Vec<u8> = Vec::with_capacity(uncompressed as usize);

        for _ in 0..chunk_count {
            br.assert_i32(&[0])?;
            let offset = br.read_i32()? as usize;
            let size = br.read_i32()? as usize;
            let compressed = br.assert_i32(&[0, 1])? == 1;

            let mut chunk = br.get_u8_vec(u64::try_from(dca_start).unwrap() + u64::try_from(dca_size).unwrap() + u64::try_from(offset).unwrap(), u64::try_from(size).unwrap())?;

            if compressed {
                let mut data = DeflateHelper::decompress_deflate_bytes(&chunk[..])?;
                output.append(&mut data);
            } else {
                output.append(&mut chunk);
            }
        }
        
        return Ok(output);
    }

    #[allow(dead_code, unused)]
    fn decompress_dcx_krak<R>(mut br: BinaryReader<R>, compression: &Box<dyn CompressionInfo>) -> io::Result<Vec<u8>> {
        unimplemented!()
    }

    fn decompress_dcx_zstd<R>(mut br: BinaryReader<R>, compression: &Box<dyn CompressionInfo>) -> io::Result<Vec<u8>>
    where 
        R: Read + Seek
    {
        let compression_level = compression.get_dcx_zstd_args()?;
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
        
        ZstdHelper::read_zstd(&mut br, u64::try_from(compressed).unwrap())
    }
}

/// Compression Internal Functions 
impl DCX {
    fn compress<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>, compression: &Box<dyn CompressionInfo>) -> io::Result<()>
    where
        W: Write + Seek
    {
        match compression.get_type() {
            Type::Zlib => todo!(),
            Type::DcpEdge => todo!(),
            Type::DcpDflt => DCX::compress_dcp_dflt(bw, data)?,
            Type::DcxEdge => todo!(),
            Type::DcxDflt => todo!(),
            Type::DcxKrak => todo!(),
            Type::DcxZstd => todo!(),
            _ => todo!(),
        };

        Ok(())
    }

    fn compress_dcp_dflt<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>) -> io::Result<()>
    where
        W: Write + Seek
    {
        bw.write_ascii("DCP\0", false)?;
        bw.write_ascii("DFLT", false)?;
        bw.write_i32(0x20)?;
        bw.write_i32(0x9000000)?;
        bw.write_i32(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0)?;
        bw.write_i32(0x00010100)?;

        bw.write_ascii("DCS\0", false)?;
        bw.write_i32(i32::try_from(data.len()).unwrap())?;
        let compressed_size_res = bw.reserve_i32()?;

        let compressed_size = ZlibHelper::write_zlib(bw, 0xDA, data)?;

        bw.fill_i32(compressed_size_res, compressed_size)?;

        bw.write_ascii("DCA\0", false)?;
        bw.write_i32(8)?;

        Ok(())
    }
}