use std::fs::File;
use std::io::{self, Cursor, Seek, SeekFrom, Write};
use std::path::Path;
use crate::io::{Endian};

macro_rules! impl_numeric_writer {
    ($type:ty, $reservation_size:expr, $write:ident, $write_vec:ident, $reserve:ident, $fill:ident) => {
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

        pub fn $reserve(&mut self) -> io::Result<Reservation> {
            self.add_reservation($reservation_size)
        }

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
    reservations: Vec<Reservation>
}

#[derive(Clone, Copy, PartialEq)]
pub struct Reservation {
    position: u64,
}

impl<W: Write + Seek> BinaryWriter<W> {
    pub fn new(inner: W, endian: Endian, varint_i64: bool) -> Self {
        Self { inner, endian, varint_i64, reservations: Vec::new() }
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

    /// Adds reservation at a specified position if that position is not added to current reservation list
    fn add_reservation(&mut self, size: usize) -> io::Result<Reservation> {
        let position = self.position()?;
        for reservation in &self.reservations {
            if reservation.position == position {
                return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("tried to reserve already reserved position"),
            ))
            }
        }

        self.write_u8_vec(&vec![0xFE; size])?;

        self.reservations.push(Reservation { position });

        Ok(Reservation { position })
    }

    /// Frees a reservation from the list
    fn free_reservation(&mut self, reservation: Reservation) -> io::Result<()> {
        if !self.reservations.contains(&reservation) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("tried to fill unreserved position")))
        }
        
        if let Some(pos) = self.reservations.iter().position(|x| *x == reservation) {
            self.reservations.remove(pos);
        }

        Ok(())
    }

    pub fn assert_closing(&mut self) -> io::Result<()> {
        if self.reservations.len() != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unable to close writer - not all reservations have been filled"),
            ))
        }

        Ok(())
    }

    pub fn write_bool(&mut self, value: bool) -> io::Result<()> {
        self.inner.write_all(&[value as u8])
    }

    pub fn write_bool_vec(&mut self, data: &Vec<bool>) -> io::Result<()> {
        for value in data {
            self.write_bool(*value)?;
        }

        Ok(())
    }

    pub fn reserve_bool(&mut self) -> io::Result<Reservation> {
        self.add_reservation(1)
    }

    pub fn fill_bool(&mut self, reservation: Reservation, value: bool) -> io::Result<()> {
        self.free_reservation(reservation)?;

        let initial_position = self.position()?;
        
        self.seek(reservation.position)?;

        self.write_bool(value)?;

        self.seek(initial_position)?;
        Ok(())
    }

    pub fn write_varint(&mut self, value: i64) -> io::Result<()> {
        match self.varint_i64 {
            true => self.write_i64(value)?,
            false => self.write_i32(value as i32)?
        }

        Ok(())
    }

    pub fn write_varint_vec(&mut self, data: &Vec<i64>) -> io::Result<()> {
        for value in data {
            self.write_varint(*value)?;
        }

        Ok(())
    }

    pub fn reserve_varint(&mut self) -> io::Result<Reservation> {
        match self.varint_i64 {
            true => self.reserve_i64(),
            false => self.reserve_i32()
        }
    }

    pub fn fill_varint(&mut self, reservation: Reservation, value: i64) -> io::Result<()> {
        match self.varint_i64 {
            true => self.fill_i64(reservation, value)?,
            false => self.fill_i32(reservation, value as i32)?
        }

        Ok(())
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
    /// Initializes the BinaryWriter to write into a vector of bytes (u8)
    pub fn to_bytes(endian: Endian, varint_i64: bool) -> Self {
        BinaryWriter::new(Cursor::new(Vec::new()), endian, varint_i64)
    }

    pub fn get_ref_bytes(&mut self) -> &Vec<u8> {
        self.inner.get_ref()
    }

    /// Asserts closing the BinaryWriter and return the written bytes (the writer can still technically be used afterwards, but this should the last step of using it - followed by dropping it)
    pub fn close_bytes(&mut self) -> io::Result<Vec<u8>> {
        self.assert_closing()?;
        Ok(self.inner.get_ref().clone())
    }
}

impl BinaryWriter<File> {
    /// Initializes the BinaryWriter, writing to a specified file. Make sure to call self.assert_closing() before dropping the BinaryWriter
    pub fn to_file<P: AsRef<Path>>(path: P, endian: Endian, varint_i64: bool) -> io::Result<Self> {
        Ok(BinaryWriter::new(File::create(path)?, endian, varint_i64))
    }

    pub fn get_ref_file(&self) -> &File {
        &self.inner
    }
}
