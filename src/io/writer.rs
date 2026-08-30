use crate::io::{ByteVector3, ByteVector4, Endian, Vector2, Vector3, Vector4};
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

        #[doc = concat!("Writes a vector of `", stringify!($type), "` values")]
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
        pub fn $reserve(&mut self) -> io::Result<Reservation> {
            self.add_reservation($reservation_size)
        }

        #[doc = concat!("Fills specified reservation with a `", stringify!($type), "` value")]
        pub fn $fill(&mut self, reservation: Reservation, value: $type) -> io::Result<()> {
            self.free_reservation(reservation)?;

            let initial_position = self.position()?;

            self.seek(reservation.position)?;

            self.$write(value)?;

            self.seek(initial_position)?;
            Ok(())
        }
    };
}

pub struct BinaryWriter<W> {
    inner: W,
    endian: Endian,
    varint_i64: bool,
    reservations: Vec<Reservation>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Reservation {
    position: u64,
}

impl<W: Write + Seek> BinaryWriter<W> {
    /// Initializes the BinaryWriter from a generic implementing `Write + Seek`
    pub fn new(inner: W, endian: Endian, varint_i64: bool) -> Self {
        Self {
            inner,
            endian,
            varint_i64,
            reservations: Vec::new(),
        }
    }

    /// Sets endianness of the stream
    pub fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }

    /// Sets `varint_i64` to `behavior`
    pub fn set_varint_behavior(&mut self, behavior: bool) {
        self.varint_i64 = behavior;
    }

    /// Returns current stream position
    pub fn position(&mut self) -> io::Result<u64> {
        return self.inner.stream_position();
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
    pub fn skip(&mut self, position: i64) -> io::Result<u64> {
        self.inner.seek(SeekFrom::Current(position))
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
        while (self.position()? - start) % align > 0 {
            self.write_u8(0x00)?;
        }

        Ok(())
    }

    /// Adds reservation at a specified position if that position is not added to current reservation list
    fn add_reservation(&mut self, size: usize) -> io::Result<Reservation> {
        let position = self.position()?;
        for reservation in &self.reservations {
            if reservation.position == position {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "tried to reserve already reserved position",
                ));
            }
        }

        self.write_u8_vec(vec![0xFE; size])?;

        self.reservations.push(Reservation { position });

        Ok(Reservation { position })
    }

    /// Frees a reservation from the list
    fn free_reservation(&mut self, reservation: Reservation) -> io::Result<()> {
        if !self.reservations.contains(&reservation) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "tried to fill unreserved position",
            ));
        }

        if let Some(pos) = self.reservations.iter().position(|x| *x == reservation) {
            self.reservations.remove(pos);
        }

        Ok(())
    }

    /// Finalize the BinaryWriter - recommended to call before dropping
    pub fn finalize(&mut self) -> io::Result<()> {
        if self.reservations.len() != 0 {
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

    /// Writes a vector of `bool` values
    pub fn write_bool_vec(&mut self, data: Vec<bool>) -> io::Result<()> {
        for value in data {
            self.write_bool(value)?;
        }

        Ok(())
    }

    /// Reserves space at the current position, sized as `bool`
    pub fn reserve_bool(&mut self) -> io::Result<Reservation> {
        self.add_reservation(1)
    }

    /// Fills specified reservation with a `bool` value
    pub fn fill_bool(&mut self, reservation: Reservation, value: bool) -> io::Result<()> {
        self.free_reservation(reservation)?;

        let initial_position = self.position()?;

        self.seek(reservation.position)?;

        self.write_bool(value)?;

        self.seek(initial_position)?;
        Ok(())
    }

    /// Writes `varint` value
    pub fn write_varint(&mut self, value: i64) -> io::Result<()> {
        match self.varint_i64 {
            true => self.write_i64(value)?,
            false => self.write_i32(value as i32)?,
        }

        Ok(())
    }

    /// Writes a vector of `varint` value
    pub fn write_varint_vec(&mut self, data: Vec<i64>) -> io::Result<()> {
        for value in data {
            self.write_varint(value)?;
        }

        Ok(())
    }

    /// Reserves space at the current position, sized as `varint`
    pub fn reserve_varint(&mut self) -> io::Result<Reservation> {
        match self.varint_i64 {
            true => self.reserve_i64(),
            false => self.reserve_i32(),
        }
    }

    /// Fills specified reservation with a `varint` value
    pub fn fill_varint(&mut self, reservation: Reservation, value: i64) -> io::Result<()> {
        match self.varint_i64 {
            true => self.fill_i64(reservation, value)?,
            false => self.fill_i32(reservation, value as i32)?,
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
    pub fn write_ascii(&mut self, text: &str, terminate: bool) -> io::Result<()> {
        self.write_chars(terminate, String::from(text).into_bytes())
    }

    /// Writes a Shift-JIS string, with a null terminator when requested
    pub fn write_shift_jis(&mut self, text: &str, terminate: bool) -> io::Result<()> {
        let bytes = encoding_rs::SHIFT_JIS.encode(text).0.to_vec();
        self.write_chars(terminate, bytes)
    }

    /// Writes a UTF-16 string, with a null terminator when requested
    pub fn write_utf16(&mut self, text: &str, terminate: bool) -> io::Result<()> {
        let mut bytes = Vec::new();
        for code_unit in text.encode_utf16() {
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
        self.write_u8_vec(bytes)
    }

    /// Writes a null-terminated Shift-JIS string in a fixed-size field
    pub fn write_fix_str(&mut self, text: &str, size: usize, padding: u8) -> io::Result<()> {
        let mut fixstr = vec![padding; size];
        let mut bytes = encoding_rs::SHIFT_JIS.encode(text).0.to_vec();
        bytes.push(0);
        for (index, byte) in bytes.iter().take(size).enumerate() {
            fixstr[index] = *byte;
        }
        self.write_u8_vec(fixstr)
    }

    /// Writes a null-terminated UTF-16 string in a fixed-size field
    pub fn write_fix_str_w(&mut self, text: &str, size: usize, padding: u8) -> io::Result<()> {
        let mut fixstr = vec![padding; size];
        let mut bytes = Vec::new();
        for code_unit in text.encode_utf16() {
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
    pub fn write_byte_vector3(&mut self, byte_vector4: ByteVector3) -> io::Result<()> {
        self.write_u8(byte_vector4.x)?;
        self.write_u8(byte_vector4.y)?;
        self.write_u8(byte_vector4.z)
    }

    /// Write `length` of the given `value`
    pub fn write_pattern(&mut self, length: usize, value: u8) -> io::Result<()> {
        let bytes = vec![value; length];
        self.write_u8_vec(bytes)
    }

    impl_numeric_writer!(u8, 1, write_u8, write_u8_vec, reserve_u8, fill_u8);
    impl_numeric_writer!(u16, 2, write_u16, write_u16_vec, reserve_u16, fill_u16);
    impl_numeric_writer!(u32, 4, write_u32, write_u32_vec, reserve_u32, fill_u32);
    impl_numeric_writer!(u64, 8, write_u64, write_u64_vec, reserve_u64, fill_u64);
    impl_numeric_writer!(i8, 1, write_i8, write_i8_vec, reserve_i8, fill_i8);
    impl_numeric_writer!(i16, 2, write_i16, write_i16_vec, reserve_i16, fill_i16);
    impl_numeric_writer!(i32, 4, write_i32, write_i32_vec, reserve_i32, fill_i32);
    impl_numeric_writer!(i64, 8, write_i64, write_i64_vec, reserve_i64, fill_i64);
    impl_numeric_writer!(f32, 4, write_f32, write_f32_vec, reserve_f32, fill_f32);
    impl_numeric_writer!(f64, 8, write_f64, write_f64_vec, reserve_f64, fill_f64);
}

impl BinaryWriter<Cursor<Vec<u8>>> {
    /// Initializes the `BinaryWriter` to write into a vector of bytes
    pub fn to_bytes(endian: Endian, varint_i64: bool) -> Self {
        BinaryWriter::new(Cursor::new(Vec::new()), endian, varint_i64)
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

impl BinaryWriter<File> {
    /// Initializes the `BinaryWriter`, writing to a specified file. Make sure to call `self.assert_closing()` before dropping the BinaryWriter
    pub fn to_file<P: AsRef<Path>>(path: P, endian: Endian, varint_i64: bool) -> io::Result<Self> {
        Ok(BinaryWriter::new(File::create(path)?, endian, varint_i64))
    }

    /// Gets the file that the writer is currently writing to
    pub fn get_ref_file(&self) -> &File {
        &self.inner
    }
}
