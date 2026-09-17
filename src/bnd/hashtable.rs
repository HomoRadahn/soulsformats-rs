use std::io::{self, Read, Seek, Write};

use crate::{
    bnd::file::BinderFileHeader,
    io::{BinaryReader, BinaryWriter},
    util,
};

pub(crate) fn assert<R>(br: &mut BinaryReader<R>) -> io::Result<()>
where
    R: Read + Seek,
{
    br.read_i64()?;
    br.read_i32()?;
    br.assert_u8(&[0x10])?;
    br.assert_u8(&[8])?;
    br.assert_u8(&[8])?;
    br.assert_u8(&[0])?;

    Ok(())
}

pub(crate) fn write<W>(bw: &mut BinaryWriter<W>, files: &Vec<BinderFileHeader>) -> io::Result<()>
where
    W: Write + Seek,
{
    let mut group_count = 0;
    for p in (files.len() as u32 / 7)..=100_000 {
        if util::is_prime(p) {
            group_count = p;
            break;
        }
    }

    if group_count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unable to determine hash group count",
        ));
    }

    let mut hash_lists: Vec<Vec<PathHash>> = (0..group_count).map(|_| Vec::new()).collect();

    for i in 0..files.len() {
        let path_hash = PathHash::new(util::try_from_to_io_result(i)?, files[i].name.clone());
        let group = path_hash.hash % group_count;
        hash_lists[group as usize].push(path_hash);
    }

    for hash_list in &mut hash_lists {
        hash_list.sort_by_key(|path_hash| path_hash.hash);
    }

    let mut hash_groups = Vec::with_capacity(hash_lists.len());
    let mut path_hashes = Vec::with_capacity(files.len());
    let mut count = 0i32;

    for hash_list in hash_lists {
        let index = count;
        for path_hash in hash_list {
            path_hashes.push(path_hash);
            count += 1;
        }
        hash_groups.push(HashGroup::new(index, count - index));
    }

    bw.reserve_i64("hashes-offset")?;
    bw.write_u32(group_count)?;
    bw.write_u8(0x10)?;
    bw.write_u8(8)?;
    bw.write_u8(8)?;
    bw.write_u8(0)?;

    for hash_group in hash_groups {
        hash_group.write(bw)?;
    }

    let hashes_offset = bw.position()?;
    bw.fill_i64("hashes-offset", util::try_from_to_io_result(hashes_offset)?)?;

    for path_hash in path_hashes {
        path_hash.write(bw)?;
    }

    Ok(())
}

struct PathHash {
    index: i32,
    hash: u32,
}

impl PathHash {
    fn new(index: i32, path: impl Into<String>) -> Self {
        Self {
            index,
            hash: util::from_path_hash(path),
        }
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bw.write_u32(self.hash)?;
        bw.write_i32(self.index)?;
        Ok(())
    }
}

struct HashGroup {
    index: i32,
    length: i32,
}

impl HashGroup {
    fn new(index: i32, length: i32) -> Self {
        Self { index, length }
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        bw.write_i32(self.length)?;
        bw.write_i32(self.index)?;
        Ok(())
    }
}
