use crate::{
    DCX,
    dcx::compression_info::{CompressionInfo},
    io::{BinaryReader, BinaryWriter, Endian},
};
use std::{
    fs,
    io::{self, Read, Seek, Write},
};

pub(crate) trait SoulsFileInternal<T>
where
    T: SoulsFileInternal<T>
{
    fn get_compression(&self) -> CompressionInfo;
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<T>
    where
        R: Read + Seek;

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek;
}

/// Trait allowing for reading and writing to bytes / files. Not for actual implementation on your own structs
#[allow(private_bounds)]
pub trait SoulsFile<T>: SoulsFileInternal<T>
where
    T: SoulsFile<T>,
{
    fn from_bytes(data: Vec<u8>) -> io::Result<T> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        T::read(&mut br)
    }

    fn from_file(path: &str) -> io::Result<T> {
        let mut br = BinaryReader::from_file(path, Endian::Little, false)?;
        T::read(&mut br)
    }

    fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.close_bytes()?;
        
        return Ok(data);
    }

    fn to_file(&self, path: &str) -> io::Result<()> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.close_bytes()?;

        fs::write(path, data)?;
        return Ok(());
    }
}
