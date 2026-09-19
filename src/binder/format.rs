use std::io::{self, Read, Seek, Write};

use crate::io::{BinaryReader, BinaryWriter, Endian};
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Format: u8 {
        /// Minimal file information
        const None = 0;
        /// File is big-endian regardless of the big-endian byte
        const BigEndian = 0b0000_0001;
        /// Files have ID numbers
        const IDs = 0b0000_0010;
        /// Files have name strings; Names2 may or may not be set
        const Names1 = 0b0000_0100;
        /// Files have name strings; Names1 may or may not be set
        const Names2 = 0b0000_1000;
        /// File data offsets are 64-bit
        const LongOffsets = 0b0001_0000;
        /// Files may be compressed
        const Compression = 0b0010_0000;
        const Flag6 = 0b0100_0000;
        const Flag7 = 0b1000_0000;
    }
}

impl TryFrom<u8> for Format {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid format flags"))
    }
}

impl Format {
    /// Reads `Format` from `BinaryReader`
    pub(crate) fn read<R>(br: &mut BinaryReader<R>, bit_endian: Endian) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let raw = br.read_u8()?;

        let reverse = match bit_endian {
            Endian::Big => true,
            Endian::Little => false,
        } || (raw & 1) != 0 && (raw & 0b1000_0000) == 0;

        match reverse {
            true => Ok(Self::try_from(raw)?),
            false => Ok(Self::try_from(raw.reverse_bits())?),
        }
    }

    /// Writes `Format` to `BinaryWriter`
    pub(crate) fn write<W>(&self, bw: &mut BinaryWriter<W>, bit_endian: Endian) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let reverse = match bit_endian {
            Endian::Big => true,
            Endian::Little => false,
        } || (self.contains(Format::BigEndian) && self.contains(Format::Flag6));

        let raw = match reverse {
            true => self.bits(),
            false => self.bits().reverse_bits(),
        };

        bw.write_u8(raw)
    }
}

/// Calculates the size of each file header for `BND4` / `BXF4`
pub(crate) fn get_bnd4_file_header_size(format: Format) -> i64 {
    0x10 + match format.contains(Format::LongOffsets) {
        true => 8,
        false => 4,
    } + match format.contains(Format::Compression) {
        true => 8,
        false => 0,
    } + match format.contains(Format::IDs) {
        true => 4,
        false => 0,
    } + match format.contains(Format::Names1 | Format::Names2) {
        true => 4,
        false => 0,
    } + match format == Format::Names1 {
        true => 8,
        false => 0,
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FileFlags: u8 {
        /// No flags set
        const None = 0;
        /// File is compressed
        const Compressed = 0b0000_0001;
        /// Files have ID numbers
        const Flag1 = 0b0000_0010;
        const Flag2 = 0b0000_0100;
        const Flag3 = 0b0000_1000;
        const Flag4 = 0b0001_0000;
        const Flag5 = 0b0010_0000;
        const Flag6 = 0b0100_0000;
        const Flag7 = 0b1000_0000;
    }
}

impl TryFrom<u8> for FileFlags {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid format flags"))
    }
}

impl FileFlags {
    /// Reads `FileFlags` from `BinaryReader`
    pub(crate) fn read<R>(br: &mut BinaryReader<R>, bit_endian: Endian) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let raw = br.read_u8()?;
        match bit_endian {
            Endian::Big => Ok(Self::try_from(raw)?),
            Endian::Little => Ok(Self::try_from(raw.reverse_bits())?),
        }
    }

    /// Writes `FileFlags` to `BinaryWriter`
    pub(crate) fn write<W>(&self, bw: &mut BinaryWriter<W>, bit_endian: Endian) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let raw = match bit_endian {
            Endian::Big => self.bits(),
            Endian::Little => self.bits().reverse_bits(),
        };

        bw.write_u8(raw)
    }
}

#[derive(Debug, Clone, Copy)]
/// Used for writing to `BND` / `BXF` timestamp string. Implementation is sloppy on purpose, as it is not widely used
pub struct DateTime {
    pub year: u16,
    pub month: u32,
    pub day: u32,

    pub hour: u32,
    pub minute: u32,
}

impl DateTime {
    /// Converts `DateTime` to a `BND` / `BXF` timestamp string
    pub fn to_bnd_timestamp(&self) -> String {
        let mut year = self.year - 2000;

        if year > 99 {
            year = 0;
        }

        let month = char::from_u32(self.month + 'A' as u32).unwrap_or('1');
        let hour = char::from_u32(self.hour + 'A' as u32).unwrap_or('1');

        let string: String = format!("{}{}{}{}{}", year, month, self.day, hour, self.minute);

        let len = string.chars().count();

        if len >= 8 {
            string
        } else {
            let padding_count = 8 - len;
            let mut result = String::with_capacity(string.len() + padding_count);
            result.push_str(&string);
            result.extend(std::iter::repeat_n('\0', padding_count));
            result
        }
    }
}
