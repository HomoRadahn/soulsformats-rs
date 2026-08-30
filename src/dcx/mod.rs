use std::{collections::HashMap, io::{self, Read, Seek, Write}};

use crate::{dcx::{compression_info::*, deflate_helper::DeflateHelper, zlib_helper::ZlibHelper, zstd_helper::ZstdHelper}, io::{BinaryReader, BinaryWriter, Endian, writer::Reservation}};
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
            Type::Zlib => DCX::compress_zlib(bw, data)?,
            Type::DcpEdge => DCX::compress_dcp_edge(bw, data)?,
            Type::DcpDflt => DCX::compress_dcp_dflt(bw, data)?,
            Type::DcxEdge => DCX::compress_dcx_edge(bw, data)?,
            Type::DcxDflt => DCX::compress_dcx_dflt(bw, data, &compression)?,
            Type::DcxKrak => DCX::compress_dcx_krak(bw, data, &compression)?,
            Type::DcxZstd => DCX::compress_dcx_zstd(bw, data, &compression)?,
            _ => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unrecognized DCX format")),
        };

        Ok(())
    }

    fn compress_zlib<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>) -> io::Result<()>
    where 
        W: Write + Seek
    {
        ZlibHelper::write_zlib(bw, 0xDA, data)?;
        Ok(())
    }

    fn compress_dcp_dflt<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>) -> io::Result<()>
    where
        W: Write + Seek
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
        bw.write_i32(i32::try_from(data.len()).unwrap())?;
        let compressed_size_res = bw.reserve_i32()?;

        let compressed_size = ZlibHelper::write_zlib(bw, 0xDA, data)?;

        bw.fill_i32(compressed_size_res, compressed_size)?;

        bw.write_ascii("DCA", true)?;
        bw.write_i32(8)?;
        bw.finalize()?;

        Ok(())
    }

    fn compress_dcx_dflt<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>, compression: &Box<dyn CompressionInfo>) -> io::Result<()>
    where
        W: Write + Seek
    {
        let args = compression.get_dcx_dflt_args()?;
        bw.write_ascii("DCX", true)?;

        bw.write_i32(args.unk04)?;
        
        bw.write_i32(0x18)?;
        bw.write_i32(0x24)?;
        
        bw.write_i32(args.unk10)?;
        bw.write_i32(args.unk14)?;

        bw.write_ascii("DCS", true)?;
        bw.write_i32(i32::try_from(data.len()).unwrap())?;
        let compressed_size_res = bw.reserve_i32()?;
        bw.write_ascii("DCP", true)?;
        bw.write_ascii("DFLT", false)?;
        bw.write_i32(0x20)?;

        bw.write_i32(args.unk30)?;
        bw.write_i32(0)?;
        bw.write_i32(args.unk38)?;
        
        bw.write_i32(0)?;
        bw.write_i32(0x00010100)?;
        bw.write_ascii("DCA", true)?;
        bw.write_i32(8)?;

        let compressed_start = bw.position()?;

        ZlibHelper::write_zlib(bw, 0xDA, data)?;

        let pos = bw.position()?;
        bw.fill_i32(compressed_size_res, i32::try_from(pos - compressed_start).unwrap())?;

        bw.finalize()?;

        Ok(())
    }

    #[allow(unused)]
    fn compress_dcx_krak<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>, compression: &Box<dyn CompressionInfo>) -> io::Result<()>
    where
        W: Write + Seek
    {
        unimplemented!()
    }

    fn compress_dcp_edge<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>) -> io::Result<()>
    where
        W: Write + Seek
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
        bw.write_i32(i32::try_from(data.len()).unwrap())?;
        let compressed_size_res = bw.reserve_i32()?;
        bw.write_i32(0)?;

        let data_start = bw.position()?;
        let mut chunk_headers: Vec<EdgeChunk> = Vec::new();
        for i in 0..chunk_count {
            let mut chunk_size = 0x10000;

            if i == chunk_count - 1 && chunk_remainder > 0 {
                chunk_size = chunk_remainder;
            }

            let chunk_offset = i * 0x10000;

            let input = &data[chunk_offset..(chunk_offset + chunk_size)];
            let compressed = DeflateHelper::compress_deflate_bytes(input)?;

            let (chunk, is_compressed) = if compressed.len() < chunk_size {
                (compressed, true)
            } else {
                (input.to_vec(), false)
            };

            let comp_chunk_offset = i32::try_from(bw.position()? - data_start).unwrap();
            let comp_chunk_size = chunk.len();
            bw.write_u8_vec(chunk)?;
            bw.pad_00(0x10)?;

            chunk_headers.push(EdgeChunk{ compressed_offset: comp_chunk_offset, compressed_length: i32::try_from(comp_chunk_size).unwrap(), is_compressed: is_compressed });
        }

        let pos = bw.position()?;
        bw.fill_i32(compressed_size_res, i32::try_from(pos - data_start).unwrap())?;

        let dca_start = bw.position()?;
        bw.write_ascii("DCA", true)?;
        let dca_size_res = bw.reserve_i32()?;

        let egdt_start = bw.position()?;

        bw.write_ascii("EgdT", false)?;
        bw.write_i32(0x10000)?;
        bw.write_i32(0x20)?;
        bw.write_i32(0x10)?;
        bw.write_i32(0x10000)?;
        let egdt_size_res = bw.reserve_i32()?;
        bw.write_i32(i32::try_from(chunk_count).unwrap())?;
        bw.write_i32(0x100000)?;

        for i in chunk_headers {
            bw.write_i32(0)?;
            bw.write_i32(i.compressed_offset)?;
            bw.write_i32(i.compressed_length)?;
            match i.is_compressed {
                true => bw.write_i32(1)?,
                false => bw.write_i32(0)?
            };
        }
        let pos = bw.position()?;
        bw.fill_i32(egdt_size_res, i32::try_from(pos - egdt_start).unwrap())?;
        bw.fill_i32(dca_size_res, i32::try_from(pos - dca_start).unwrap())?;
        bw.finalize()?;

        Ok(())
    }

    fn compress_dcx_edge<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>) -> io::Result<()>
    where
        W: Write + Seek
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
        bw.write_i32(i32::try_from(0x50 + chunk_count * 0x10).unwrap())?;

        bw.write_ascii("DCS", true)?;
        bw.write_i32(i32::try_from(data.len()).unwrap())?;
        let compressed_size_res = bw.reserve_i32()?;

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
        let dca_size_res = bw.reserve_i32()?;
        let egdt_start = bw.position()?;
        bw.write_ascii("EgdT", false)?;
        bw.write_i32(0x00010100)?;
        bw.write_i32(0x24)?;
        bw.write_i32(0x10)?;
        bw.write_i32(0x10000)?;
        bw.write_i32(i32::try_from(chunk_remainder).unwrap())?;
        let egdt_size_res = bw.reserve_i32()?;
        bw.write_i32(i32::try_from(chunk_count).unwrap())?;
        bw.write_i32(0x100000)?;

        let mut chunk_size_res: HashMap<usize, Reservation> = HashMap::new();
        let mut chunk_offset_res: HashMap<usize, Reservation> = HashMap::new();
        let mut chunk_compressed_res: HashMap<usize, Reservation> = HashMap::new();

        for i in 0..chunk_count {
            bw.write_i32(0)?;
            chunk_offset_res.insert(i, bw.reserve_i32()?);
            chunk_size_res.insert(i, bw.reserve_i32()?);
            chunk_compressed_res.insert(i, bw.reserve_i32()?);
        }

        let pos = bw.position()?;
        bw.fill_i32(dca_size_res, i32::try_from(pos - dca_start).unwrap())?;
        bw.fill_i32(egdt_size_res, i32::try_from(pos - egdt_start).unwrap())?;

        let data_start = bw.position()?;

        let mut compressed_size: i32 = 0;
        for i in 0..chunk_count {
            let mut chunk_size = 0x10000;

            if i == chunk_count - 1 && chunk_remainder > 0 {
                chunk_size = chunk_remainder;
            }

            let chunk_offset = i * 0x10000;

            let input = &data[chunk_offset..(chunk_offset + chunk_size)];
            let compressed = DeflateHelper::compress_deflate_bytes(input)?;

            let (chunk, is_compressed) = if compressed.len() < chunk_size {
                (compressed, true)
            } else {
                (input.to_vec(), false)
            };

            match is_compressed {
                true => bw.fill_i32(chunk_compressed_res[&i], 1)?,
                false => bw.fill_i32(chunk_compressed_res[&i], 0)?
            };
            
            compressed_size += i32::try_from(chunk.len()).unwrap();
            let pos = bw.position()?;
            bw.fill_i32(chunk_offset_res[&i], i32::try_from(pos - data_start).unwrap())?;
            bw.fill_i32(chunk_size_res[&i], i32::try_from(chunk.len()).unwrap())?;
            bw.write_u8_vec(chunk)?;
            bw.pad_00(0x10)?;
        }

        bw.fill_i32(compressed_size_res, compressed_size)?;

        bw.finalize()?;

        Ok(())
    }

    fn compress_dcx_zstd<W>(bw: &mut BinaryWriter<W>, data: &Vec<u8>, compression: &Box<dyn CompressionInfo>) -> io::Result<()>
    where
        W: Write + Seek
    {
        let compression_level = compression.get_dcx_zstd_args()?;
        let compressed = ZstdHelper::write_zstd(data, compression_level)?;

        bw.write_ascii("DCX", true)?;
        bw.write_i32(0x11000)?;
        bw.write_i32(0x18)?;
        bw.write_i32(0x24)?;
        bw.write_i32(0x44)?;
        bw.write_i32(0x4C)?;
        bw.write_ascii("DCS", true)?;        
        bw.write_u32(u32::try_from(data.len()).unwrap())?;
        bw.write_u32(u32::try_from(compressed.len()).unwrap())?;
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
        bw.write_u8_vec(compressed)?;
        bw.pad_00(0x10)?;

        bw.finalize()?;

        Ok(())
    }
}