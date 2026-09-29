use soulsformats_rs::{ByteIO, FileIO, ParamDef};

#[test]
fn paramdef() {
    let paramdef = ParamDef::from_file("./tests/files/param/speffect.paramdef").unwrap();
    let round_trip = ParamDef::from_bytes(paramdef.to_bytes().unwrap()).unwrap();
    assert_eq!(paramdef.data_version, round_trip.data_version);
    assert_eq!(paramdef.param_type, round_trip.param_type);
    assert_eq!(paramdef.endian, round_trip.endian);
    assert_eq!(paramdef.unicode, round_trip.unicode);
    assert_eq!(paramdef.fields, round_trip.fields);
    assert_eq!(paramdef.version_aware, round_trip.version_aware);
    assert_eq!(paramdef.basic_fields, round_trip.basic_fields);
}