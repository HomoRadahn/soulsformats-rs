mod entry;
use md5::{Digest, Md5};
use std::{
    collections::HashMap,
    io::{self, Read, Seek, Write},
};

pub use entry::Entry;

use crate::{
    ByteIO, FileIO,
    io::{BinaryReader, BinaryWriter, Endian, StreamIO},
    util,
};

#[derive(Debug, Clone, PartialEq)]
/// A simple string container used throughout the series
pub struct FMG {
    /// The strings contained in this `FMG`
    pub entries: Vec<Entry>,
    /// Indicates file format; 0 - DeS, 1 - DS1/DS2, 2 - DS3/BB
    pub version: Version,
    /// Data endianness
    pub endian: Endian,
    /// Whether or not the FMG uses UTF16 encoding
    pub unicode: bool,
    /// Whether or not to add an MD5 hash to the top of the FMG file (Gundam Unicorn)
    pub md5: bool,
    /// Whether or not to reuse offsets to save space on duplicate entries
    pub reuse_offsets: bool,
}

impl FMG {
    /// Creates an empty `FMG`, formatted for DS1/DS2
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            version: Version::DarkSouls1,
            endian: Endian::Little,
            unicode: true,
            md5: false,
            reuse_offsets: false,
        }
    }

    /// Initializes `FMG` with specified version
    pub fn with_version(version: Version) -> Self {
        let mut out = Self::empty();
        out.version = version;
        out
    }

    /// Finds `Entry` text by ID
    pub fn find(&self, id: i32) -> Option<String> {
        if let Some(entry) = self.entries.iter().find(|entry| entry.id == id) {
            entry.text.clone()
        } else {
            None
        }
    }

    /// Adds a new `Entry` to `FMG` from specified parameters
    pub fn append(&mut self, id: i32, text: Option<impl Into<String>>) {
        self.entries.push(Entry::new(id, text));
    }

    /// Replaces the text in an existing `Entry`, or adds a new one if it doesn't exist.
    pub fn replace(&mut self, id: i32, text: Option<impl Into<String>>) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            entry.text = text.map(Into::into);
        } else {
            self.append(id, text);
        }
    }

    pub fn write_strings_reuse_offsets<W>(
        &self,
        bw: &mut BinaryWriter<W>,
        entries: Vec<Entry>,
    ) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let mut offset_dict: HashMap<String, i64> = HashMap::new();
        for (index, entry) in entries.into_iter().enumerate() {
            let reservation_name = format!("string-offset-{index}");
            let text = entry.text;

            if let Some(text) = text {
                if let Some(&offset) = offset_dict.get(&text) {
                    bw.fill_varint(format!("string-offset-{index}"), offset)?;
                } else {
                    let offset = bw.position()?;
                    offset_dict.insert(text.clone(), util::convert_num(offset)?);
                    bw.fill_varint(reservation_name, util::convert_num(offset)?)?;

                    if self.unicode {
                        bw.write_utf16(text, true)?;
                    } else {
                        bw.write_shift_jis(text, true)?;
                    }
                }
            } else {
                bw.fill_varint(reservation_name, 0)?;
            }
        }
        Ok(())
    }

    pub fn write_strings<W>(&self, bw: &mut BinaryWriter<W>, entries: Vec<Entry>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        for (index, entry) in entries.into_iter().enumerate() {
            let reservation_name = format!("string-offset-{index}");
            if let Some(text) = entry.text {
                let pos = bw.position()?;
                bw.fill_varint(reservation_name, util::convert_num(pos)?)?;

                if self.unicode {
                    bw.write_utf16(text, true)?;
                } else {
                    bw.write_shift_jis(text, true)?;
                }
            } else {
                bw.fill_varint(reservation_name, 0)?;
            }
        }

        Ok(())
    }
}

impl StreamIO<FMG> for FMG {
    fn read<R>(br: &mut BinaryReader<R>) -> io::Result<FMG>
    where
        R: Read + Seek,
    {
        let mut out = Self::empty();
        if br.get_u8(0)? != 0 {
            out.md5 = true;
            br.skip(16)?;
        }

        br.assert_u8(&[0])?;
        out.endian = match br.read_bool()? {
            true => Endian::Big,
            false => Endian::Little,
        };
        br.endian = out.endian;
        out.version = br.read_enum_u8::<Version>()?;
        br.assert_u8(&[0])?;

        let wide = out.version == Version::DarkSouls3;
        br.varint_64bit = wide;

        br.read_i32()?;
        out.unicode = br.read_bool()?;

        if out.version == Version::DemonsSouls {
            br.assert_u8(&[0xFF])?;
        } else {
            br.assert_u8(&[0x00])?;
        }
        br.assert_u8(&[0])?;
        br.assert_u8(&[0])?;

        let group_count = br.read_i32()?;
        br.read_i32()?;

        if wide {
            br.assert_i32(&[0xFF])?;
        }

        let string_offsets_offset = if out.md5 {
            br.read_varint()? + 16
        } else {
            br.read_varint()?
        };

        br.assert_varint(&[0])?;

        for _ in 0..group_count {
            let offset_index = br.read_i32()?;
            let first_id = br.read_i32()?;
            let last_id = br.read_i32()?;

            if wide {
                br.assert_i32(&[0])?;
            }

            let pos = br.position()?;
            br.seek(match wide {
                true => string_offsets_offset as u64 + offset_index as u64 * 8,
                false => string_offsets_offset as u64 + offset_index as u64 * 4,
            })?;

            for index in 0..=(last_id - first_id) {
                let string_offset = if out.md5 {
                    br.read_varint()? + 16
                } else {
                    br.read_varint()?
                };

                let id = first_id + index;
                let text = if string_offset > 0 {
                    if out.unicode {
                        Some(br.get_utf16(string_offset as u64)?)
                    } else {
                        Some(br.get_shift_jis(string_offset as u64)?)
                    }
                } else {
                    None
                };
                out.append(id, text);
            }

            br.seek(pos)?;
        }

        Ok(out)
    }

    fn write<W>(&self, bw: &mut BinaryWriter<W>) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let mut ref_bw = BinaryWriter::to_bytes(self.endian, false);
        ref_bw.endian = self.endian;

        let wide = self.version == Version::DarkSouls3;
        ref_bw.varint_64bit = wide;

        ref_bw.write_u8(0)?;
        ref_bw.write_bool(match ref_bw.endian {
            Endian::Big => true,
            Endian::Little => false,
        })?;
        ref_bw.write_u8(self.version as u8)?;
        ref_bw.write_u8(0)?;

        ref_bw.reserve_i32("file-size")?;
        ref_bw.write_bool(self.unicode)?;

        if self.version == Version::DemonsSouls {
            ref_bw.write_u8(0xFF)?;
        } else {
            ref_bw.write_u8(0x00)?;
        }
        ref_bw.write_u8(0)?;
        ref_bw.write_u8(0)?;
        ref_bw.reserve_i32("group-count")?;
        ref_bw.write_i32(util::convert_num(self.entries.len())?)?;

        if wide {
            ref_bw.write_i32(0xFF)?;
        }

        ref_bw.reserve_varint("string-offsets")?;
        ref_bw.write_varint(0)?;

        let mut group_count = 0;
        let mut entries = self.entries.clone();
        entries.sort_by(|e1, e2| e1.id.cmp(&e2.id));

        let mut iter = 0;
        while iter < entries.len() {
            ref_bw.write_i32(util::convert_num(iter)?)?;
            ref_bw.write_i32(entries[iter].id)?;
            while iter < entries.len() - 1 && entries[iter + 1].id == entries[iter].id + 1 {
                iter += 1;
            }
            ref_bw.write_i32(entries[iter].id)?;

            if wide {
                ref_bw.write_i32(0)?;
            }

            group_count += 1;
            iter += 1;
        }

        ref_bw.fill_i32("group-count", group_count)?;
        let pos = ref_bw.position()?;
        ref_bw.fill_varint("string-offsets", util::convert_num(pos)?)?;

        for index in 0..entries.len() {
            ref_bw.reserve_varint(format!("string-offset-{index}"))?;
        }

        if self.reuse_offsets {
            self.write_strings_reuse_offsets(&mut ref_bw, entries)?;
        } else {
            self.write_strings(&mut ref_bw, entries)?;
        }

        let pos = ref_bw.position()?;
        ref_bw.fill_i32("file-size", util::convert_num(pos)?)?;

        if self.md5 {
            ref_bw.seek(0)?;
            let final_data = ref_bw.close_bytes()?;

            let mut hasher = Md5::new();
            hasher.update(&final_data);

            let hash = hasher.finalize().to_vec();

            bw.seek(0)?;
            bw.write_vec_u8(hash)?;
            bw.write_vec_u8(final_data)?;
        } else {
            let final_data = ref_bw.close_bytes()?;
            bw.write_vec_u8(final_data)?;
        }

        Ok(())
    }
}

impl ByteIO<FMG> for FMG {}
impl FileIO<FMG> for FMG {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Indicates the game this FMG is for, and thus the format it will be written in
pub enum Version {
    /// Demon's Souls
    DemonsSouls = 0,
    /// Dark Souls 1 and Dark Souls 2
    DarkSouls1 = 1,
    /// Bloodborne and Dark Souls 3
    DarkSouls3 = 2,
}

impl TryFrom<u8> for Version {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::DemonsSouls),
            1 => Ok(Self::DarkSouls1),
            2 => Ok(Self::DarkSouls3),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid enum value",
            )),
        }
    }
}
