use std::io::{self, Read, Seek, Write};

use crate::{bnd::file::BinderFileHeader, io::{BinaryReader, BinaryWriter}, util};

pub(crate) fn assert<R>(br: &mut BinaryReader<R>) -> io::Result<()>
where 
    R: Read + Seek
{
    br.read_i64()?;
    br.read_i32()?;
    br.assert_u8(&[0x10])?;
    br.assert_u8(&[8])?;
    br.assert_u8(&[8])?;
    br.assert_u8(&[0])?;

    Ok(())
}

pub(crate) fn write<W>(bw: &mut BinaryWriter<W>, files: Vec<BinderFileHeader>) -> io::Result<()>
where 
    W: Write + Seek
{
    let mut group_count = 0;
    for p in (files.len() as u32 / 7)..=100_000 {
        if util::is_prime(p) {
            group_count = p;
            break;
        }
    }

    if group_count == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unable to determine hash group count"));
    }

    Ok(())
}

struct PathHash {
    index: i32,
    hash: u32
}

impl PathHash {
    fn new(index: i32, path: impl Into<String>) -> Self {
        Self { index, hash: util::from_path_hash(path) }
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where 
        W: Write + Seek
    {
        bw.write_u32(self.hash)?;
        bw.write_i32(self.index)?;
        Ok(())
    }
}

struct HashGroup {
    index: i32,
    length: i32
}

impl HashGroup {
    fn new(index: i32, length: i32) -> Self {
        Self { index, length }
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where 
        W: Write + Seek
    {
        bw.write_i32(self.length)?;
        bw.write_i32(self.index)?;
        Ok(())
    }
}