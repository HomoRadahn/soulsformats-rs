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

pub(crate) struct BinaryReader<R> {
    inner: R,
    endian: Endian,
    varint_64bit: bool,
}

#[allow(unused)]
impl<R: Read + Seek> BinaryReader<R> {
    /// Initializes the `BinaryReader` from a generic implementing `Read + Seek`
    pub fn new(inner: R, endian: Endian, varint_64bit: bool) -> Self {
        Self {
            inner,
            endian,
            varint_64bit,
        }
    }

    /// Sets endianness of the stream
    pub fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }

    /// Sets `varint_64bit` to `behavior`
    pub fn set_varint_64bit(&mut self, behavior: bool) {
        self.varint_64bit = behavior;
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

    /// Reads either `i32` or `i64` depending on `varint_64bit`
    pub fn read_varint(&mut self) -> io::Result<i64> {
        if self.varint_64bit {
            self.read_i64()
        } else {
            self.read_i32().map(i64::from)
        }
    }

    /// Reads a vector of either `i32` or `i64` depending on `varint_64bit`
    pub fn read_varint_vec(&mut self, count: u64) -> io::Result<Vec<i64>> {
        (0..count).map(|_| self.read_varint()).collect()
    }

    /// Reads a `varint` value and validates it against the supplied values
    pub fn assert_varint(&mut self, expected: &[i64]) -> io::Result<i64> {
        Self::assert_value(self.read_varint()?, expected, "varint")
    }

    /// Reads either `i32` or `i64` depending on `varint_64bit` from the specified position without advancing the stream
    pub fn get_varint(&mut self, position: u64) -> io::Result<i64> {
        if self.varint_64bit {
            self.get_i64(position)
        } else {
            self.get_i32(position).map(i64::from)
        }
    }

    /// Reads a vector of either `i32` or `i64` depending on `varint_64bit` from the specified position without advancing the stream
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
    pub fn from_bytes(bytes: Vec<u8>, endian: Endian, varint_64bit: bool) -> Self {
        BinaryReader::new(Cursor::new(bytes), endian, varint_64bit)
    }
}

impl BinaryReader<File> {
    /// Initializes the `BinaryReader` from a file
    pub fn from_file<P: AsRef<Path>>(
        path: P,
        endian: Endian,
        varint_64bit: bool,
    ) -> io::Result<Self> {
        Ok(BinaryReader::new(File::open(path)?, endian, varint_64bit))
    }
}

#[cfg(test)]
mod tests {
    use crate::io::{BinaryReader, Endian};
    use std::{fs, io, vec};

    macro_rules! define_enum_fixture {
        ($name:ident, $type:ty) => {
            #[repr($type)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            enum $name {
                Zero = 0,
                One = 1,
            }

            impl TryFrom<$type> for $name {
                type Error = io::Error;

                fn try_from(value: $type) -> Result<Self, Self::Error> {
                    match value {
                        0 => Ok(Self::Zero),
                        1 => Ok(Self::One),
                        _ => Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "invalid enum value",
                        )),
                    }
                }
            }
        };
    }

    define_enum_fixture!(EnumU8, u8);
    define_enum_fixture!(EnumU16, u16);
    define_enum_fixture!(EnumU32, u32);
    define_enum_fixture!(EnumU64, u64);
    define_enum_fixture!(EnumI8, i8);
    define_enum_fixture!(EnumI16, i16);
    define_enum_fixture!(EnumI32, i32);
    define_enum_fixture!(EnumI64, i64);

    #[test]
    fn read_boolean() {
        for endian in [Endian::Little, Endian::Big] {
            let byte_true = vec![0x1];
            let byte_false = vec![0x0];
            let mut reader_true = BinaryReader::from_bytes(byte_true, endian, false);
            let mut reader_false = BinaryReader::from_bytes(byte_false, endian, false);
            assert!(reader_true.read_bool().unwrap());
            assert!(!reader_false.read_bool().unwrap());

            let bytes = vec![0x1, 0x0, 0x1];
            let mut vec_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(
                vec_reader.read_bool_vec(3).unwrap(),
                vec![true, false, true]
            );

            let mut get_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert!(!get_reader.get_bool(1).unwrap());
            assert_eq!(0, get_reader.position().unwrap());
            assert_eq!(get_reader.get_bool_vec(1, 2).unwrap(), vec![false, true]);
            assert_eq!(0, get_reader.position().unwrap());
        }
    }

    #[test]
    fn read_from_file() {
        let path =
            std::env::temp_dir().join(format!("soulsformats-rs-reader-{}.bin", std::process::id()));
        fs::write(&path, [0x12, 0x13]).unwrap();

        let mut reader = BinaryReader::from_file(&path, Endian::Little, true).unwrap();
        assert_eq!(reader.read_u16().unwrap(), 0x1312);
        assert_eq!(reader.position().unwrap(), 2);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn read_uint8() {
        for endian in [Endian::Little, Endian::Big] {
            let byte = vec![0x12];

            let mut reader = BinaryReader::from_bytes(byte, endian, true);
            assert_eq!(reader.read_u8().unwrap(), 0x12);

            let bytes = vec![0x12, 0x13, 0x14];
            let mut vec_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(vec_reader.read_u8_vec(3).unwrap(), vec![0x12, 0x13, 0x14]);

            let mut get_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(get_reader.get_u8(2).unwrap(), 0x14);
            assert_eq!(0, get_reader.position().unwrap());
            assert_eq!(get_reader.get_u8_vec(1, 2).unwrap(), vec![0x13, 0x14]);
            assert_eq!(0, get_reader.position().unwrap());
        }
    }

    #[test]
    fn read_uint16() {
        let single = vec![0x12, 0x13];
        let many = vec![0x12, 0x13, 0x14, 0x15, 0x16, 0x17];

        let mut little_reader = BinaryReader::from_bytes(single.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_u16().unwrap(), 0x1312);

        let mut little_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(
            little_vec_reader.read_u16_vec(2).unwrap(),
            vec![0x1312, 0x1514]
        );

        let mut little_get_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_u16(2).unwrap(), 0x1514);
        assert_eq!(0, little_get_reader.position().unwrap());
        assert_eq!(
            little_get_reader.get_u16_vec(1, 2).unwrap(),
            vec![0x1413, 0x1615]
        );
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(single.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_u16().unwrap(), 0x1213);

        let mut big_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Big, true);
        assert_eq!(
            big_vec_reader.read_u16_vec(2).unwrap(),
            vec![0x1213, 0x1415]
        );

        let mut big_get_reader = BinaryReader::from_bytes(many.clone(), Endian::Big, true);
        assert_eq!(big_get_reader.get_u16(2).unwrap(), 0x1415);
        assert_eq!(0, big_get_reader.position().unwrap());
        assert_eq!(
            big_get_reader.get_u16_vec(1, 2).unwrap(),
            vec![0x1314, 0x1516]
        );
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_uint32() {
        let single = vec![0x12, 0x13, 0x14, 0x15];
        let many = vec![
            0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x20, 0x21, 0x22, 0x23,
        ];

        let mut little_reader = BinaryReader::from_bytes(single.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_u32().unwrap(), 0x15141312);

        let mut little_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(
            little_vec_reader.read_u32_vec(2).unwrap(),
            vec![0x15141312, 0x19181716]
        );

        let mut little_get_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_u32(2).unwrap(), 0x17161514);
        assert_eq!(0, little_get_reader.position().unwrap());
        assert_eq!(
            little_get_reader.get_u32_vec(1, 2).unwrap(),
            vec![0x16151413, 0x20191817]
        );
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(single.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_u32().unwrap(), 0x12131415);

        let mut big_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Big, true);
        assert_eq!(
            big_vec_reader.read_u32_vec(2).unwrap(),
            vec![0x12131415, 0x16171819]
        );

        let mut big_get_reader = BinaryReader::from_bytes(many.clone(), Endian::Big, true);
        assert_eq!(big_get_reader.get_u32(2).unwrap(), 0x14151617);
        assert_eq!(0, big_get_reader.position().unwrap());
        assert_eq!(
            big_get_reader.get_u32_vec(1, 2).unwrap(),
            vec![0x13141516, 0x17181920]
        );
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_uint64() {
        let single = vec![0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19];
        let many = vec![
            0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26,
            0x27,
        ];

        let mut little_reader = BinaryReader::from_bytes(single.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_u64().unwrap(), 0x1918171615141312);

        let mut little_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(
            little_vec_reader.read_u64_vec(2).unwrap(),
            vec![0x1918171615141312, 0x2726252423222120]
        );

        let mut little_get_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_u64(8).unwrap(), 0x2726252423222120);
        assert_eq!(0, little_get_reader.position().unwrap());
        assert_eq!(
            little_get_reader.get_u64_vec(0, 2).unwrap(),
            vec![0x1918171615141312, 0x2726252423222120]
        );
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(single.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_u64().unwrap(), 0x1213141516171819);

        let mut big_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Big, true);
        assert_eq!(
            big_vec_reader.read_u64_vec(2).unwrap(),
            vec![0x1213141516171819, 0x2021222324252627]
        );

        let mut big_get_reader = BinaryReader::from_bytes(many, Endian::Big, true);
        assert_eq!(big_get_reader.get_u64(8).unwrap(), 0x2021222324252627);
        assert_eq!(0, big_get_reader.position().unwrap());
        assert_eq!(
            big_get_reader.get_u64_vec(0, 2).unwrap(),
            vec![0x1213141516171819, 0x2021222324252627]
        );
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_int8() {
        let bytes = vec![0xFE, 0x02, 0x7F];

        for endian in [Endian::Little, Endian::Big] {
            let mut reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(reader.read_i8().unwrap(), -2);

            let mut vec_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(vec_reader.read_i8_vec(3).unwrap(), vec![-2, 2, 127]);

            let mut get_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(get_reader.get_i8(1).unwrap(), 2);
            assert_eq!(0, get_reader.position().unwrap());
            assert_eq!(get_reader.get_i8_vec(0, 3).unwrap(), vec![-2, 2, 127]);
            assert_eq!(0, get_reader.position().unwrap());
        }
    }

    #[test]
    fn read_int16() {
        let bytes = vec![0xFE, 0xFF, 0x00, 0x02, 0xFF, 0x7F];

        let mut little_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_i16().unwrap(), -2);
        let mut little_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(
            little_vec_reader.read_i16_vec(3).unwrap(),
            vec![-2, 512, 32767]
        );
        let mut little_get_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_i16(2).unwrap(), 512);
        assert_eq!(
            little_get_reader.get_i16_vec(0, 3).unwrap(),
            vec![-2, 512, 32767]
        );
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_i16().unwrap(), -257);
        let mut big_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(big_vec_reader.read_i16_vec(3).unwrap(), vec![-257, 2, -129]);
        let mut big_get_reader = BinaryReader::from_bytes(bytes, Endian::Big, true);
        assert_eq!(big_get_reader.get_i16(2).unwrap(), 2);
        assert_eq!(
            big_get_reader.get_i16_vec(0, 3).unwrap(),
            vec![-257, 2, -129]
        );
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_int32() {
        let bytes = vec![0xFE, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x02, 0x00];

        let mut little_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_i32().unwrap(), -2);
        let mut little_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_vec_reader.read_i32_vec(2).unwrap(), vec![-2, 131072]);
        let mut little_get_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_i32(4).unwrap(), 131072);
        assert_eq!(
            little_get_reader.get_i32_vec(0, 2).unwrap(),
            vec![-2, 131072]
        );
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_i32().unwrap(), -16777217);
        let mut big_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(
            big_vec_reader.read_i32_vec(2).unwrap(),
            vec![-16777217, 512]
        );
        let mut big_get_reader = BinaryReader::from_bytes(bytes, Endian::Big, true);
        assert_eq!(big_get_reader.get_i32(4).unwrap(), 512);
        assert_eq!(
            big_get_reader.get_i32_vec(0, 2).unwrap(),
            vec![-16777217, 512]
        );
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_int64() {
        let bytes = vec![
            0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
            0x00,
        ];

        let mut little_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_i64().unwrap(), -2);
        let mut little_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(
            little_vec_reader.read_i64_vec(2).unwrap(),
            vec![-2, 562949953421312]
        );
        let mut little_get_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_i64(8).unwrap(), 562949953421312);
        assert_eq!(
            little_get_reader.get_i64_vec(0, 2).unwrap(),
            vec![-2, 562949953421312]
        );
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_i64().unwrap(), -72057594037927937);
        let mut big_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(
            big_vec_reader.read_i64_vec(2).unwrap(),
            vec![-72057594037927937, 512]
        );
        let mut big_get_reader = BinaryReader::from_bytes(bytes, Endian::Big, true);
        assert_eq!(big_get_reader.get_i64(8).unwrap(), 512);
        assert_eq!(
            big_get_reader.get_i64_vec(0, 2).unwrap(),
            vec![-72057594037927937, 512]
        );
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_float32() {
        let little_bytes = vec![0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x20, 0xC0];
        let big_bytes = vec![0x3F, 0x80, 0x00, 0x00, 0xC0, 0x20, 0x00, 0x00];

        let mut little_reader = BinaryReader::from_bytes(little_bytes.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_f32().unwrap(), 1.0);
        let mut little_vec_reader =
            BinaryReader::from_bytes(little_bytes.clone(), Endian::Little, true);
        assert_eq!(little_vec_reader.read_f32_vec(2).unwrap(), vec![1.0, -2.5]);
        let mut little_get_reader = BinaryReader::from_bytes(little_bytes, Endian::Little, true);
        assert_eq!(little_get_reader.get_f32(4).unwrap(), -2.5);
        assert_eq!(
            little_get_reader.get_f32_vec(0, 2).unwrap(),
            vec![1.0, -2.5]
        );
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(big_bytes.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_f32().unwrap(), 1.0);
        let mut big_vec_reader = BinaryReader::from_bytes(big_bytes.clone(), Endian::Big, true);
        assert_eq!(big_vec_reader.read_f32_vec(2).unwrap(), vec![1.0, -2.5]);
        let mut big_get_reader = BinaryReader::from_bytes(big_bytes, Endian::Big, true);
        assert_eq!(big_get_reader.get_f32(4).unwrap(), -2.5);
        assert_eq!(big_get_reader.get_f32_vec(0, 2).unwrap(), vec![1.0, -2.5]);
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_float64() {
        let little_bytes = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04,
            0xC0,
        ];
        let big_bytes = vec![
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00,
        ];

        let mut little_reader = BinaryReader::from_bytes(little_bytes.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_f64().unwrap(), 1.0);
        let mut little_vec_reader =
            BinaryReader::from_bytes(little_bytes.clone(), Endian::Little, true);
        assert_eq!(little_vec_reader.read_f64_vec(2).unwrap(), vec![1.0, -2.5]);
        let mut little_get_reader = BinaryReader::from_bytes(little_bytes, Endian::Little, true);
        assert_eq!(little_get_reader.get_f64(8).unwrap(), -2.5);
        assert_eq!(
            little_get_reader.get_f64_vec(0, 2).unwrap(),
            vec![1.0, -2.5]
        );
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(big_bytes.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_f64().unwrap(), 1.0);
        let mut big_vec_reader = BinaryReader::from_bytes(big_bytes.clone(), Endian::Big, true);
        assert_eq!(big_vec_reader.read_f64_vec(2).unwrap(), vec![1.0, -2.5]);
        let mut big_get_reader = BinaryReader::from_bytes(big_bytes, Endian::Big, true);
        assert_eq!(big_get_reader.get_f64(8).unwrap(), -2.5);
        assert_eq!(big_get_reader.get_f64_vec(0, 2).unwrap(), vec![1.0, -2.5]);
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_varint_vectors() {
        let short_bytes = vec![0xFE, 0xFF, 0xFF, 0xFF, 0x02, 0x00, 0x00, 0x00];
        let mut short_reader = BinaryReader::from_bytes(short_bytes.clone(), Endian::Little, false);
        assert_eq!(short_reader.read_varint_vec(2).unwrap(), vec![-2, 2]);
        let mut short_get_reader = BinaryReader::from_bytes(short_bytes, Endian::Little, false);
        assert_eq!(short_get_reader.get_varint_vec(0, 2).unwrap(), vec![-2, 2]);
        assert_eq!(0, short_get_reader.position().unwrap());

        let long_bytes = vec![
            0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00,
        ];
        let mut long_reader = BinaryReader::from_bytes(long_bytes.clone(), Endian::Little, true);
        assert_eq!(long_reader.read_varint_vec(2).unwrap(), vec![-2, 2]);
        let mut long_get_reader = BinaryReader::from_bytes(long_bytes, Endian::Little, true);
        assert_eq!(long_get_reader.get_varint_vec(0, 2).unwrap(), vec![-2, 2]);
        assert_eq!(0, long_get_reader.position().unwrap());
    }

    #[test]
    fn read_enum_functions() {
        let mut reader = BinaryReader::from_bytes(vec![1], Endian::Little, true);
        assert_eq!(reader.read_enum8::<EnumU8>().unwrap(), EnumU8::One);

        let mut reader = BinaryReader::from_bytes(vec![0x01, 0x00], Endian::Little, true);
        assert_eq!(reader.read_enum_u16::<EnumU16>().unwrap(), EnumU16::One);

        let mut reader = BinaryReader::from_bytes(vec![1, 0, 0, 0], Endian::Little, true);
        assert_eq!(reader.read_enum_u32::<EnumU32>().unwrap(), EnumU32::One);

        let mut reader = BinaryReader::from_bytes(vec![1, 0, 0, 0, 0, 0, 0, 0], Endian::Little, true);
        assert_eq!(reader.read_enum_u64::<EnumU64>().unwrap(), EnumU64::One);

        let mut reader = BinaryReader::from_bytes(vec![1], Endian::Little, true);
        assert_eq!(reader.read_enum_i8::<EnumI8>().unwrap(), EnumI8::One);

        let mut reader = BinaryReader::from_bytes(vec![1, 0], Endian::Little, true);
        assert_eq!(reader.read_enum_i16::<EnumI16>().unwrap(), EnumI16::One);

        let mut reader = BinaryReader::from_bytes(vec![1, 0, 0, 0], Endian::Little, true);
        assert_eq!(reader.read_enum_i32::<EnumI32>().unwrap(), EnumI32::One);

        let mut reader = BinaryReader::from_bytes(vec![1, 0, 0, 0, 0, 0, 0, 0], Endian::Little, true);
        assert_eq!(reader.read_enum_i64::<EnumI64>().unwrap(), EnumI64::One);

        macro_rules! assert_get_enum {
            ($bytes:expr, $method:ident, $type:ty) => {
                let mut reader = BinaryReader::from_bytes($bytes, Endian::Little, true);
                assert_eq!(reader.$method::<$type>(0).unwrap(), <$type>::One);
                assert_eq!(reader.position().unwrap(), 0);
            };
        }

        assert_get_enum!(vec![1], get_enum8, EnumU8);
        assert_get_enum!(vec![1, 0], get_enum_u16, EnumU16);
        assert_get_enum!(vec![1, 0, 0, 0], get_enum_u32, EnumU32);
        assert_get_enum!(vec![1, 0, 0, 0, 0, 0, 0, 0], get_enum_u64, EnumU64);
        assert_get_enum!(vec![1], get_enum_i8, EnumI8);
        assert_get_enum!(vec![1, 0], get_enum_i16, EnumI16);
        assert_get_enum!(vec![1, 0, 0, 0], get_enum_i32, EnumI32);
        assert_get_enum!(vec![1, 0, 0, 0, 0, 0, 0, 0], get_enum_i64, EnumI64);

        let mut invalid_reader = BinaryReader::from_bytes(vec![0x02, 0x00], Endian::Little, true);
        assert!(invalid_reader.read_enum_u16::<EnumU16>().is_err());
    }

    #[test]
    fn read_vectors() {
        let mut vector2_reader = BinaryReader::from_bytes(
            vec![0x00, 0x00, 0x80, 0x3F, 0xCD, 0xCC, 0x2C, 0x40],
            Endian::Little,
            true,
        );
        let vector2 = vector2_reader.read_vector_2().unwrap();
        assert_eq!(vector2.x, 1.0);
        assert_eq!(vector2.y, 2.7);

        let mut vector3_reader = BinaryReader::from_bytes(
            vec![
                0x00, 0x00, 0x80, 0x3F, 0xCD, 0xCC, 0x2C, 0x40, 0x00, 0x00, 0x00, 0x00,
            ],
            Endian::Little,
            true,
        );
        let vector3 = vector3_reader.read_vector_3().unwrap();
        assert_eq!(vector3.x, 1.0);
        assert_eq!(vector3.y, 2.7);
        assert_eq!(vector3.z, 0.0);

        let mut vector4_reader = BinaryReader::from_bytes(
            vec![
                0x00, 0x00, 0x80, 0x3F, 0xCD, 0xCC, 0x2C, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
            Endian::Little,
            true,
        );
        let vector4 = vector4_reader.read_vector_4().unwrap();
        assert_eq!(vector4.x, 1.0);
        assert_eq!(vector4.y, 2.7);
        assert_eq!(vector4.z, 0.0);
        assert_eq!(vector4.w, 0.0);
    }

    #[test]
    fn read_strings() {
        let mut ascii_reader = BinaryReader::from_bytes(b"hello\0world".to_vec(), Endian::Little, true);
        assert_eq!(ascii_reader.read_ascii().unwrap(), "hello");
        assert_eq!(ascii_reader.read_ascii_len(5).unwrap(), "world");

        let mut ascii_get_reader = BinaryReader::from_bytes(b"xMAGIC\0".to_vec(), Endian::Little, true);
        assert_eq!(ascii_get_reader.get_ascii_len(1, 5).unwrap(), "MAGIC");
        assert_eq!(ascii_get_reader.position().unwrap(), 0);
        assert_eq!(
            ascii_get_reader.assert_ascii(&["xMAGIC", "OTHER"]).unwrap(),
            "xMAGIC"
        );

        let (shift_jis_bytes, _, _) = encoding_rs::SHIFT_JIS.encode("テスト");
        let mut shift_jis_data = shift_jis_bytes.into_owned();
        shift_jis_data.push(0);
        let mut shift_jis_reader = BinaryReader::from_bytes(shift_jis_data, Endian::Little, true);
        assert_eq!(shift_jis_reader.read_shift_jis().unwrap(), "テスト");

        let mut utf16_reader = BinaryReader::from_bytes(
            vec![0x48, 0x00, 0x69, 0x00, 0x00, 0x00],
            Endian::Little,
            true,
        );
        assert_eq!(utf16_reader.read_utf16().unwrap(), "Hi");

        let mut utf16_get_reader = BinaryReader::from_bytes(
            vec![0x00, 0x00, 0x00, 0x48, 0x00, 0x69, 0x00, 0x00],
            Endian::Big,
            true,
        );
        assert_eq!(utf16_get_reader.get_utf16(2).unwrap(), "Hi");
        assert_eq!(utf16_get_reader.position().unwrap(), 0);

        let mut fixed_reader =
            BinaryReader::from_bytes(b"name\0padding".to_vec(), Endian::Little, true);
        assert_eq!(fixed_reader.read_fix_str(8).unwrap(), "name");

        let mut fixed_w_reader = BinaryReader::from_bytes(
            vec![0x6E, 0x00, 0x61, 0x00, 0x00, 0x00, 0xFF, 0xFF],
            Endian::Little,
            true,
        );
        assert_eq!(fixed_w_reader.read_fix_str_w(8).unwrap(), "na");
    }

    #[test]
    fn assert_values() {
        let mut bool_reader = BinaryReader::from_bytes(vec![1], Endian::Little, true);
        assert_eq!(bool_reader.assert_bool(&[false, true]).unwrap(), true);

        let mut integer_reader =
            BinaryReader::from_bytes(vec![1, 0, 0, 0, 0, 0, 0, 0], Endian::Little, true);
        assert_eq!(integer_reader.assert_u8(&[0, 1]).unwrap(), 1);
        assert_eq!(integer_reader.assert_u16(&[0, 1]).unwrap(), 0);
        assert_eq!(integer_reader.assert_u32(&[0, 1]).unwrap(), 0);

        let mut signed_reader = BinaryReader::from_bytes(vec![0xFF], Endian::Little, true);
        assert_eq!(signed_reader.assert_i8(&[-1, 1]).unwrap(), -1);

        let mut float_reader =
            BinaryReader::from_bytes(vec![0x00, 0x00, 0x80, 0x3F], Endian::Little, true);
        assert_eq!(float_reader.assert_f32(&[0.0, 1.0]).unwrap(), 1.0);

        let mut varint_reader =
            BinaryReader::from_bytes(vec![0xFF, 0xFF, 0xFF, 0xFF], Endian::Little, false);
        assert_eq!(varint_reader.assert_varint(&[0, -1]).unwrap(), -1);

        let mut dcp_reader = BinaryReader::from_bytes(
            vec![
                0x44, 0x43, 0x50, 0x00, 0x44, 0x46, 0x4C, 0x54, 0x00, 0x00, 0x00, 0x20,
            ],
            Endian::Big,
            true,
        );

        dcp_reader.assert_ascii(&["DCP\0"]).unwrap();
        dcp_reader.assert_ascii(&["DFLT"]).unwrap();
        dcp_reader.assert_i32(&[0x20]).unwrap();

        let mut invalid_reader = BinaryReader::from_bytes(vec![2], Endian::Little, true);
        assert!(invalid_reader.assert_bool(&[false, true]).is_err());
    }

    #[test]
    fn pad_leaves_next_data_untouched() {
        let mut data = vec![0u8; 0x10];
        data.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

        let mut reader = BinaryReader::from_bytes(data, Endian::Little, true);

        reader.seek(0x0D).unwrap();
        reader.pad(0x10).unwrap();

        let value = reader.read_u32().unwrap();

        assert_eq!(value, 0xEFBEADDE);
    }

    #[test]
    fn pad_supports_different_alignments() {
        let cases = [
            (0x00, 0x04, 0x00),
            (0x01, 0x04, 0x04),
            (0x03, 0x04, 0x04),
            (0x04, 0x04, 0x04),
            (0x07, 0x08, 0x08),
            (0x11, 0x10, 0x20),
            (0x21, 0x20, 0x40),
        ];

        for (position, alignment, expected) in cases {
            let data = vec![0u8; 128];
            let mut reader = BinaryReader::from_bytes(data.clone(), Endian::Little, true);

            reader.seek(position).unwrap();
            reader.pad(alignment).unwrap();

            assert_eq!(reader.position().unwrap(), expected);
        }
    }

    #[test]
    fn pad_rejects_zero_alignment() {
        let data = vec![0u8; 16];
        let mut reader = BinaryReader::from_bytes(data.clone(), Endian::Little, true);

        assert!(reader.pad(0).is_err());
    }

}