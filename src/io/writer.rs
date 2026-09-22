use crate::io::{ByteVector3, ByteVector4, Endian, Vector2, Vector3, Vector4};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Cursor, Seek, SeekFrom, Write};
use std::path::Path;

macro_rules! impl_numeric_writer {
    ($type:ty, $reservation_size:expr, $write:ident, $write_vec:ident, $reserve:ident, $fill:ident) => {
        #[doc = concat!("Writes `", stringify!($type), "` value")]
        pub fn $write(&mut self, data: $type) -> io::Result<()> {
            match self.endian {
                Endian::Little => self.inner.write_all(&data.to_le_bytes()),
                Endian::Big => self.inner.write_all(&data.to_be_bytes()),
            }
        }

        #[doc = concat!("Writes `Vec<", stringify!($type), ">`")]
        pub fn $write_vec(&mut self, data: Vec<$type>) -> io::Result<()> {
            for value in data {
                match self.endian {
                    Endian::Little => self.inner.write_all(&value.to_le_bytes())?,
                    Endian::Big => self.inner.write_all(&value.to_be_bytes())?,
                };
            }

            Ok(())
        }

        #[doc = concat!("Reserves space at the current position, sized as `", stringify!($type), "`")]
        pub fn $reserve(&mut self, name: impl Into<String>) -> io::Result<()> {
            self.add_reservation(name, $reservation_size)
        }

        #[doc = concat!("Fills specified reservation with a `", stringify!($type), "` value")]
        pub fn $fill(&mut self, name: impl Into<String>, value: $type) -> io::Result<()> {
            let position = self.free_reservation(name)?;

            let initial_position = self.position()?;

            self.seek(position)?;

            self.$write(value)?;

            self.seek(initial_position)?;
            Ok(())
        }
    };
}

pub struct BinaryWriter<W> {
    inner: W,
    pub endian: Endian,
    pub varint_64bit: bool,
    reservations: HashMap<String, u64>,
}

#[allow(unused)]
impl<W: Write + Seek> BinaryWriter<W> {
    /// Initializes the BinaryWriter from a generic implementing `Write + Seek`
    pub fn new(inner: W, endian: Endian, varint_64bit: bool) -> Self {
        Self {
            inner,
            endian,
            varint_64bit,
            reservations: HashMap::new(),
        }
    }

    /// Returns current stream position
    pub fn position(&mut self) -> io::Result<u64> {
        self.inner.stream_position()
    }

    /// Returns total stream length
    pub fn length(&mut self) -> io::Result<u64> {
        let initial = self.position()?;
        let length = self.inner.seek(SeekFrom::End(0))?;
        self.seek(initial)?;
        Ok(length)
    }

    /// Returns remaining length of the stream
    pub fn remaining(&mut self) -> io::Result<u64> {
        Ok(self.length()? - self.position()?)
    }

    /// Moves stream position to the target
    pub fn seek(&mut self, position: u64) -> io::Result<u64> {
        self.inner.seek(SeekFrom::Start(position))
    }

    /// Moves stream position relative to current position
    pub fn skip(&mut self, amount: i64) -> io::Result<u64> {
        self.inner.seek(SeekFrom::Current(amount))
    }

    /// Writes specified `u8` until the stream position meets the specified alignment
    pub fn pad(&mut self, align: u64, value: u8) -> io::Result<()> {
        while self.position()? % align > 0 {
            self.write_u8(value)?;
        }

        Ok(())
    }

    /// Writes `0x00` until the stream position meets the specified alignment
    pub fn pad_00(&mut self, align: u64) -> io::Result<()> {
        self.pad(align, 0x00)
    }

    /// Writes `0xFF` bytes until the stream position meets the specified alignment. BluePoint files do this
    pub fn pad_ff(&mut self, align: u64) -> io::Result<()> {
        self.pad(align, 0xFF)
    }

    /// Writes `0x00` bytes until the stream position meets the specified alignment relative to the given starting position
    pub fn pad_relative(&mut self, start: u64, align: u64) -> io::Result<()> {
        while !(self.position()? - start).is_multiple_of(align) {
            self.write_u8(0x00)?;
        }

        Ok(())
    }

    /// Adds reservation at a specified position if that position is not added to current reservation list
    fn add_reservation(&mut self, name: impl Into<String>, size: usize) -> io::Result<()> {
        let name = name.into();
        if self.reservations.contains_key(&name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "tried to reserve an already reserved name",
            ));
        }

        let position = self.position()?;

        self.write_vec_u8(vec![0xFE; size])?;

        self.reservations.insert(name, position);
        Ok(())
    }

    /// Frees a reservation from the list
    fn free_reservation(&mut self, name: impl Into<String>) -> io::Result<u64> {
        let name = name.into();
        self.reservations.remove(&name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "tried to fill an unreserved name",
            )
        })
    }

    /// Finalize the BinaryWriter - recommended to call before dropping
    pub fn finalize(&mut self) -> io::Result<()> {
        if !self.reservations.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unable to close writer - not all reservations have been filled",
            ));
        }

        Ok(())
    }

    /// Writes `bool` value
    pub fn write_bool(&mut self, value: bool) -> io::Result<()> {
        self.inner.write_all(&[value as u8])
    }

    /// Writes `Vec<bool>`
    pub fn write_vec_bool(&mut self, data: Vec<bool>) -> io::Result<()> {
        for value in data {
            self.write_bool(value)?;
        }

        Ok(())
    }

    /// Reserves space at the current position, sized as `bool`
    pub fn reserve_bool(&mut self, name: impl Into<String>) -> io::Result<()> {
        self.add_reservation(name, 1)
    }

    /// Fills specified reservation with a `bool` value
    pub fn fill_bool(&mut self, name: impl Into<String>, value: bool) -> io::Result<()> {
        let position = self.free_reservation(name)?;

        let initial_position = self.position()?;

        self.seek(position)?;

        self.write_bool(value)?;

        self.seek(initial_position)?;
        Ok(())
    }

    /// Writes `varint` value
    pub fn write_varint(&mut self, value: i64) -> io::Result<()> {
        match self.varint_64bit {
            true => self.write_i64(value)?,
            false => self.write_i32(value as i32)?,
        }

        Ok(())
    }

    /// Writes a vector of `varint` values
    pub fn write_vec_varint(&mut self, data: Vec<i64>) -> io::Result<()> {
        for value in data {
            self.write_varint(value)?;
        }

        Ok(())
    }

    /// Reserves space at the current position, sized as `varint`
    pub fn reserve_varint(&mut self, name: impl Into<String>) -> io::Result<()> {
        match self.varint_64bit {
            true => self.reserve_i64(name),
            false => self.reserve_i32(name),
        }
    }

    /// Fills specified reservation with a `varint` value
    pub fn fill_varint(&mut self, name: impl Into<String>, value: i64) -> io::Result<()> {
        match self.varint_64bit {
            true => self.fill_i64(name, value)?,
            false => self.fill_i32(name, value as i32)?,
        }

        Ok(())
    }

    fn write_chars(&mut self, terminate: bool, bytes: Vec<u8>) -> io::Result<()> {
        let mut output = bytes;
        if terminate {
            output.push(0);
        }
        self.inner.write_all(&output)
    }

    /// Writes an ASCII string, with a null terminator when requested
    pub fn write_ascii(&mut self, text: impl Into<String>, terminate: bool) -> io::Result<()> {
        let write_text = text.into();
        self.write_chars(terminate, String::from(&write_text).into_bytes())
    }

    /// Writes a Shift-JIS string, with a null terminator when requested
    pub fn write_shift_jis(&mut self, text: impl Into<String>, terminate: bool) -> io::Result<()> {
        let write_text = text.into();
        let bytes = encoding_rs::SHIFT_JIS.encode(&write_text).0.to_vec();
        self.write_chars(terminate, bytes)
    }

    /// Writes a UTF-16 string, with a null terminator when requested
    pub fn write_utf16(&mut self, text: impl Into<String>, terminate: bool) -> io::Result<()> {
        let mut bytes = Vec::new();
        let write_text = text.into();
        for code_unit in write_text.encode_utf16() {
            match self.endian {
                Endian::Little => bytes.extend_from_slice(&code_unit.to_le_bytes()),
                Endian::Big => bytes.extend_from_slice(&code_unit.to_be_bytes()),
            }
        }
        if terminate {
            match self.endian {
                Endian::Little => bytes.extend_from_slice(&0u16.to_le_bytes()),
                Endian::Big => bytes.extend_from_slice(&0u16.to_be_bytes()),
            }
        }
        self.write_vec_u8(bytes)
    }

    /// Writes a null-terminated Shift-JIS string in a fixed-size field
    pub fn write_fix_str(
        &mut self,
        text: impl Into<String>,
        size: usize,
        padding: u8,
    ) -> io::Result<()> {
        let mut fixstr = vec![padding; size];
        let mut bytes = encoding_rs::SHIFT_JIS.encode(&text.into()).0.to_vec();
        bytes.push(0);
        for (index, byte) in bytes.iter().take(size).enumerate() {
            fixstr[index] = *byte;
        }
        self.write_vec_u8(fixstr)
    }

    /// Writes a null-terminated UTF-16 string in a fixed-size field
    pub fn write_fix_str_w(
        &mut self,
        text: impl Into<String>,
        size: usize,
        padding: u8,
    ) -> io::Result<()> {
        let mut fixstr = vec![padding; size];
        let mut bytes = Vec::new();
        for code_unit in text.into().encode_utf16() {
            match self.endian {
                Endian::Little => bytes.extend_from_slice(&code_unit.to_le_bytes()),
                Endian::Big => bytes.extend_from_slice(&code_unit.to_be_bytes()),
            }
        }
        match self.endian {
            Endian::Little => bytes.extend_from_slice(&0u16.to_le_bytes()),
            Endian::Big => bytes.extend_from_slice(&0u16.to_be_bytes()),
        }
        for (index, byte) in bytes.iter().take(size).enumerate() {
            fixstr[index] = *byte;
        }
        self.inner.write_all(&fixstr)
    }

    /// Writes a `Vector2` as two `f32` numbers
    pub fn write_vector2(&mut self, vector2: Vector2) -> io::Result<()> {
        self.write_f32(vector2.x)?;
        self.write_f32(vector2.y)
    }

    /// Writes a `Vector3` as three `f32` numbers
    pub fn write_vector3(&mut self, vector3: Vector3) -> io::Result<()> {
        self.write_f32(vector3.x)?;
        self.write_f32(vector3.y)?;
        self.write_f32(vector3.z)
    }

    /// Writes a `Vector4` as four `f32` numbers
    pub fn write_vector4(&mut self, vector4: Vector4) -> io::Result<()> {
        self.write_f32(vector4.x)?;
        self.write_f32(vector4.y)?;
        self.write_f32(vector4.z)?;
        self.write_f32(vector4.w)
    }

    /// Writes a `ByteVector4` as four `u8` numbers
    pub fn write_byte_vector4(&mut self, byte_vector4: ByteVector4) -> io::Result<()> {
        self.write_u8(byte_vector4.x)?;
        self.write_u8(byte_vector4.y)?;
        self.write_u8(byte_vector4.z)?;
        self.write_u8(byte_vector4.w)
    }

    /// Writes a `ByteVector4` representing color as four `u8` numbers
    pub fn write_byte_vector4_argb(&mut self, byte_vector4: ByteVector4) -> io::Result<()> {
        self.write_u8(byte_vector4.w)?;
        self.write_u8(byte_vector4.x)?;
        self.write_u8(byte_vector4.y)?;
        self.write_u8(byte_vector4.z)
    }

    /// Writes a `ByteVector4` representing color as four `u8` numbers
    pub fn write_byte_vector4_abgr(&mut self, byte_vector4: ByteVector4) -> io::Result<()> {
        self.write_u8(byte_vector4.w)?;
        self.write_u8(byte_vector4.z)?;
        self.write_u8(byte_vector4.y)?;
        self.write_u8(byte_vector4.x)
    }

    /// Writes a `ByteVector4` representing color as four `u8` numbers
    pub fn write_byte_vector4_rgba(&mut self, byte_vector4: ByteVector4) -> io::Result<()> {
        self.write_u8(byte_vector4.x)?;
        self.write_u8(byte_vector4.y)?;
        self.write_u8(byte_vector4.z)?;
        self.write_u8(byte_vector4.w)
    }

    /// Writes a `ByteVector4` representing color as four `u8` numbers
    pub fn write_byte_vector4_bgra(&mut self, byte_vector4: ByteVector4) -> io::Result<()> {
        self.write_u8(byte_vector4.z)?;
        self.write_u8(byte_vector4.y)?;
        self.write_u8(byte_vector4.x)?;
        self.write_u8(byte_vector4.w)
    }

    /// Writes a `ByteVector3` as three `u8` numbers
    pub fn write_byte_vector3(&mut self, byte_vector3: ByteVector3) -> io::Result<()> {
        self.write_u8(byte_vector3.x)?;
        self.write_u8(byte_vector3.y)?;
        self.write_u8(byte_vector3.z)
    }

    /// Write `length` of the given `value`
    pub fn write_pattern(&mut self, length: usize, value: u8) -> io::Result<()> {
        let bytes = vec![value; length];
        self.write_vec_u8(bytes)
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.inner.write_all(bytes)
    }

    impl_numeric_writer!(u8, 1, write_u8, write_vec_u8, reserve_u8, fill_u8);
    impl_numeric_writer!(u16, 2, write_u16, write_vec_u16, reserve_u16, fill_u16);
    impl_numeric_writer!(u32, 4, write_u32, write_vec_u32, reserve_u32, fill_u32);
    impl_numeric_writer!(u64, 8, write_u64, write_vec_u64, reserve_u64, fill_u64);
    impl_numeric_writer!(i8, 1, write_i8, write_vec_i8, reserve_i8, fill_i8);
    impl_numeric_writer!(i16, 2, write_i16, write_vec_i16, reserve_i16, fill_i16);
    impl_numeric_writer!(i32, 4, write_i32, write_vec_i32, reserve_i32, fill_i32);
    impl_numeric_writer!(i64, 8, write_i64, write_vec_i64, reserve_i64, fill_i64);
    impl_numeric_writer!(f32, 4, write_f32, write_vec_f32, reserve_f32, fill_f32);
    impl_numeric_writer!(f64, 8, write_f64, write_vec_f64, reserve_f64, fill_f64);
}

#[allow(unused)]
impl BinaryWriter<Cursor<Vec<u8>>> {
    /// Initializes the `BinaryWriter` to write into a vector of bytes
    pub fn to_bytes(endian: Endian, varint_64bit: bool) -> Self {
        BinaryWriter::new(Cursor::new(Vec::new()), endian, varint_64bit)
    }

    /// Gets currently written bytes as a reference
    pub fn get_ref_bytes(&mut self) -> &Vec<u8> {
        self.inner.get_ref()
    }

    

    /// Finalizes the `BinaryWriter` and return the written bytes (the writer can still technically be used afterwards, but this should the last step of using it - followed by dropping it)
    pub fn close_bytes(&mut self) -> io::Result<Vec<u8>> {
        self.finalize()?;
        Ok(self.inner.get_ref().clone())
    }
}

#[allow(unused)]
impl BinaryWriter<File> {
    /// Initializes the `BinaryWriter`, writing to a specified file. Make sure to call `self.assert_closing()` before dropping the BinaryWriter
    pub fn to_file<P: AsRef<Path>>(
        path: P,
        endian: Endian,
        varint_64bit: bool,
    ) -> io::Result<Self> {
        Ok(BinaryWriter::new(File::create(path)?, endian, varint_64bit))
    }

    /// Gets the file that the writer is currently writing to
    pub fn get_ref_file(&self) -> &File {
        &self.inner
    }
}
