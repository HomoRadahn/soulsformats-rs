use crate::{CompressionInfo, Dcx, io::{BinaryReader, BinaryWriter, Endian}, util};
use std::{
    fs,
    io::{self, Read, Seek, Write},
};

/// Reads and writes a format using seekable binary streams.
///
/// The read and write methods operate on the format itself, without handling an
/// outer DCX wrapper. Detection behavior depends on the implementation.
pub trait StreamIO<T>
where
    T: StreamIO<T>,
{
    /// Reads format from the provided `BinaryReader`.
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<T>
    where
        R: Read + Seek;

    /// Writes format to the provided `BinaryWriter`.
    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek;

    #[allow(unused)]
    /// Checks whether the stream appears to contain this format.
    ///
    /// The default implementation returns an `InvalidInput` error.
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

/// Reads and writes a format directly from and to files.
///
/// Reading and writing do not handle an outer DCX wrapper; use [`DcxIO`] for that.
pub trait FileIO<T>: StreamIO<T>
where
    T: FileIO<T>,
{
    /// Checks whether a file appears to contain this format, decompressing DCX if needed.
    fn is_file(path: impl Into<String>) -> io::Result<bool> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Little, false)?;
        let (mut decompressed, _) = crate::util::get_decompressed_binary_reader(&mut br)?;
        T::is(&mut decompressed)
    }

    /// Reads format from the provided file path.
    fn from_file(path: impl Into<String>) -> io::Result<T> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Little, false)?;
        T::read(&mut br)
    }

    /// Writes format to the provided file path.
    fn to_file(&self, path: impl Into<String>) -> io::Result<()> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.into_bytes()?;

        fs::write(path.into(), data)?;
        Ok(())
    }
}

/// Reads and writes a format directly from and to byte vectors.
///
/// Reading and writing do not handle an outer DCX wrapper; use [`DcxIO`] for that.
pub trait ByteIO<T>: StreamIO<T>
where
    T: ByteIO<T>,
{
    /// Checks whether bytes appear to contain this format, decompressing DCX if needed.
    fn is_bytes(data: Vec<u8>) -> io::Result<bool> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        let (mut decompressed, _) = crate::util::get_decompressed_binary_reader(&mut br)?;
        T::is(&mut decompressed)
    }

    /// Reads format from provided bytes.
    fn from_bytes(data: Vec<u8>) -> io::Result<T> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        T::read(&mut br)
    }

    /// Writes format to bytes.
    fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.into_bytes()?;

        Ok(data)
    }
}

/// Reads and writes formats with optional outer DCX compression.
///
/// Decompression methods also accept unwrapped input and return [`CompressionInfo::None`].
/// Detection recognizes DCP and DCX headers, but not bare zlib streams.
pub trait DcxIO<T>: StreamIO<T>
where 
    T: DcxIO<T>
{
    /// Decompress bytes and read decompressed data as format.
    fn decompress_bytes(data: Vec<u8>) -> io::Result<(T, CompressionInfo)> {
        let mut br = BinaryReader::from_bytes(data, Endian::Little, false);
        let (mut br, compression) = util::get_decompressed_binary_reader(&mut br)?;

        Ok((T::read(&mut br)?, compression))
    }

    /// Decompress file and read decompressed data as format.
    fn decompress_file(path: impl Into<String>) -> io::Result<(T, CompressionInfo)> {
        let mut br = BinaryReader::from_file(path.into(), Endian::Little, false)?;
        let (mut br, compression) = util::get_decompressed_binary_reader(&mut br)?;

        Ok((T::read(&mut br)?, compression))
    }

    /// Write the format and compress it to bytes.
    fn compress_to_bytes(&self, compression: CompressionInfo) -> io::Result<Vec<u8>> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.into_bytes()?;

        Dcx::new(data, compression).to_bytes()
    }

    /// Write the format and compress it to file.
    fn compress_to_file(&self, path: impl Into<String>, compression: CompressionInfo) -> io::Result<()> {
        let mut bw = BinaryWriter::to_bytes(Endian::Little, false);
        self.write(&mut bw)?;
        let data = bw.into_bytes()?;

        Dcx::new(data, compression).to_file(path)
    }
}