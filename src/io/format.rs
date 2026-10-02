use crate::{CompressionInfo, Dcx, io::{BinaryReader, BinaryWriter, Endian}, util};
use std::{
    fs,
    io::{self, Read, Seek, Write},
};

/// Trait for reading and writing to `BinaryReader` and `BinaryWriter`
pub trait StreamIO<T>
where
    T: StreamIO<T>,
{
    /// Reads format from the provided `BinaryReader`
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<T>
    where
        R: Read + Seek;

    /// Writes format to the provided `BinaryWriter`
    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek;

    #[allow(unused)]
    /// Check if the `BinaryReader` appears to contain a valid format
    fn is<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek,
    {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Is function not implemented for this format",
        ))
    }
}

/// Trait allowing for reading and writing to files. Requires `StreamIO`
pub trait FileIO<T>: StreamIO<T>
where
    T: FileIO<T>,
{
    /// Check if the file path appears to contain a valid format
    fn is_file(path: impl Into<String>) -> io::Result<bool> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Little, false)?;
        let (mut decompressed, _) = crate::util::get_decompressed_binary_reader(&mut br)?;
        T::is(&mut decompressed)
    }

    /// Reads format from the provided file path
    fn from_file(path: impl Into<String>) -> io::Result<T> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Little, false)?;
        T::read(&mut br)
    }

    /// Writes format to the provided file path
    fn to_file(&self, path: impl Into<String>) -> io::Result<()> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.close_bytes()?;

        fs::write(path.into(), data)?;
        Ok(())
    }
}

/// Trait allowing for reading and writing to bytes. Requires `StreamIO`
pub trait ByteIO<T>: StreamIO<T>
where
    T: ByteIO<T>,
{
    /// Check if the bytes appear to contain a valid format
    fn is_bytes(data: Vec<u8>) -> io::Result<bool> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        let (mut decompressed, _) = crate::util::get_decompressed_binary_reader(&mut br)?;
        T::is(&mut decompressed)
    }

    /// Reads format from provided bytes
    fn from_bytes(data: Vec<u8>) -> io::Result<T> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        T::read(&mut br)
    }

    /// Writes format to bytes
    fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.close_bytes()?;

        Ok(data)
    }
}

/// Trait for automatic `Dcx` compression handling when reading / writing formats
pub trait DcxIO<T>: StreamIO<T>
where 
    T: DcxIO<T>
{
    /// Decompress bytes and read decompressed data as format
    fn decompress_bytes(data: Vec<u8>) -> io::Result<(T, CompressionInfo)> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        let (mut br, compression) = util::get_decompressed_binary_reader(&mut br)?;

        Ok((T::read(&mut br)?, compression))
    }

    /// Decompress file and read decompressed data as format
    fn decompress_file(path: impl Into<String>) -> io::Result<(T, CompressionInfo)> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Little, false)?;
        let (mut br, compression) = util::get_decompressed_binary_reader(&mut br)?;

        Ok((T::read(&mut br)?, compression))
    }

    /// Write the format and compress it to bytes
    fn compress_to_bytes(&self, compression: CompressionInfo) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.close_bytes()?;

        Dcx::new(data, compression).to_bytes()
    }

    /// Write the format and compress it to file
    fn compress_to_file(&self, path: impl Into<String>, compression: CompressionInfo) -> io::Result<()> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.close_bytes()?;

        Dcx::new(data, compression).to_file(path)
    }
}