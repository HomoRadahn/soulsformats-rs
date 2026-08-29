use std::io::{self, Read, Seek};

use crate::{dcx::{compression_info::*, zlib_helper::ZlibHelper}, io::{BinaryReader, Endian}};
pub mod compression_info;
mod zlib_helper;


pub struct DCX;

impl DCX {
    fn is<R>(mut br: BinaryReader<R>) -> io::Result<bool>
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
        DCX::is(br)
    }
    
    /// Checks whether provided file is a valid dcx
    pub fn is_file(path: String) -> io::Result<bool> {
        let br = BinaryReader::from_file(path, Endian::Big, true)?;
        DCX::is(br)
    }

    /// Decompress DCX from provided bytes
    pub fn decompress_bytes(data: Vec<u8>) -> io::Result<(Vec<u8>, Box<dyn CompressionInfo>)> {
        let br = BinaryReader::from_bytes(data, Endian::Big, true);
        DCX::decompress(br)
    }

    /// Decompress DCX from provided file
    pub fn decompress_file(path: String) -> io::Result<(Vec<u8>, Box<dyn CompressionInfo>)> {
        let br = BinaryReader::from_file(path, Endian::Big, true)?;
        DCX::decompress(br)
    }
}

/// Decompression Internal Functions
impl DCX {
    fn decompress<R>(mut br: BinaryReader<R>) -> io::Result<(Vec<u8>, Box<dyn CompressionInfo>)>
    where
        R: Read + Seek
    {
        let mut compression: Box<dyn CompressionInfo> = Box::new(UnkCompressionInfo);
        br.set_endian(Endian::Big);

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
                    compression = Box::new(DcxDfltCompressionInfo { unk04, unk10, unk14, unk30, unk38 });
                },
                "EDGE" => compression = Box::new(DcxEdgeCompressionInfo),
                "KRAK" => unimplemented!(),
                "ZSTD" => {
                    let zstd_compression_level = br.get_u8(0x30)?;
                    compression = Box::new(DcxZstdCompressionInfo { compression_level: zstd_compression_level });
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
            _ => todo!()
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
        let _ = br.read_i32()?;
        let compressed = br.read_i32()?;
        
        let output = ZlibHelper::read_zlib(&mut br, compressed as u64)?;
        
        br.assert_ascii(&["DCA\0"])?;
        br.assert_i32(&[8])?;

        return Ok(output);
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
        let _ = br.read_i32()?;
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

            let mut chunk = br.get_u8_vec(data_start + offset as u64, size as u64)?;

            if compressed {
                let mut data = ZlibHelper::decompress_deflate_bytes(&chunk[..])?;
                output.append(&mut data);
            } else {
                output.append(&mut chunk);
            }
        }
        
        return Ok(output);
    }
}