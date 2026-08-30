use std::io::{self, Read, Seek, Write};

use crate::{
    dcx::compression_info::CompressionInfo,
    io::{BinaryReader, BinaryWriter, Endian},
};

pub trait SoulsFile<T>
where
    T: SoulsFile<T>,
{
    #[allow(unused)]
    fn is<R>(br: &mut BinaryReader<R>) -> io::Result<bool> {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "\"is()\" not implemented for this format`",
        ))
    }

    fn is_bytes(bytes: &Vec<u8>) -> io::Result<bool> {
        T::is(&mut BinaryReader::from_bytes(
            bytes.to_vec(),
            Endian::Little,
            false,
        ))
    }

    fn is_file(path: &str) -> io::Result<bool> {
        T::is(&mut BinaryReader::from_file(path, Endian::Little, false)?)
    }

    #[allow(unused)]
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<T>
    where
        R: Read + Seek,
    {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "\"read()\" not implemented for this format`",
        ))
    }

    fn from_bytes(bytes: &Vec<u8>) -> io::Result<T> {
        T::read(&mut BinaryReader::from_bytes(
            bytes.to_vec(),
            Endian::Little,
            false,
        ))
    }

    fn from_file(path: &str) -> io::Result<T> {
        T::read(&mut BinaryReader::from_file(path, Endian::Little, false)?)
    }

    #[allow(unused)]
    fn write<W>(
        br: &mut BinaryWriter<W>,
        compression: Option<Box<dyn CompressionInfo>>,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "\"write()\" not implemented for this format`",
        ))
    }

    fn to_bytes(&mut self, compression: Option<Box<dyn CompressionInfo>>) -> io::Result<Vec<u8>> {
        let bw = &mut BinaryWriter::to_bytes(Endian::Little, false);
        T::write(bw, compression)?;
        bw.close_bytes()
    }

    fn to_file<W>(
        &mut self,
        path: &str,
        compression: Option<Box<dyn CompressionInfo>>,
    ) -> io::Result<()> {
        T::write(
            &mut BinaryWriter::to_file(path, Endian::Little, false)?,
            compression,
        )
    }
}
