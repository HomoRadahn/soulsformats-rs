use crate::{
    DCX,
    dcx::compression_info::{CompressionInfo, Type},
    io::{BinaryReader, BinaryWriter, Endian},
};
use std::{
    fs,
    io::{self, Read, Seek, Write},
};

pub trait SoulsFile<T>
where
    T: SoulsFile<T>,
{
    fn get_compression(&self) -> Box<dyn CompressionInfo>;
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<T>
    where
        R: Read + Seek;

    fn from_bytes(data: Vec<u8>) -> io::Result<T> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        T::read(&mut br)
    }

    fn from_file(path: &str) -> io::Result<T> {
        let mut br = BinaryReader::from_file(path, Endian::Little, false)?;
        T::read(&mut br)
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek;

    fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        if self.get_compression().get_type() == Type::None {
            return Ok(bw.close_bytes()?);
        }

        let dcx = DCX::new(bw.close_bytes()?, self.get_compression().box_clone());
        return dcx.compress_to_bytes();
    }

    fn to_file(&self, path: &str) -> io::Result<()> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        if self.get_compression().get_type() == Type::None {
            fs::write(path, bw.close_bytes()?)?;
        }

        let dcx = DCX::new(bw.close_bytes()?, self.get_compression().box_clone());
        return dcx.compress_to_file(path);
    }
}
