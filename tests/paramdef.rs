use soulsformats_rs::{ByteIO, FileIO, ParamDef, io::Endian, param::CellValue, paramdef::{EditFlags, Field, ParamDefType}};

#[test]
fn paramdef_er() {
    let paramdef = ParamDef::from_xml("./tests/files/paramdef/EquipParamCustomWeapon.xml", false).unwrap();
    let expected_fields = vec![
        Field { 
            display_name: "武器ベースID".to_string(),
            display_type: ParamDefType::I32,
            display_format: "%d".to_string(),
            default: CellValue::I32(0),
            min: CellValue::I32(0),
            max: CellValue::I32(999999999),
            increment: CellValue::I32(1),
            edit_flags: EditFlags::Wrap,
            array_length: None,
            description: Some("武器ベースID".to_string()),
            internal_type: Some("s32".to_string()),
            internal_name: Some("baseWepId".to_string()),
            bit_size: None,
            sort_id: Some(100),
            unk_b8: None,
            unk_c0: None,
            unk_c8: None,
            first_regulation_version: None,
            removed_regulation_version: None
        },
        Field { 
            display_name: "魔石ID".to_string(),
            display_type: ParamDefType::I32,
            display_format: "%d".to_string(),
            default: CellValue::I32(0),
            min: CellValue::I32(0),
            max: CellValue::I32(999999999),
            increment: CellValue::I32(1),
            edit_flags: EditFlags::Wrap,
            array_length: None,
            description: Some("魔石ID".to_string()),
            internal_type: Some("s32".to_string()),
            internal_name: Some("gemId".to_string()),
            bit_size: None,
            sort_id: Some(300),
            unk_b8: None,
            unk_c0: None,
            unk_c8: None,
            first_regulation_version: None,
            removed_regulation_version: None
        },
        Field { 
            display_name: "強化値".to_string(),
            display_type: ParamDefType::U8,
            display_format: "%d".to_string(),
            default: CellValue::U8(0),
            min: CellValue::U8(0),
            max: CellValue::U8(99),
            increment: CellValue::U8(1),
            edit_flags: EditFlags::Wrap,
            array_length: Some(1),
            description: Some("強化値".to_string()),
            internal_type: Some("u8".to_string()),
            internal_name: Some("reinforceLv".to_string()),
            bit_size: None,
            sort_id: Some(200),
            unk_b8: None,
            unk_c0: None,
            unk_c8: None,
            first_regulation_version: None,
            removed_regulation_version: None
        },
        Field { 
            display_name: "pad".to_string(),
            display_type: ParamDefType::ArrayU8,
            display_format: "".to_string(),
            default: CellValue::ArrayU8(Vec::new()),
            min: CellValue::ArrayU8(Vec::new()),
            max: CellValue::ArrayU8(Vec::new()),
            increment: CellValue::ArrayU8(Vec::new()),
            edit_flags: EditFlags::None,
            array_length: Some(7),
            description: Some("pad".to_string()),
            internal_type: Some("dummy8".to_string()),
            internal_name: Some("pad".to_string()),
            bit_size: None,
            sort_id: Some(301),
            unk_b8: None,
            unk_c0: None,
            unk_c8: None,
            first_regulation_version: None,
            removed_regulation_version: None
        }
    ];

    assert_eq!(paramdef.param_type, "EQUIP_PARAM_CUSTOM_WEAPON_ST");
    assert_eq!(paramdef.basic_fields, false);
    assert_eq!(paramdef.data_version, 1);
    assert_eq!(paramdef.endian, Endian::Little);
    assert_eq!(paramdef.format_version, 203);
    assert_eq!(paramdef.version_aware, false);
    assert_eq!(paramdef.unicode, true);

    for i in 0..paramdef.fields.len() {
        assert_eq!(paramdef.fields[i], expected_fields[i])
    }
}

#[test]
fn paramdef_legacy() {
    let paramdef = ParamDef::from_file("./tests/files/paramdef/speffect.paramdef").unwrap();
    let round_trip = ParamDef::from_bytes(paramdef.to_bytes().unwrap()).unwrap();
    let xml_paramdef = ParamDef::from_xml("./tests/files/paramdef/SpEffect.xml", false).unwrap();
    assert_eq!(paramdef.data_version, round_trip.data_version);
    assert_eq!(paramdef.param_type, round_trip.param_type);
    assert_eq!(paramdef.endian, round_trip.endian);
    assert_eq!(paramdef.unicode, round_trip.unicode);
    assert_eq!(paramdef.fields, round_trip.fields);
    assert_eq!(paramdef.version_aware, round_trip.version_aware);
    assert_eq!(paramdef.basic_fields, round_trip.basic_fields);
    assert_eq!(paramdef.data_version, xml_paramdef.data_version);
    assert_eq!(paramdef.param_type, xml_paramdef.param_type);
    assert_eq!(paramdef.endian, xml_paramdef.endian);
    assert_eq!(paramdef.unicode, xml_paramdef.unicode);
    assert_eq!(paramdef.fields, xml_paramdef.fields);
    assert_eq!(paramdef.version_aware, xml_paramdef.version_aware);
    assert_eq!(paramdef.basic_fields, xml_paramdef.basic_fields);
}


#[test]
fn paramdef_legacy_xml() {
    let def = ParamDef::from_xml("./tests/files/paramdef/SpEffect.xml", false).unwrap();

    assert_eq!(def.param_type, "SP_EFFECT_PARAM_ST");
    assert_eq!(def.format_version, 102);
    assert_eq!(def.fields.len(), 106);

    let serialized = def.to_xml_string(0, true).unwrap();
    assert!(serialized.contains("<Unk06>1</Unk06>"));
    assert!(serialized.contains("<Version>102</Version>"));
    assert!(serialized.contains("<!-- +0x0 -->"));

    let round_trip = ParamDef::from_xml_string(&serialized, false).unwrap();
    assert_eq!(def.data_version, round_trip.data_version);
    assert_eq!(def.param_type, round_trip.param_type);
    assert_eq!(def.endian, round_trip.endian);
    assert_eq!(def.unicode, round_trip.unicode);
    assert_eq!(def.format_version, round_trip.format_version);
    assert_eq!(def.version_aware, round_trip.version_aware);
    assert_eq!(def.basic_fields, round_trip.basic_fields);
    assert_eq!(def.fields.len(), round_trip.fields.len());
    for (index, (original, decoded)) in def.fields.iter().zip(&round_trip.fields).enumerate() {
        assert_eq!(original, decoded, "field {index} did not round-trip");
    }
}

#[test]
fn paramdef_xml_version_aware() {
    let xml = r#"
<PARAMDEF XmlVersion="3">
  <ParamType>TEST_PARAM_ST</ParamType>
  <DataVersion>4</DataVersion>
  <BigEndian>false</BigEndian>
  <Unicode>true</Unicode>
  <FormatVersion>203</FormatVersion>
  <Fields>
    <Field Def="u8 flags[4]" FirstVersion="2" RemovedVersion="8">
      <DisplayName>Flags &amp; values</DisplayName>
      <EditFlags>Wrap, Lock</EditFlags>
      <Minimum>0</Minimum>
      <Maximum>255</Maximum>
    </Field>
    <Field Def="s32 obsolete" RemovedVersion="9"/>
  </Fields>
</PARAMDEF>"#;

    let latest = ParamDef::from_xml_string(xml, false).unwrap();
    assert_eq!(latest.fields.len(), 0);

    let version_aware = ParamDef::from_xml_string(xml, true).unwrap();
    assert_eq!(version_aware.fields.len(), 2);
    assert_eq!(version_aware.fields[0].display_name, "Flags & values");
    assert_eq!(version_aware.fields[0].first_regulation_version, Some(2));
    assert_eq!(version_aware.fields[0].removed_regulation_version, Some(8));

    let serialized = version_aware.to_xml_string(3, false).unwrap();
    let round_trip = ParamDef::from_xml_string(&serialized, true).unwrap();
    assert_eq!(version_aware, round_trip);
}

#[test]
fn paramdef_xml_rejects_invalid_values_and_versions() {
    let xml = r#"<PARAMDEF><ParamType>T</ParamType><DataVersion>0</DataVersion><BigEndian>False</BigEndian><Unicode>False</Unicode><FormatVersion>203</FormatVersion><Fields><Field Def="s32 value = nope"/></Fields></PARAMDEF>"#;
    assert!(ParamDef::from_xml_string(xml, false).is_err());

    let def = ParamDef::default();
    assert!(def.to_xml_string(4, false).is_err());
}
