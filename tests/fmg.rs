use soulsformats_rs::io::{BinaryReader, Endian};
use soulsformats_rs::{ByteIO, FileIO, Fmg};

#[test]
fn fmg() {
    let fmg = Fmg::from_file("./tests/files/fmg/PlaceName.fmg").unwrap();
    assert_eq!(fmg.find(12029), None);
    assert_eq!(fmg.find(12030), Some(String::from("Deeproot Depths")));
    assert_eq!(fmg.find(12050), Some(String::from("Mohgwyn Palace")));
    let round_trip = Fmg::from_bytes(fmg.to_bytes().unwrap()).unwrap();
    assert_eq!(fmg.entries, round_trip.entries);
    assert_eq!(fmg.md5, round_trip.md5);
    assert_eq!(fmg.unicode, round_trip.unicode);
    assert_eq!(fmg.version, round_trip.version);
}

#[test]
fn sorted_entries_and_reused_offsets_round_trip() {
    for unicode in [false, true] {
        for reuse_offsets in [false, true] {
            for md5 in [false, true] {
                let mut fmg = Fmg {
                    unicode,
                    reuse_offsets,
                    md5,
                    ..Default::default()
                };
                fmg.append(3, Some("duplicate"));
                fmg.append(1, Some("other"));
                fmg.append(2, Some("duplicate"));
                let bytes = fmg.to_bytes().unwrap();
                let base = if md5 { 16 } else { 0 };
                let mut reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, false);
                let offsets = reader.get_i32(base + 20).unwrap() as u64 + base;
                let duplicate_offset = reader.get_i32(offsets + 4).unwrap();
                let second_offset = reader.get_i32(offsets + 8).unwrap();
                assert_eq!(duplicate_offset == second_offset, reuse_offsets);
                let read = Fmg::from_bytes(bytes).unwrap();
                assert_eq!(
                    read.entries.iter().map(|entry| entry.id).collect::<Vec<_>>(),
                    [1, 2, 3]
                );
                assert_eq!(read.find(2).as_deref(), Some("duplicate"));
                assert_eq!(read.find(3).as_deref(), Some("duplicate"));
            }
        }
    }
}
