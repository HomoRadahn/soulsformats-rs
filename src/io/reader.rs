use crate::io::{ByteVector3, ByteVector4, Endian, Vector2, Vector3, Vector4};
use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

macro_rules! impl_generic_enum_reader {
    ($type:ty, $read_value:ident, $get_value:ident, $read:ident, $get:ident) => {
        #[doc = concat!("Reads `", stringify!($type) , "` as the specified enum, throwing an exception if not present")]
        pub fn $read<T>(&mut self) -> io::Result<T>
        where
        T: TryFrom<$type, Error = io::Error>,
        {
            T::try_from(self.$read_value()?)
        }

        #[doc = concat!("Reads `", stringify!($type) , "` as the specified enum from the specified position without advancing the stream")]
        pub fn $get<T>(&mut self, position: u64) -> io::Result<T>
        where
            T: TryFrom<$type, Error = io::Error>,
        {
            T::try_from(self.$get_value(position)?)
        }
    };
}

macro_rules! impl_numeric_reader {
    ($type:ty, $size:expr, $read:ident, $read_vec:ident, $get:ident, $get_vec:ident, $assert:ident) => {
        #[doc = concat!("Reads `", stringify!($type) , "` value")]
        pub fn $read(&mut self) -> io::Result<$type> {
            let mut buf = [0u8; $size];
            self.inner.read_exact(&mut buf)?;

            Ok(match self.endian {
                Endian::Little => <$type>::from_le_bytes(buf),
                Endian::Big => <$type>::from_be_bytes(buf),
            })
        }

        #[doc = concat!("Reads vector of `", stringify!($type) , "` values")]
        pub fn $read_vec(&mut self, count: u64) -> io::Result<Vec<$type>> {
            (0..count).map(|_| self.$read()).collect()
        }

        #[doc = concat!("Reads `", stringify!($type), "` value from the specified offset without advancing the stream")]
        pub fn $get(&mut self, position: u64) -> io::Result<$type> {
            let initial = self.position()?;
            self.seek(position)?;

            let result = self.$read();
            let restore = self.seek(initial);

            result.and_then(|value| restore.map(|_| value))
        }

        #[doc = concat!("Reads a vector of `", stringify!($type), "` values from the specified offset without advancing the stream")]
        pub fn $get_vec(&mut self, position: u64, count: u64) -> io::Result<Vec<$type>> {
            let initial = self.position()?;
            self.seek(position)?;

            let result: io::Result<Vec<$type>> =
                (0..count).map(|_| self.$read()).collect();
                let restore = self.seek(initial);

                result.and_then(|values| restore.map(|_| values))
        }

        #[doc = concat!("Reads `", stringify!($type), "` value and validates it against the supplied values")]
        pub fn $assert(&mut self, expected: &[$type]) -> io::Result<$type>
        where
            $type: PartialEq + std::fmt::Debug,
        {
            Self::assert_value(self.$read()?, expected, stringify!($type))
        }
    };
}

pub struct BinaryReader<R> {
    inner: R,
    endian: Endian,
    varint_i64: bool,
}

impl<R: Read + Seek> BinaryReader<R> {
    /// Initializes the `BinaryReader` from a generic implementing `Read + Seek`
    pub fn new(inner: R, endian: Endian, varint_i64: bool) -> Self {
        Self {
            inner,
            endian,
            varint_i64,
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

    fn assert_value<T>(value: T, expected: &[T], type_name: &str) -> io::Result<T>
    where
        T: PartialEq + std::fmt::Debug,
    {
        if expected.iter().any(|candidate| candidate == &value) {
            Ok(value)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected {type_name} value: {value:?}"),
            ))
        }
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

    /// Advances the stream position until it meets the specified alignment
    pub fn pad(&mut self, align: u64) -> io::Result<u64> {
        if align == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "padding was 0"));
        }
        let pos = self.position()?;
        if pos % align > 0 {
            self.skip((align - (pos % align)) as i64)?;
        }

        Ok(self.position()?)
    }

    /// Advances the stream position until it meets the specified alignment relative to the given starting position
    pub fn pad_relative(&mut self, start: u64, align: u64) -> io::Result<u64> {
        if align == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "padding was 0"));
        }

        let rel_pos = self.position()? - start;

        if rel_pos % align > 0 {
            self.skip((align - (rel_pos % align)) as i64)?;
        }

        Ok(self.position()?)
    }

    /// Reads `bool` value
    pub fn read_bool(&mut self) -> io::Result<bool> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;

        match buf[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "invalid bool")),
        }
    }

    /// Reads a vector of `bool` values
    pub fn read_bool_vec(&mut self, count: u64) -> io::Result<Vec<bool>> {
        (0..count).map(|_| self.read_bool()).collect()
    }

    /// Reads `bool` value and validates it against the supplied values
    pub fn assert_bool(&mut self, expected: &[bool]) -> io::Result<bool> {
        Self::assert_value(self.read_bool()?, expected, "bool")
    }

    /// Reads `bool` value from the specified offset without advancing the stream
    pub fn get_bool(&mut self, position: u64) -> io::Result<bool> {
        let initial = self.position()?;
        self.seek(position)?;

        let result = self.read_bool()?;

        self.seek(initial)?;

        Ok(result)
    }

    /// Reads a vector of `bool` values from the specified offset without advancing the stream
    pub fn get_bool_vec(&mut self, position: u64, count: u64) -> io::Result<Vec<bool>> {
        let initial = self.position()?;
        self.seek(position)?;

        let result = (0..count).map(|_| self.read_bool().unwrap()).collect();

        self.seek(initial)?;

        Ok(result)
    }

    fn read_bytes(&mut self, count: usize) -> io::Result<Vec<u8>> {
        let mut bytes = vec![0u8; count];
        self.inner.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    fn get_string<F>(&mut self, position: u64, read: F) -> io::Result<String>
    where
        F: FnOnce(&mut Self) -> io::Result<String>,
    {
        let initial = self.position()?;
        self.seek(position)?;

        let result = read(self)?;
        self.seek(initial)?;
        
        Ok(result)
    }

    fn decode_shift_jis(bytes: &[u8]) -> String {
        encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned()
    }

    fn decode_utf16(&self, bytes: &[u8]) -> io::Result<String> {
        if bytes.len() % 2 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "UTF-16 data has an odd byte length",
            ));
        }

        let code_units = bytes.chunks_exact(2).map(|pair| match self.endian {
            Endian::Little => u16::from_le_bytes([pair[0], pair[1]]),
            Endian::Big => u16::from_be_bytes([pair[0], pair[1]]),
        });
        
        String::from_utf16(&code_units.collect::<Vec<_>>())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    /// Reads a null-terminated ASCII string
    pub fn read_ascii(&mut self) -> io::Result<String> {
        let mut bytes = Vec::new();
        loop {
            let byte = self.read_u8()?;
            if byte == 0 {
                break;
            }
            bytes.push(byte);
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Reads an ASCII string with the specified byte length
    pub fn read_ascii_len(&mut self, length: usize) -> io::Result<String> {
        let bytes = self.read_bytes(length)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Reads a null-terminated ASCII string from the specified offset
    pub fn get_ascii(&mut self, position: u64) -> io::Result<String> {
        self.get_string(position, Self::read_ascii)
    }

    /// Reads an ASCII string with the specified byte length from the specified offset
    pub fn get_ascii_len(&mut self, position: u64, length: usize) -> io::Result<String> {
        self.get_string(position, |reader| reader.read_ascii_len(length))
    }

    /// Reads a null-terminated Shift-JIS string
    pub fn read_shift_jis(&mut self) -> io::Result<String> {
        let mut bytes = Vec::new();
        loop {
            let byte = self.read_u8()?;
            if byte == 0 {
                break;
            }
            bytes.push(byte);
        }
        Ok(Self::decode_shift_jis(&bytes))
    }

    /// Reads a Shift-JIS string with the specified byte length
    pub fn read_shift_jis_len(&mut self, length: usize) -> io::Result<String> {
        Ok(Self::decode_shift_jis(&self.read_bytes(length)?))
    }

    /// Reads a null-terminated Shift-JIS string from the specified offset
    pub fn get_shift_jis(&mut self, position: u64) -> io::Result<String> {
        self.get_string(position, Self::read_shift_jis)
    }

    /// Reads a Shift-JIS string with the specified byte length from the specified offset
    pub fn get_shift_jis_len(&mut self, position: u64, length: usize) -> io::Result<String> {
        self.get_string(position, |reader| reader.read_shift_jis_len(length))
    }

    /// Reads a null-terminated UTF-16 string
    pub fn read_utf16(&mut self) -> io::Result<String> {
        let mut bytes = Vec::new();
        loop {
            let pair = self.read_bytes(2)?;
            if pair == [0, 0] {
                break;
            }
            bytes.extend_from_slice(&pair);
        }
        
        self.decode_utf16(&bytes)
    }

    /// Reads a null-terminated UTF-16 string from the specified offset
    pub fn get_utf16(&mut self, position: u64) -> io::Result<String> {
        self.get_string(position, Self::read_utf16)
    }

    /// Reads a null-terminated Shift-JIS string in a fixed-size field
    pub fn read_fix_str(&mut self, size: usize) -> io::Result<String> {
        let bytes = self.read_bytes(size)?;
        let end = bytes.iter().position(|&byte| byte == 0).unwrap_or(size);
        Ok(Self::decode_shift_jis(&bytes[..end]))
    }

    /// Reads a null-terminated UTF-16 string in a fixed-size field
    pub fn read_fix_str_w(&mut self, size: usize) -> io::Result<String> {
        let bytes = self.read_bytes(size)?;
        let end = bytes
            .chunks_exact(2)
            .position(|pair| pair == [0, 0])
            .map_or(size - size % 2, |index| index * 2);
        self.decode_utf16(&bytes[..end])
    }

    /// Reads a null-terminated Shift-JIS string in a fixed-size field from the specified offset
    pub fn get_fix_str(&mut self, position: u64, size: usize) -> io::Result<String> {
        self.get_string(position, |reader| reader.read_fix_str(size))
    }

    /// Reads a null-terminated UTF-16 string in a fixed-size field from the specified offset
    pub fn get_fix_str_w(&mut self, position: u64, size: usize) -> io::Result<String> {
        self.get_string(position, |reader| reader.read_fix_str_w(size))
    }

    /// Reads ASCII bytes matching one of the supplied values
    pub fn assert_ascii(&mut self, values: &[&str]) -> io::Result<String> {
        let expected_length = values.first().map_or(0, |value| value.len());
        let value = self.read_ascii_len(expected_length)?;
        if values.iter().any(|expected| *expected == value) {
            Ok(value)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected ASCII value",
            ))
        }
    }

    impl_numeric_reader!(u8, 1, read_u8, read_u8_vec, get_u8, get_u8_vec, assert_u8);
    impl_numeric_reader!(
        u16,
        2,
        read_u16,
        read_u16_vec,
        get_u16,
        get_u16_vec,
        assert_u16
    );
    impl_numeric_reader!(
        u32,
        4,
        read_u32,
        read_u32_vec,
        get_u32,
        get_u32_vec,
        assert_u32
    );
    impl_numeric_reader!(
        u64,
        8,
        read_u64,
        read_u64_vec,
        get_u64,
        get_u64_vec,
        assert_u64
    );
    impl_numeric_reader!(i8, 1, read_i8, read_i8_vec, get_i8, get_i8_vec, assert_i8);
    impl_numeric_reader!(
        i16,
        2,
        read_i16,
        read_i16_vec,
        get_i16,
        get_i16_vec,
        assert_i16
    );
    impl_numeric_reader!(
        i32,
        4,
        read_i32,
        read_i32_vec,
        get_i32,
        get_i32_vec,
        assert_i32
    );
    impl_numeric_reader!(
        i64,
        8,
        read_i64,
        read_i64_vec,
        get_i64,
        get_i64_vec,
        assert_i64
    );
    impl_numeric_reader!(
        f32,
        4,
        read_f32,
        read_f32_vec,
        get_f32,
        get_f32_vec,
        assert_f32
    );
    impl_numeric_reader!(
        f64,
        8,
        read_f64,
        read_f64_vec,
        get_f64,
        get_f64_vec,
        assert_f64
    );
    impl_generic_enum_reader!(u8, read_u8, get_u8, read_enum8, get_enum8);
    impl_generic_enum_reader!(u16, read_u16, get_u16, read_enum_u16, get_enum_u16);
    impl_generic_enum_reader!(u32, read_u32, get_u32, read_enum_u32, get_enum_u32);
    impl_generic_enum_reader!(u64, read_u64, get_u64, read_enum_u64, get_enum_u64);
    impl_generic_enum_reader!(i8, read_i8, get_i8, read_enum_i8, get_enum_i8);
    impl_generic_enum_reader!(i16, read_i16, get_i16, read_enum_i16, get_enum_i16);
    impl_generic_enum_reader!(i32, read_i32, get_i32, read_enum_i32, get_enum_i32);
    impl_generic_enum_reader!(i64, read_i64, get_i64, read_enum_i64, get_enum_i64);

    /// Reads either `i32` or `i64` depending on `varint_i64`
    pub fn read_varint(&mut self) -> io::Result<i64> {
        if self.varint_i64 {
            self.read_i64()
        } else {
            self.read_i32().map(i64::from)
        }
    }

    /// Reads a vector of either `i32` or `i64` depending on `varint_i64`
    pub fn read_varint_vec(&mut self, count: u64) -> io::Result<Vec<i64>> {
        (0..count).map(|_| self.read_varint()).collect()
    }

    /// Reads a `varint` value and validates it against the supplied values
    pub fn assert_varint(&mut self, expected: &[i64]) -> io::Result<i64> {
        Self::assert_value(self.read_varint()?, expected, "varint")
    }

    /// Reads either `i32` or `i64` depending on `varint_i64` from the specified position without advancing the stream
    pub fn get_varint(&mut self, position: u64) -> io::Result<i64> {
        if self.varint_i64 {
            self.get_i64(position)
        } else {
            self.get_i32(position).map(i64::from)
        }
    }

    /// Reads a vector of either `i32` or `i64` depending on `varint_i64` from the specified position without advancing the stream
    pub fn get_varint_vec(&mut self, position: u64, count: u64) -> io::Result<Vec<i64>> {
        let initial = self.position()?;
        self.seek(position)?;

        let result: io::Result<Vec<i64>> = (0..count).map(|_| self.read_varint()).collect();
        let restore = self.seek(initial);

        result.and_then(|values| restore.map(|_| values))
    }

    /// Reads a `Vector2` value from two consecutive `f32` values
    pub fn read_vector_2(&mut self) -> io::Result<Vector2> {
        Ok(Vector2 {
            x: self.read_f32()?,
            y: self.read_f32()?,
        })
    }

    /// Reads a `Vector3` value from three consecutive `f32` values
    pub fn read_vector_3(&mut self) -> io::Result<Vector3> {
        Ok(Vector3 {
            x: self.read_f32()?,
            y: self.read_f32()?,
            z: self.read_f32()?,
        })
    }

    /// Reads a `Vector4` value from four consecutive `f32` values
    pub fn read_vector_4(&mut self) -> io::Result<Vector4> {
        Ok(Vector4 {
            x: self.read_f32()?,
            y: self.read_f32()?,
            z: self.read_f32()?,
            w: self.read_f32()?,
        })
    }

    /// Reads a `ByteVector4` value from four consecutive `u8` values
    pub fn read_byte_vector_4(&mut self) -> io::Result<ByteVector4> {
        Ok(ByteVector4 {
            x: self.read_u8()?,
            y: self.read_u8()?,
            z: self.read_u8()?,
            w: self.read_u8()?,
        })
    }

    /// Reads a `ByteVector4` - used to represent color
    pub fn read_byte_vector_4_argb(&mut self) -> io::Result<ByteVector4> {
        let bytes = self.read_u8_vec(4)?;
        Ok(ByteVector4 {
            x: bytes[1],
            y: bytes[2],
            z: bytes[3],
            w: bytes[0],
        })
    }

    /// Reads a `ByteVector4` - used to represent color
    pub fn read_byte_vector_4_abgr(&mut self) -> io::Result<ByteVector4> {
        let bytes = self.read_u8_vec(4)?;
        Ok(ByteVector4 {
            x: bytes[3],
            y: bytes[2],
            z: bytes[1],
            w: bytes[0],
        })
    }

    /// Reads a `ByteVector4` - used to represent color
    pub fn read_byte_vector_4_rgba(&mut self) -> io::Result<ByteVector4> {
        let bytes = self.read_u8_vec(4)?;
        Ok(ByteVector4 {
            x: bytes[0],
            y: bytes[1],
            z: bytes[2],
            w: bytes[3],
        })
    }

    /// Reads a `ByteVector4` - used to represent color
    pub fn read_byte_vector_4_bgra(&mut self) -> io::Result<ByteVector4> {
        let bytes = self.read_u8_vec(4)?;
        Ok(ByteVector4 {
            x: bytes[2],
            y: bytes[1],
            z: bytes[0],
            w: bytes[3],
        })
    }

    /// Reads a `ByteVector3` value from three consecutive `u8` values
    pub fn read_byte_vector_3(&mut self) -> io::Result<ByteVector3> {
        Ok(ByteVector3 {
            x: self.read_u8()?,
            y: self.read_u8()?,
            z: self.read_u8()?,
        })
    }

    /// Read specified a `length` of bytes against a specified `pattern` - all bytes must match the `pattern`
    pub fn assert_pattern(&mut self, length: u64, pattern: u8) -> io::Result<()> {
        let bytes = self.read_u8_vec(length)?;

        if !bytes.iter().all(|&byte| byte == pattern) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed assertion of: {}", pattern),
            ));
        }

        Ok(())
    }
}

impl BinaryReader<Cursor<Vec<u8>>> {
    /// Initializes the `BinaryReader` from a vector of bytes
    pub fn from_bytes(bytes: Vec<u8>, endian: Endian, varint_i64: bool) -> Self {
        BinaryReader::new(Cursor::new(bytes), endian, varint_i64)
    }
}

impl BinaryReader<File> {
    /// Initializes the `BinaryReader` from a file
    pub fn from_file<P: AsRef<Path>>(
        path: P,
        endian: Endian,
        varint_i64: bool,
    ) -> io::Result<Self> {
        Ok(BinaryReader::new(File::open(path)?, endian, varint_i64))
    }
}
