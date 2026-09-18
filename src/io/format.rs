use crate::{
    DCX,
    dcx::compression_info::CompressionInfo,
    io::{BinaryReader, BinaryWriter, Endian},
};
use std::{
    fs,
    io::{self, Read, Seek, Write},
};

pub trait StreamIO<T>
where
    T: StreamIO<T>,
{
    fn get_compression(&self) -> CompressionInfo;
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<T>
    where
        R: Read + Seek;

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek;

    #[allow(unused)]
    fn is<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek,
    {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Is function not implemented for type: {}", stringify!(T)),
        ))
    }
}

/// Enable for reading / writing formats to files. Not for implementation in own projects
#[allow(private_bounds)]
pub trait FileIO<T>: StreamIO<T>
where
    T: FileIO<T>,
{
    /// Checks if the file appears to be the format
    fn is_file(path: impl Into<String>) -> io::Result<bool> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Little, false)?;
        let (mut decompressed, _) = crate::util::get_decompressed_binary_reader(&mut br)?;
        T::is(&mut decompressed)
    }

    /// Reads the format from a file
    fn from_file(path: impl Into<String>) -> io::Result<T> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Little, false)?;
        T::read(&mut br)
    }

    /// Writes the format to a file
    fn to_file(&self, path: impl Into<String>) -> io::Result<()> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.close_bytes()?;

        let dcx = DCX::new(data, self.get_compression());
        let out = dcx.compress_to_bytes()?;
        fs::write(path.into(), out)?;

        Ok(())
    }
}

/// Trait allowing for reading and writing to bytes / files. Not for actual implementation on your own structs
#[allow(private_bounds)]
pub trait ByteIO<T>: StreamIO<T>
where
    T: ByteIO<T>,
{
    /// Checks if the bytes appear to be the format
    fn is_bytes(data: Vec<u8>) -> io::Result<bool> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        let (mut decompressed, _) = crate::util::get_decompressed_binary_reader(&mut br)?;
        T::is(&mut decompressed)
    }

    /// Reads the format from bytes
    fn from_bytes(data: Vec<u8>) -> io::Result<T> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        T::read(&mut br)
    }

    /// Writes the format to bytes
    fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.close_bytes()?;

        let dcx = DCX::new(data, self.get_compression());
        let out = dcx.compress_to_bytes()?;

        Ok(out)
    }
}
