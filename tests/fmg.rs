use soulsformats_rs::{ByteIO, FMG, FileIO};

#[test]
fn fmg() {
    let fmg = FMG::from_file("./tests/files/fmg/PlaceName.fmg").unwrap();
    assert_eq!(fmg.find(12029), None);
    assert_eq!(fmg.find(12030), Some(String::from("Deeproot Depths")));
    assert_eq!(fmg.find(12050), Some(String::from("Mohgwyn Palace")));
    let round_trip = FMG::from_bytes(fmg.to_bytes().unwrap()).unwrap();
    assert_eq!(fmg.entries, round_trip.entries);
    assert_eq!(fmg.md5, round_trip.md5);
    assert_eq!(fmg.unicode, round_trip.unicode);
    assert_eq!(fmg.version, round_trip.version);
}
