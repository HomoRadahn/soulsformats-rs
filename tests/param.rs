use std::fs;

use soulsformats_rs::{
    ByteIO, CompressionInfo, DcxIO, Param, ParamDef,
    io::Endian,
    param::{CellValue, FormatFlags1, FormatFlags2, Row},
    paramdef::{Field, ParamDefType},
};

fn test_definition() -> ParamDef {
    let mut def = ParamDef {
        param_type: "TEST_ST".into(),
        data_version: 3,
        ..Default::default()
    };
    let mut signed = Field::new(ParamDefType::I8, "signed");
    signed.bit_size = Some(3);
    let mut unsigned = Field::new(ParamDefType::U8, "unsigned");
    unsigned.bit_size = Some(3);
    let mut padding = Field::new(ParamDefType::ArrayU8, "bits");
    padding.bit_size = Some(2);
    let mut array = Field::new(ParamDefType::U8, "bytes");
    array.array_length = Some(3);
    array.default = CellValue::F32(2.0);
    let mut text = Field::new(ParamDefType::StringUTF16, "text");
    text.array_length = Some(3);
    let mut number = Field::new(ParamDefType::I32, "number");
    number.default = CellValue::F32(3.5);
    def.fields = vec![
        number,
        signed,
        unsigned,
        padding,
        array,
        text,
        Field::new(ParamDefType::Bool, "boolean"),
        Field::new(ParamDefType::F64, "double"),
    ];
    def
}

fn make_row(def: &ParamDef, id: i32, name: Option<&str>) -> Row {
    let mut row = Row::try_new(id, name.map(str::to_owned), def).unwrap();
    assert_eq!(row.cells[0].value, CellValue::I32(4));
    assert_eq!(row.cells[4].value, CellValue::ArrayU8(vec![2, 2, 2]));
    row.cells[0].value = CellValue::I32(99);
    row.cells[1].value = CellValue::I8(-2);
    row.cells[2].value = CellValue::U8(5);
    row.cells[3].value = CellValue::U8(2);
    row.cells[4].value = CellValue::ArrayU8(vec![7, 8, 9]);
    row.cells[5].value = CellValue::StringUTF16("hi".into());
    row.cells[6].value = CellValue::I32(-99);
    row.cells[7].value = CellValue::F64(1.5);
    row
}

#[test]
fn param_layouts_and_cell_types_round_trip() {
    let def = test_definition();
    assert_eq!(def.row_size().unwrap(), 26);
    for endian in [Endian::Little, Endian::Big] {
        for flags in [
            FormatFlags1::None,
            FormatFlags1::Flag01,
            FormatFlags1::Flag01 | FormatFlags1::DataOffsets_32bit,
            FormatFlags1::DataOffsets_64bit,
            FormatFlags1::DataOffsets_64bit | FormatFlags1::OffsetParamType,
        ] {
            for unicode in [false, true] {
                let mut param = Param::new(&def).unwrap();
                param.endian = endian;
                param.format_1 = flags;
                param.format_2 = if unicode {
                    FormatFlags2::UnicodeRowNames
                } else {
                    FormatFlags2::None
                };
                param.rows.push(make_row(&def, 7, Some("name")));
                let bytes = param.to_bytes().unwrap();
                let start = if flags
                    .contains(FormatFlags1::Flag01 | FormatFlags1::DataOffsets_32bit)
                    || flags.contains(FormatFlags1::DataOffsets_64bit)
                {
                    64
                } else {
                    48
                };
                let long = flags.contains(FormatFlags1::DataOffsets_64bit);
                let position = start + if long { 8 } else { 4 };
                let data_offset = if long {
                    let raw: [u8; 8] = bytes[position..position + 8].try_into().unwrap();
                    match endian {
                        Endian::Little => u64::from_le_bytes(raw),
                        Endian::Big => u64::from_be_bytes(raw),
                    }
                } else {
                    let raw: [u8; 4] = bytes[position..position + 4].try_into().unwrap();
                    u64::from(match endian {
                        Endian::Little => u32::from_le_bytes(raw),
                        Endian::Big => u32::from_be_bytes(raw),
                    })
                } as usize;
                let mut expected = Vec::new();
                match endian {
                    Endian::Little => expected.extend_from_slice(&99i32.to_le_bytes()),
                    Endian::Big => expected.extend_from_slice(&99i32.to_be_bytes()),
                }
                expected.extend_from_slice(&[0xae, 7, 8, 9]);
                expected.extend_from_slice(match endian {
                    Endian::Little => &[b'h', 0, b'i', 0, 0, 0],
                    Endian::Big => &[0, b'h', 0, b'i', 0, 0],
                });
                match endian {
                    Endian::Little => {
                        expected.extend_from_slice(&(-99i32).to_le_bytes());
                        expected.extend_from_slice(&1.5f64.to_le_bytes());
                    }
                    Endian::Big => {
                        expected.extend_from_slice(&(-99i32).to_be_bytes());
                        expected.extend_from_slice(&1.5f64.to_be_bytes());
                    }
                }
                assert_eq!(&bytes[data_offset..data_offset + expected.len()], expected);
                let mut decoded = Param::from_bytes(bytes).unwrap();
                assert!(
                    decoded.apply_paramdef_carefully(&def).unwrap(),
                    "{endian:?} {flags:?}"
                );
                assert_eq!(decoded.rows[0].name.as_deref(), Some("name"));
                assert_eq!(decoded.rows[0].cells, param.rows[0].cells);
            }
        }
    }
}

#[test]
fn unnamed_and_headerless_rows_round_trip() {
    let def = test_definition();
    for headerless in [false, true] {
        for flags in [
            FormatFlags1::None,
            FormatFlags1::Flag01,
            FormatFlags1::DataOffsets_64bit,
        ] {
            let mut param = Param::new(&def).unwrap();
            param.format_1 = flags;
            param.unnamed_rows = true;
            param.headerless_rows = headerless;
            param.rows.push(make_row(&def, 0, None));
            param.rows.push(make_row(&def, 1, None));
            let mut decoded = Param::from_bytes(param.to_bytes().unwrap()).unwrap();
            assert_eq!(decoded.headerless_rows, headerless, "{flags:?}");
            assert!(decoded.unnamed_rows);
            assert!(decoded.apply_paramdef_carefully(&def).unwrap());
            assert_eq!(decoded.rows[1].cells, param.rows[1].cells);
        }
    }
}

#[test]
fn versioned_fields_are_filtered_and_packed() {
    let mut def = ParamDef {
        param_type: "VERSIONED".into(),
        data_version: 1,
        version_aware: true,
        ..Default::default()
    };
    let mut removed = Field::new(ParamDefType::U8, "removed");
    removed.removed_regulation_version = Some(2);
    let mut added = Field::new(ParamDefType::I32, "added");
    added.first_regulation_version = Some(2);
    def.fields = vec![removed, added];
    assert_eq!(def.row_size_for_version(1).unwrap(), 1);
    assert_eq!(def.row_size_for_version(2).unwrap(), 4);
    let mut param = Param::new_for_version(&def, 2).unwrap();
    param
        .rows
        .push(Row::try_new_for_version(0, None, &def, 2).unwrap());
    param.rows[0].cells[0].value = CellValue::I32(1234);
    let mut decoded = Param::from_bytes(param.to_bytes().unwrap()).unwrap();
    assert!(
        decoded
            .apply_regulation_versioned_paramdef_carefully(&def, 2)
            .unwrap()
    );
    assert_eq!(decoded.rows[0].cells[0].value, CellValue::I32(1234));
}

#[test]
fn rejects_invalid_layouts_and_incompatible_rows() {
    let mut def = test_definition();
    let mut invalid = def.clone();
    invalid.fields[1].bit_size = Some(9);
    assert!(invalid.row_size().is_err());
    assert!(Row::try_new(0, None, &invalid).is_err());
    invalid = def.clone();
    invalid.fields[2].default = CellValue::U8(8);
    assert!(Row::try_new(0, None, &invalid).is_err());
    def.fields[0].default = CellValue::F32(1e30);
    assert!(Row::try_new(0, None, &def).is_err());

    let def = test_definition();
    let mut param = Param::new(&def).unwrap();
    param.rows.push(make_row(&def, 0, None));
    param.rows[0].cells[2].value = CellValue::U8(8);
    assert!(param.to_bytes().is_err());
    param.rows[0].cells[2].value = CellValue::U8(5);
    param.rows[0].cells[4].value = CellValue::ArrayU8(vec![1]);
    assert!(param.to_bytes().is_err());
    assert!(Param::from_bytes(vec![0; 20]).is_err());
}

#[test]
fn applying_a_bad_definition_is_atomic() {
    let mut def = ParamDef {
        param_type: "BITS".into(),
        ..Default::default()
    };
    let mut bit = Field::new(ParamDefType::U8, "flag");
    bit.bit_size = Some(1);
    def.fields.push(bit);
    let mut param = Param::new(&def).unwrap();
    param.rows.push(Row::new(0, None, &def));
    param.rows.push(Row::new(1, None, &def));
    let mut bytes = param.to_bytes().unwrap();
    let row_offset = u32::from_le_bytes(bytes[64..68].try_into().unwrap()) as usize;
    bytes[row_offset] = 0x80;
    let mut decoded = Param::from_bytes(bytes).unwrap();
    assert!(decoded.apply_paramdef(&def).is_err());
    assert!(!decoded.has_paramdef());
    assert!(decoded.rows.iter().all(|row| row.cells.is_empty()));
}

#[test]
fn empty_params_and_signed_data_versions() {
    let def = ParamDef {
        param_type: "EMPTY".into(),
        data_version: -1,
        ..Default::default()
    };
    let param = Param::new(&def).unwrap();
    let mut decoded = Param::from_bytes(param.to_bytes().unwrap()).unwrap();
    assert!(decoded.rows.is_empty());
    assert_eq!(decoded.detected_size, None);
    assert!(decoded.apply_paramdef_carefully(&def).unwrap());
}

#[test]
fn param_dcx_round_trip() {
    let def = test_definition();
    let mut param = Param::new(&def).unwrap();
    param.rows.push(make_row(&def, 1, None));
    let bytes = param.compress_to_bytes(CompressionInfo::DcpDflt).unwrap();
    let (mut decoded, compression) = Param::decompress_bytes(bytes).unwrap();
    assert_eq!(compression, CompressionInfo::DcpDflt);
    assert!(decoded.apply_paramdef_carefully(&def).unwrap());
    assert_eq!(decoded.rows[0].cells, param.rows[0].cells);
}

#[test]
fn bloodborne_params_match_definitions_and_preserve_row_bytes() {
    for (name, count, size, first_id, first_value) in [
        ("EquipParamWeapon", 1090, 316, 1000, 0),
        ("SpEffectParam", 7855, 528, 0, -1),
    ] {
        let bytes = fs::read(format!("tests/files/param/{name}.param")).unwrap();
        let def = ParamDef::from_xml(format!("tests/files/param/{name}.xml"), false).unwrap();
        let mut param = Param::from_bytes(bytes.clone()).unwrap();
        assert_eq!(param.rows.len(), count);
        assert_eq!(param.detected_size, Some(size));
        assert_eq!(param.rows[0].id, first_id);
        assert!(param.rows[0].name.is_some());
        assert!(param.apply_paramdef_carefully(&def).unwrap());
        assert_eq!(param.rows[0].cells[0].value, CellValue::I32(first_value));
        let output = param.to_bytes().unwrap();
        for index in 0..count {
            let header = 64 + index * 24;
            let old_offset =
                i64::from_le_bytes(bytes[header + 8..header + 16].try_into().unwrap()) as usize;
            let new_offset =
                i64::from_le_bytes(output[header + 8..header + 16].try_into().unwrap()) as usize;
            assert_eq!(
                &bytes[old_offset..old_offset + size as usize],
                &output[new_offset..new_offset + size as usize],
                "{name} row {index}"
            );
        }
        let mut read_back = Param::from_bytes(output).unwrap();
        assert!(read_back.apply_paramdef_carefully(&def).unwrap());
        assert_eq!(read_back.rows[0].cells, param.rows[0].cells);
        assert_eq!(
            read_back.rows.last().unwrap().cells,
            param.rows.last().unwrap().cells
        );
        if name == "EquipParamWeapon" {
            read_back.rows[0].cells[0].set(123_i32).unwrap();
            let mut edited = Param::from_bytes(read_back.to_bytes().unwrap()).unwrap();
            assert!(edited.apply_paramdef_carefully(&def).unwrap());
            assert_eq!(edited.rows[0].cells[0].value, CellValue::I32(123));
        }
    }
}
