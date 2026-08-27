use std::{io::{self, Seek, SeekFrom, Write}};
use crate::io_types::{Endian};

macro_rules! impl_numeric_writer {
    ($type:ty, $write:ident, $write_vec:ident, $reserve:ident, $fill:ident) => {
        pub fn $write(&mut self, data: $type) -> io::Result<()> {
            match self.endian {
                Endian::Little => self.inner.write_all(&data.to_le_bytes()),
                Endian::Big => self.inner.write_all(&data.to_be_bytes()),
            }
        }

        pub fn $write_vec(&mut self, data: &Vec<$type>) -> io::Result<()> {
            for value in data {
                match self.endian {
                    Endian::Little => self.inner.write_all(&value.to_le_bytes())?,
                    Endian::Big => self.inner.write_all(&value.to_be_bytes())?,
                };
            }

            Ok(())
        }
    };
}

pub struct BinaryWriter<W> {
    inner: W,
    endian: Endian,
    reservations: Vec<Reservation>
}

pub struct Reservation {
    position: u64,
}

impl<W: Write + Seek> BinaryWriter<W> {
    pub fn new(inner: W, endian: Endian) -> Self {
        Self { inner, endian, reservations: Vec::new() }
    }

    /// Returns current stream position
    pub fn position(&mut self) -> io::Result<u64> {
        return self.inner.stream_position()
    }

    // Returns total stream length
    pub fn length(&mut self) -> io::Result<u64> {
        let initial = self.position()?;
        let length = self.inner.seek(SeekFrom::End(0))?;
        self.seek(initial)?;
        Ok(length)
    }

    // Returns remaining length of the stream
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

    // Reservation: functions for reserving and filling - only need to return position. Before reserving check if can reserve

    impl_numeric_writer!(u8, write_u8, write_u8_vec, reserve_u8, fill_u8);
    impl_numeric_writer!(u16, write_u16, write_u16_vec, reserve_u16, fill_u16);
    impl_numeric_writer!(u32, write_u32, write_u32_vec, reserve_u32, fill_u32);
    impl_numeric_writer!(u64, write_u64, write_u64_vec, reserve_u64, fill_u64);
    impl_numeric_writer!(i8, write_i8, write_i8_vec, reserve_i8, fill_i8);
    impl_numeric_writer!(i16, write_i16, write_i16_vec, reserve_i16, fill_i16);
    impl_numeric_writer!(i32, write_i32, write_i32_vec, reserve_i32, fill_i32);
    impl_numeric_writer!(i64, write_i64, write_i64_vec, reserve_i64, fill_i64);
    impl_numeric_writer!(f32, write_f32, write_f32_vec, reserve_f32, fill_f32);
    impl_numeric_writer!(f64, write_f64, write_f64_vec, reserve_f64, fill_f64);

    pub fn write_bool(&mut self, value: bool) -> io::Result<()> {
        self.inner.write_all(&[value as u8])
    }

    pub fn write_bool_vec(&mut self, data: &Vec<bool>) -> io::Result<()> {
        for value in data {
            self.write_bool(*value)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BinaryWriter, Endian};
    use std::io::Cursor;

    macro_rules! test_numeric_writer {
        ($name:ident, $type:ty, $write:ident, $write_vec:ident, $single:expr, $many:expr, $little_single:expr, $big_single:expr, $little_many:expr, $big_many:expr) => {
            #[test]
            fn $name() {
                for (endian, expected_single, expected_many) in [
                    (Endian::Little, $little_single, $little_many),
                    (Endian::Big, $big_single, $big_many),
                ] {
                    let mut writer = BinaryWriter::new(Cursor::new(Vec::new()), endian);
                    writer.$write($single).unwrap();
                    assert_eq!(writer.inner.get_ref(), &expected_single[..]);

                    let mut vec_writer = BinaryWriter::new(Cursor::new(Vec::new()), endian);
                    vec_writer.$write_vec(&$many).unwrap();
                    assert_eq!(vec_writer.inner.get_ref(), &expected_many[..]);
                }
            }
        };
    }

    test_numeric_writer!(
        write_uint8,
        u8,
        write_u8,
        write_u8_vec,
        0x12,
        vec![0x12, 0x13, 0x14],
        [0x12],
        [0x12],
        [0x12, 0x13, 0x14],
        [0x12, 0x13, 0x14]
    );
    test_numeric_writer!(
        write_uint16,
        u16,
        write_u16,
        write_u16_vec,
        0x1312,
        vec![0x1312, 0x1514],
        [0x12, 0x13],
        [0x13, 0x12],
        [0x12, 0x13, 0x14, 0x15],
        [0x13, 0x12, 0x15, 0x14]
    );
    test_numeric_writer!(
        write_uint32,
        u32,
        write_u32,
        write_u32_vec,
        0x15141312,
        vec![0x15141312, 0x19181716],
        [0x12, 0x13, 0x14, 0x15],
        [0x15, 0x14, 0x13, 0x12],
        [0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19],
        [0x15, 0x14, 0x13, 0x12, 0x19, 0x18, 0x17, 0x16]
    );
    test_numeric_writer!(
        write_uint64,
        u64,
        write_u64,
        write_u64_vec,
        0x1918171615141312,
        vec![0x1918171615141312, 0x2726252423222120],
        [0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19],
        [0x19, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12],
        [0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27],
        [0x19, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x20]
    );
    test_numeric_writer!(
        write_int8,
        i8,
        write_i8,
        write_i8_vec,
        -2,
        vec![-2, 2, 127],
        [0xFE],
        [0xFE],
        [0xFE, 0x02, 0x7F],
        [0xFE, 0x02, 0x7F]
    );
    test_numeric_writer!(
        write_int16,
        i16,
        write_i16,
        write_i16_vec,
        -2,
        vec![-2, 512, 32767],
        [0xFE, 0xFF],
        [0xFF, 0xFE],
        [0xFE, 0xFF, 0x00, 0x02, 0xFF, 0x7F],
        [0xFF, 0xFE, 0x02, 0x00, 0x7F, 0xFF]
    );
    test_numeric_writer!(
        write_int32,
        i32,
        write_i32,
        write_i32_vec,
        -2,
        vec![-2, 131072],
        [0xFE, 0xFF, 0xFF, 0xFF],
        [0xFF, 0xFF, 0xFF, 0xFE],
        [0xFE, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x02, 0x00],
        [0xFF, 0xFF, 0xFF, 0xFE, 0x00, 0x02, 0x00, 0x00]
    );
    test_numeric_writer!(
        write_int64,
        i64,
        write_i64,
        write_i64_vec,
        -2,
        vec![-2, 562949953421312],
        [0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE],
        [0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00],
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );

    test_numeric_writer!(
        write_float64,
        f64,
        write_f64,
        write_f64_vec,
        2.7,
        vec![2.7, 0.0],
        [0x9A, 0x99, 0x99, 0x99, 0x99, 0x99, 0x05, 0x40],
        [0x40, 0x05, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A],
        [0x9A, 0x99, 0x99, 0x99, 0x99, 0x99, 0x05, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        [0x40, 0x05, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}
