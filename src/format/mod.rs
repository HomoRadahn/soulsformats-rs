use crate::{
    io::{BinaryReader, BinaryWriter, Endian},
};
use std::{
    fs, io::{self, Read, Seek, Write},
};

pub(crate) trait SoulsFileInternal<T>
where
    T: SoulsFileInternal<T>
{
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<T>
    where
        R: Read + Seek;

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek;

    #[allow(unused)]
    fn is<R>(br: &mut BinaryReader<R>) -> io::Result<bool>
    where
        R: Read + Seek
    {
        Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Is function not implemented for type: {}", stringify!(T))))
    }
}

/// Trait allowing for reading and writing to bytes / files. Not for actual implementation on your own structs
#[allow(private_bounds)]
pub trait SoulsFile<T>: SoulsFileInternal<T>
where
    T: SoulsFile<T>,
{
    /// Reads the format from bytes
    fn from_bytes(data: Vec<u8>) -> io::Result<T> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        T::read(&mut br)
    }

    /// Reads the format from a file
    fn from_file(path: impl Into<String>) -> io::Result<T> {
        let path_into = path.into();
        let mut br = BinaryReader::from_file(path_into, Endian::Little, false)?;
        T::read(&mut br)
    }

    /// Checks if the bytes appear to be the format
    fn is_bytes(data: Vec<u8>) -> io::Result<bool> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        T::is(&mut br)
    }

    /// Checks if the file appears to be the format
    fn is_file(path: impl Into<String>) -> io::Result<bool> {
        let path_into = path.into();
        let mut br = BinaryReader::from_file(path_into, Endian::Little, false)?;
        T::is(&mut br)
    }

    /// Writes the format to bytes
    fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.close_bytes()?;
        
        return Ok(data);
    }

    /// Writes the format to a file
    fn to_file(&self, path: impl Into<String>) -> io::Result<()> {
        let path_into = path.into();
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.close_bytes()?;

        fs::write(path_into, data)?;
        return Ok(());
    }
}
