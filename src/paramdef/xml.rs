use std::{fs, io::{self, ErrorKind::InvalidData}};

use quick_xml::{
    Reader,
    escape::unescape,
    events::{BytesStart, Event},
};

use crate::{
    io::Endian,
    param::CellValue,
    paramdef::{EditFlags, Field, ParamDef, ParamDefType, util},
};

const CURRENT_XML_VERSION: u8 = 3;

#[derive(Default)]
struct XmlNode {
    name: String,
    attributes: Vec<(String, String)>,
    text: String,
    children: Vec<XmlNode>,
}

impl XmlNode {
    fn child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|child| child.name == name)
    }

    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Self> {
        self.children.iter().filter(move |child| child.name == name)
    }

    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn content(&self) -> String {
        let mut content = self.text.clone();
        for child in &self.children {
            content.push_str(&child.content());
        }
        content
    }
}

impl ParamDef {
    /// Deserializes a ParamDef from a file containing XML data.
    ///
    /// `version_aware` set to false by default
    /// 
    /// When `version_aware` is false, fields with a non-zero `RemovedVersion`
    /// are omitted, matching the latest-version view of a ParamDef.
    pub fn from_xml(path: impl Into<String>, version_aware: bool) -> io::Result<Self> {
        let xml = fs::read_to_string(path.into())?;
        Self::from_xml_string(&xml, version_aware)
    }
    /// Deserializes a ParamDef from a string containing XML data.
    ///
    /// When `version_aware` is false, fields with a non-zero `RemovedVersion`
    /// are omitted, matching the latest-version view of a ParamDef.
    pub fn from_xml_string(xml: &str, version_aware: bool) -> io::Result<Self> {
        let root = parse_xml(xml)?;
        if root.name != "PARAMDEF" {
            return Err(invalid_xml(format!(
                "Expected PARAMDEF root element, found {}",
                root.name
            )));
        }

        let param_type = required_child(&root, "ParamType")?;
        let data_version = parse_child::<i16>(&root, "DataVersion")?
            .or(parse_child::<i16>(&root, "Unk06")?)
            .ok_or_else(|| invalid_xml("Missing DataVersion or Unk06 element"))?;
        let endian = match parse_bool_child(&root, "BigEndian")? {
            true => Endian::Big,
            false => Endian::Little,
        };
        let unicode = parse_bool_child(&root, "Unicode")?;
        let format_version = parse_child::<i16>(&root, "FormatVersion")?
            .or(parse_child::<i16>(&root, "Version")?)
            .ok_or_else(|| invalid_xml("Missing FormatVersion or Version element"))?;

        let fields_node = root
            .child("Fields")
            .ok_or_else(|| invalid_xml("Missing Fields element"))?;
        let mut out = Self {
            data_version,
            param_type,
            endian,
            unicode,
            format_version,
            fields: Vec::new(),
            version_aware,
            basic_fields: format_version == 0,
        };

        for field_node in fields_node.children_named("Field") {
            let (field, removed_version) = deserialize_field(&out, field_node, version_aware)?;
            if version_aware || removed_version.unwrap_or(0) == 0 {
                out.fields.push(field);
            }
        }

        Ok(out)
    }

    /// Serializes this ParamDef to file containing XML data.
    /// 
    /// `include_offsets` set to false by default
    ///
    /// `xml_version` controls the legacy element names used by version 0;
    /// `include_offsets` adds field-layout offset comments.
    pub fn to_xml(&self, path: impl Into<String>, xml_version: u8, include_offsets: bool) -> io::Result<()> {
        let data = self.to_xml_string(xml_version, include_offsets)?;

        fs::write(path.into(), data)
    }

    /// Serializes this ParamDef to string containing XML data.
    ///
    /// `xml_version` controls the legacy element names used by version 0;
    /// `include_offsets` adds field-layout offset comments.
    pub fn to_xml_string(&self, xml_version: u8, include_offsets: bool) -> io::Result<String> {
        if xml_version > CURRENT_XML_VERSION {
            return Err(io::Error::new(
                InvalidData,
                format!("XML version {xml_version} not recognized"),
            ));
        }

        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        xml.push_str(&format!(
            "<PARAMDEF XmlVersion=\"{xml_version}\">\n  <ParamType>{}</ParamType>\n",
            escape_xml(&self.param_type, false)
        ));
        write_element(
            &mut xml,
            if xml_version == 0 {
                "Unk06"
            } else {
                "DataVersion"
            },
            &self.data_version.to_string(),
            1,
        );
        write_element(
            &mut xml,
            "BigEndian",
            if self.endian == Endian::Big {
                "True"
            } else {
                "False"
            },
            1,
        );
        write_element(
            &mut xml,
            "Unicode",
            if self.unicode { "True" } else { "False" },
            1,
        );
        write_element(
            &mut xml,
            if xml_version == 0 {
                "Version"
            } else {
                "FormatVersion"
            },
            &self.format_version.to_string(),
            1,
        );
        xml.push_str("  <Fields>\n");

        let mut offset = 0i64;
        for field in &self.fields {
            let field_size = i64::from(util::get_value_size(field.display_type))
                * i64::from(field.array_length.unwrap_or(1));
            if include_offsets && field_size != 0 {
                xml.push_str(&format!("    <!-- +0x{offset:X} -->\n"));
            }
            write_field(&mut xml, self, field)?;
            offset += field_size;
        }

        xml.push_str("  </Fields>\n</PARAMDEF>\n");
        Ok(xml)
    }
}

fn parse_xml(xml: &str) -> io::Result<XmlNode> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root = None;

    loop {
        let event = reader
            .read_event()
            .map_err(|error| invalid_xml(error.to_string()))?;
        match event {
            Event::Start(element) => stack.push(parse_element(&reader, &element)?),
            Event::Empty(element) => {
                let node = parse_element(&reader, &element)?;
                append_node(&mut stack, &mut root, node)?;
            }
            Event::Text(text) => {
                if let Some(node) = stack.last_mut() {
                    let decoded = text
                        .decode()
                        .map_err(|error| invalid_xml(error.to_string()))?;
                    node.text.push_str(
                        &unescape(&decoded).map_err(|error| invalid_xml(error.to_string()))?,
                    );
                }
            }
            Event::CData(text) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(
                        &text
                            .decode()
                            .map_err(|error| invalid_xml(error.to_string()))?,
                    );
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(node) = stack.last_mut() {
                    let reference = reference
                        .decode()
                        .map_err(|error| invalid_xml(error.to_string()))?;
                    let character = match reference.as_ref() {
                        "amp" => '&',
                        "lt" => '<',
                        "gt" => '>',
                        "apos" => '\'',
                        "quot" => '"',
                        value if value.starts_with("#x") => {
                            let codepoint = u32::from_str_radix(&value[2..], 16)
                                .map_err(|error| invalid_xml(error.to_string()))?;
                            char::from_u32(codepoint)
                                .ok_or_else(|| invalid_xml("Invalid XML character reference"))?
                        }
                        value if value.starts_with('#') => {
                            let codepoint = value[1..]
                                .parse()
                                .map_err(|error| invalid_xml(format!("{error}")))?;
                            char::from_u32(codepoint)
                                .ok_or_else(|| invalid_xml("Invalid XML character reference"))?
                        }
                        unknown => {
                            return Err(invalid_xml(format!(
                                "Unknown XML entity reference: &{unknown};"
                            )));
                        }
                    };
                    node.text.push(character);
                }
            }
            Event::End(_) => {
                let node = stack
                    .pop()
                    .ok_or_else(|| invalid_xml("Unexpected closing element"))?;
                append_node(&mut stack, &mut root, node)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    if !stack.is_empty() {
        return Err(invalid_xml("Unclosed XML element"));
    }
    root.ok_or_else(|| invalid_xml("Missing PARAMDEF root element"))
}

fn parse_element(reader: &Reader<&[u8]>, element: &BytesStart<'_>) -> io::Result<XmlNode> {
    let name = String::from_utf8(element.name().as_ref().to_vec())
        .map_err(|error| invalid_xml(error.to_string()))?;
    let mut node = XmlNode {
        name,
        ..Default::default()
    };
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| invalid_xml(error.to_string()))?;
        let key = String::from_utf8(attribute.key.as_ref().to_vec())
            .map_err(|error| invalid_xml(error.to_string()))?;
        let value = attribute
            .decode_and_unescape_value(reader.decoder())
            .map_err(|error| invalid_xml(error.to_string()))?
            .into_owned();
        node.attributes.push((key, value));
    }
    Ok(node)
}

fn append_node(stack: &mut [XmlNode], root: &mut Option<XmlNode>, node: XmlNode) -> io::Result<()> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else if root.replace(node).is_some() {
        return Err(invalid_xml("XML has more than one root element"));
    }
    Ok(())
}

fn deserialize_field(
    def: &ParamDef,
    node: &XmlNode,
    version_aware: bool,
) -> io::Result<(Field, Option<u64>)> {
    let first_version =
        parse_version_attribute(node, "FirstVersion")?.filter(|version| *version != 0);
    let removed_version =
        parse_version_attribute(node, "RemovedVersion")?.filter(|version| *version != 0);
    let definition = node
        .attribute("Def")
        .ok_or_else(|| invalid_xml("Field is missing its Def attribute"))?;
    let (left, default_text) = match definition.split_once('=') {
        Some((left, default)) => (left.trim(), Some(default.trim())),
        None => (definition.trim(), None),
    };
    let (type_name, field_name) = left
        .split_once(char::is_whitespace)
        .ok_or_else(|| invalid_xml(format!("Invalid field definition: {definition}")))?;
    let display_type = parse_type(type_name.trim())?;
    let mut internal_name = field_name.trim().to_owned();
    if internal_name.is_empty() {
        return Err(invalid_xml(format!(
            "Missing field name in definition: {definition}"
        )));
    }

    let mut bit_size = None;
    let mut array_length = if util::is_array_type(display_type) {
        Some(1)
    } else {
        None
    };
    if util::is_bit_type(display_type) {
        if let Some((name, size)) = internal_name.rsplit_once(':') {
            if !size.is_empty() && size.chars().all(|character| character.is_ascii_digit()) {
                bit_size = Some(
                    size.parse()
                        .map_err(|error| invalid_xml(format!("Invalid bit size: {error}")))?,
                );
                internal_name = name.trim().to_owned();
            }
        }
    }
    if bit_size.is_none() && util::is_array_type(display_type) {
        if let Some(open) = internal_name.rfind('[') {
            if internal_name.ends_with(']') {
                let length = &internal_name[open + 1..internal_name.len() - 1];
                if !length.is_empty() && length.chars().all(|character| character.is_ascii_digit())
                {
                    array_length =
                        Some(length.parse().map_err(|error| {
                            invalid_xml(format!("Invalid array length: {error}"))
                        })?);
                    internal_name = internal_name[..open].trim().to_owned();
                }
            }
        }
    }
    if def.format_version < 102 && internal_name == "unnamed" {
        internal_name.clear();
    }

    let mut field = Field::new(display_type, internal_name.clone());
    field.display_name = child_text(node, "DisplayName").unwrap_or_else(|| internal_name.clone());
    field.internal_name = Some(internal_name);
    field.display_type = display_type;
    field.array_length = array_length;
    field.bit_size = bit_size;
    field.internal_type =
        Some(child_text(node, "Enum").unwrap_or_else(|| type_name.trim().to_owned()));
    field.description = child_text(node, "Description");
    field.display_format =
        child_text(node, "DisplayFormat").unwrap_or_else(|| util::get_default_format(display_type));
    field.edit_flags = child_text(node, "EditFlags")
        .map(|text| parse_edit_flags(&text))
        .transpose()?
        .unwrap_or_else(|| util::get_default_edit_flags(display_type));
    field.default = xml_default_value(def, display_type, ValueKind::Default)?;
    field.min = xml_default_value(def, display_type, ValueKind::Minimum)?;
    field.max = xml_default_value(def, display_type, ValueKind::Maximum)?;
    field.increment = xml_default_value(def, display_type, ValueKind::Increment)?;

    if let Some(text) = default_text {
        field.default = parse_xml_value(def, display_type, text)?;
    }
    if let Some(text) = child_text(node, "Minimum") {
        field.min = parse_xml_value(def, display_type, &text)?;
    }
    if let Some(text) = child_text(node, "Maximum") {
        field.max = parse_xml_value(def, display_type, &text)?;
    }
    if let Some(text) = child_text(node, "Increment") {
        field.increment = parse_xml_value(def, display_type, &text)?;
    }
    field.sort_id = parse_child::<i32>(node, "SortID")?;
    field.unk_b8 = child_text(node, "UnkB8");
    field.unk_c0 = child_text(node, "UnkC0");
    field.unk_c8 = child_text(node, "UnkC8");

    if version_aware {
        field.first_regulation_version = first_version;
        field.removed_regulation_version = removed_version;
    }

    Ok((field, removed_version))
}

#[derive(Clone, Copy)]
enum ValueKind {
    Default,
    Minimum,
    Maximum,
    Increment,
}

fn xml_default_value(
    def: &ParamDef,
    display_type: ParamDefType,
    kind: ValueKind,
) -> io::Result<CellValue> {
    if !def.variable_editor_value_types() {
        let value = match kind {
            ValueKind::Default => 0.0,
            ValueKind::Minimum => match display_type {
                ParamDefType::I8 => i8::MIN as f32,
                ParamDefType::U8 | ParamDefType::U16 | ParamDefType::U32 => 0.0,
                ParamDefType::I16 => i16::MIN as f32,
                ParamDefType::I32 => -2147483520.0,
                ParamDefType::Bool | ParamDefType::ArrayU8 => 0.0,
                ParamDefType::F32 | ParamDefType::Angle | ParamDefType::F64 => f32::MIN,
                ParamDefType::StringShiftJIS | ParamDefType::StringUTF16 => -1.0,
            },
            ValueKind::Maximum => match display_type {
                ParamDefType::I8 => i8::MAX as f32,
                ParamDefType::U8 => u8::MAX as f32,
                ParamDefType::I16 => i16::MAX as f32,
                ParamDefType::U16 => u16::MAX as f32,
                ParamDefType::I32 => 2147483520.0,
                ParamDefType::U32 => 4294967040.0,
                ParamDefType::Bool => 1.0,
                ParamDefType::F32 | ParamDefType::Angle | ParamDefType::F64 => f32::MAX,
                ParamDefType::ArrayU8 => 0.0,
                ParamDefType::StringShiftJIS | ParamDefType::StringUTF16 => 1_000_000_000.0,
            },
            ValueKind::Increment => match display_type {
                ParamDefType::F32 | ParamDefType::Angle | ParamDefType::F64 => 0.01,
                ParamDefType::ArrayU8 => 0.0,
                _ => 1.0,
            },
        };
        return Ok(CellValue::F32(value));
    }

    let value = match (display_type, kind) {
        (ParamDefType::I8, ValueKind::Default) => CellValue::I8(0),
        (ParamDefType::I8, ValueKind::Minimum) => CellValue::I8(i8::MIN),
        (ParamDefType::I8, ValueKind::Maximum) => CellValue::I8(i8::MAX),
        (ParamDefType::I8, ValueKind::Increment) => CellValue::I8(1),
        (ParamDefType::U8, ValueKind::Default) => CellValue::U8(0),
        (ParamDefType::U8, ValueKind::Minimum) => CellValue::U8(u8::MIN),
        (ParamDefType::U8, ValueKind::Maximum) => CellValue::U8(u8::MAX),
        (ParamDefType::U8, ValueKind::Increment) => CellValue::U8(1),
        (ParamDefType::I16, ValueKind::Default) => CellValue::I16(0),
        (ParamDefType::I16, ValueKind::Minimum) => CellValue::I16(i16::MIN),
        (ParamDefType::I16, ValueKind::Maximum) => CellValue::I16(i16::MAX),
        (ParamDefType::I16, ValueKind::Increment) => CellValue::I16(1),
        (ParamDefType::U16, ValueKind::Default) => CellValue::U16(0),
        (ParamDefType::U16, ValueKind::Minimum) => CellValue::U16(u16::MIN),
        (ParamDefType::U16, ValueKind::Maximum) => CellValue::U16(u16::MAX),
        (ParamDefType::U16, ValueKind::Increment) => CellValue::U16(1),
        (ParamDefType::I32, ValueKind::Default) => CellValue::I32(0),
        (ParamDefType::I32, ValueKind::Minimum) => CellValue::I32(i32::MIN),
        (ParamDefType::I32, ValueKind::Maximum) => CellValue::I32(i32::MAX),
        (ParamDefType::I32, ValueKind::Increment) => CellValue::I32(1),
        (ParamDefType::U32, ValueKind::Default) => CellValue::U32(0),
        (ParamDefType::U32, ValueKind::Minimum) => CellValue::U32(0),
        (ParamDefType::U32, ValueKind::Maximum) => CellValue::U32(i32::MAX as u32),
        (ParamDefType::U32, ValueKind::Increment) => CellValue::U32(1),
        (ParamDefType::Bool, ValueKind::Default | ValueKind::Minimum) => CellValue::Bool(false),
        (ParamDefType::Bool, ValueKind::Maximum | ValueKind::Increment) => CellValue::Bool(true),
        (ParamDefType::F32, ValueKind::Default) => CellValue::F32(0.0),
        (ParamDefType::F32, ValueKind::Minimum) => CellValue::F32(f32::MIN),
        (ParamDefType::F32, ValueKind::Maximum) => CellValue::F32(f32::MAX),
        (ParamDefType::F32, ValueKind::Increment) => CellValue::F32(0.01),
        (ParamDefType::Angle, ValueKind::Default) => CellValue::Angle(0.0),
        (ParamDefType::Angle, ValueKind::Minimum) => CellValue::Angle(f32::MIN),
        (ParamDefType::Angle, ValueKind::Maximum) => CellValue::Angle(f32::MAX),
        (ParamDefType::Angle, ValueKind::Increment) => CellValue::Angle(0.01),
        (ParamDefType::F64, ValueKind::Default) => CellValue::F64(0.0),
        (ParamDefType::F64, ValueKind::Minimum) => CellValue::F64(f64::MIN),
        (ParamDefType::F64, ValueKind::Maximum) => CellValue::F64(f64::MAX),
        (ParamDefType::F64, ValueKind::Increment) => CellValue::F64(0.01),
        (ParamDefType::ArrayU8, _) => CellValue::ArrayU8(Vec::new()),
        (ParamDefType::StringShiftJIS, _) => CellValue::StringShiftJIS(String::new()),
        (ParamDefType::StringUTF16, _) => CellValue::StringUTF16(String::new()),
    };
    Ok(value)
}

fn parse_xml_value(
    def: &ParamDef,
    display_type: ParamDefType,
    text: &str,
) -> io::Result<CellValue> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("null") {
        return xml_default_value(def, display_type, ValueKind::Default);
    }
    if !def.variable_editor_value_types() {
        return text.parse::<f32>().map(CellValue::F32).map_err(|error| {
            invalid_xml(format!("Invalid numeric field value {text:?}: {error}"))
        });
    }

    let invalid = |error: &dyn std::fmt::Display| {
        invalid_xml(format!(
            "Invalid value {text:?} for {display_type:?}: {error}"
        ))
    };
    match display_type {
        ParamDefType::I8 => text.parse().map(CellValue::I8).map_err(|e| invalid(&e)),
        ParamDefType::U8 => text.parse().map(CellValue::U8).map_err(|e| invalid(&e)),
        ParamDefType::I16 => text.parse().map(CellValue::I16).map_err(|e| invalid(&e)),
        ParamDefType::U16 => text.parse().map(CellValue::U16).map_err(|e| invalid(&e)),
        ParamDefType::I32 => text.parse().map(CellValue::I32).map_err(|e| invalid(&e)),
        ParamDefType::U32 => text.parse().map(CellValue::U32).map_err(|e| invalid(&e)),
        ParamDefType::Bool => text
            .parse::<i32>()
            .map(|value| CellValue::Bool(value != 0))
            .map_err(|e| invalid(&e)),
        ParamDefType::F32 => text.parse().map(CellValue::F32).map_err(|e| invalid(&e)),
        ParamDefType::Angle => text.parse().map(CellValue::Angle).map_err(|e| invalid(&e)),
        ParamDefType::F64 => text.parse().map(CellValue::F64).map_err(|e| invalid(&e)),
        ParamDefType::ArrayU8 => Ok(CellValue::ArrayU8(Vec::new())),
        ParamDefType::StringShiftJIS => Ok(CellValue::StringShiftJIS(String::new())),
        ParamDefType::StringUTF16 => Ok(CellValue::StringUTF16(String::new())),
    }
}

fn write_field(xml: &mut String, def: &ParamDef, field: &Field) -> io::Result<()> {
    let type_name = type_name(field.display_type);
    let internal_name = field
        .internal_name
        .as_deref()
        .filter(|name| !name.is_empty())
        .unwrap_or("unnamed");
    let mut definition = format!("{type_name} {internal_name}");
    if let Some(bit_size) = field
        .bit_size
        .filter(|_| util::is_bit_type(field.display_type))
    {
        definition.push_str(&format!(":{bit_size}"));
    } else if util::is_array_type(field.display_type) {
        if let Some(array_length) = field.array_length {
            definition.push_str(&format!("[{array_length}]"));
        }
    }
    let default = xml_default_value(def, field.display_type, ValueKind::Default)?;
    if !same_xml_value(def, field.display_type, &field.default, &default)? {
        definition.push_str(" = ");
        definition.push_str(&format_xml_value(def, field.display_type, &field.default)?);
    }

    xml.push_str("    <Field Def=\"");
    xml.push_str(&escape_xml(&definition, true));
    xml.push('"');
    if def.version_aware {
        if let Some(version) = field
            .first_regulation_version
            .filter(|version| *version != 0)
        {
            xml.push_str(&format!(" FirstVersion=\"{version}\""));
        }
        if let Some(version) = field
            .removed_regulation_version
            .filter(|version| *version != 0)
        {
            xml.push_str(&format!(" RemovedVersion=\"{version}\""));
        }
    }
    xml.push_str(">\n");

    let display_name_default = field.internal_name.as_deref().unwrap_or_default();
    write_default_element(
        xml,
        "DisplayName",
        &field.display_name,
        display_name_default,
        3,
    );
    write_default_element(
        xml,
        "Enum",
        field.internal_type.as_deref().unwrap_or(type_name),
        type_name,
        3,
    );
    if let Some(description) = field.description.as_deref() {
        write_element(xml, "Description", description, 3);
    }
    write_default_element(
        xml,
        "DisplayFormat",
        &field.display_format,
        &util::get_default_format(field.display_type),
        3,
    );
    write_default_element(
        xml,
        "EditFlags",
        &format_edit_flags(field.edit_flags)?,
        &format_edit_flags(util::get_default_edit_flags(field.display_type))?,
        3,
    );
    write_variable_element(xml, def, field, "Minimum", &field.min, ValueKind::Minimum)?;
    write_variable_element(xml, def, field, "Maximum", &field.max, ValueKind::Maximum)?;
    write_variable_element(
        xml,
        def,
        field,
        "Increment",
        &field.increment,
        ValueKind::Increment,
    )?;
    write_default_element(
        xml,
        "SortID",
        &field.sort_id.unwrap_or_default().to_string(),
        "0",
        3,
    );
    for (tag, value) in [
        ("UnkB8", field.unk_b8.as_deref()),
        ("UnkC0", field.unk_c0.as_deref()),
        ("UnkC8", field.unk_c8.as_deref()),
    ] {
        if let Some(value) = value {
            write_element(xml, tag, value, 3);
        }
    }
    xml.push_str("    </Field>\n");
    Ok(())
}

fn write_variable_element(
    xml: &mut String,
    def: &ParamDef,
    field: &Field,
    tag: &str,
    value: &CellValue,
    kind: ValueKind,
) -> io::Result<()> {
    if def.variable_editor_value_types()
        && matches!(
            field.display_type,
            ParamDefType::ArrayU8 | ParamDefType::StringShiftJIS | ParamDefType::StringUTF16
        )
    {
        return Ok(());
    }
    let default = xml_default_value(def, field.display_type, kind)?;
    if !same_xml_value(def, field.display_type, value, &default)? {
        write_element(
            xml,
            tag,
            &format_xml_value(def, field.display_type, value)?,
            3,
        );
    }
    Ok(())
}

fn format_xml_value(
    def: &ParamDef,
    display_type: ParamDefType,
    value: &CellValue,
) -> io::Result<String> {
    if !def.variable_editor_value_types() {
        return Ok(fixed_value_as_f32(value)?.to_string());
    }
    match (display_type, value) {
        (ParamDefType::I8, CellValue::I8(value)) => Ok(value.to_string()),
        (ParamDefType::U8, CellValue::U8(value)) => Ok(value.to_string()),
        (ParamDefType::I16, CellValue::I16(value)) => Ok(value.to_string()),
        (ParamDefType::U16, CellValue::U16(value)) => Ok(value.to_string()),
        (ParamDefType::I32, CellValue::I32(value)) => Ok(value.to_string()),
        (ParamDefType::U32, CellValue::U32(value)) => Ok(value.to_string()),
        (ParamDefType::Bool, CellValue::Bool(value)) => {
            Ok(if *value { "1" } else { "0" }.to_owned())
        }
        (ParamDefType::F32, CellValue::F32(value))
        | (ParamDefType::Angle, CellValue::Angle(value)) => Ok(value.to_string()),
        (ParamDefType::F64, CellValue::F64(value)) => Ok(value.to_string()),
        (ParamDefType::ArrayU8 | ParamDefType::StringShiftJIS | ParamDefType::StringUTF16, _) => {
            Err(invalid_xml(format!(
                "No scalar XML value exists for {display_type:?}"
            )))
        }
        _ => Err(invalid_xml(format!(
            "Value type does not match field type {display_type:?}"
        ))),
    }
}

fn fixed_value_as_f32(value: &CellValue) -> io::Result<f32> {
    match value {
        CellValue::I8(value) => Ok(*value as f32),
        CellValue::U8(value) => Ok(*value as f32),
        CellValue::I16(value) => Ok(*value as f32),
        CellValue::U16(value) => Ok(*value as f32),
        CellValue::I32(value) => Ok(*value as f32),
        CellValue::U32(value) => Ok(*value as f32),
        CellValue::Bool(value) => Ok(if *value { 1.0 } else { 0.0 }),
        CellValue::F32(value) | CellValue::Angle(value) => Ok(*value),
        CellValue::F64(value) => Ok(*value as f32),
        CellValue::ArrayU8(value) if value.is_empty() => Ok(0.0),
        CellValue::StringShiftJIS(value) | CellValue::StringUTF16(value) if value.is_empty() => {
            Ok(0.0)
        }
        _ => Err(invalid_xml(
            "Non-empty array/string value cannot be written as a fixed XML numeric value",
        )),
    }
}

fn same_xml_value(
    def: &ParamDef,
    display_type: ParamDefType,
    value: &CellValue,
    default: &CellValue,
) -> io::Result<bool> {
    if def.variable_editor_value_types()
        && matches!(
            display_type,
            ParamDefType::ArrayU8 | ParamDefType::StringShiftJIS | ParamDefType::StringUTF16
        )
    {
        return Ok(true);
    }
    if def.variable_editor_value_types() {
        Ok(format_xml_value(def, display_type, value)?
            == format_xml_value(def, display_type, default)?)
    } else {
        Ok(fixed_value_as_f32(value)? == fixed_value_as_f32(default)?)
    }
}

fn parse_type(type_name: &str) -> io::Result<ParamDefType> {
    match type_name {
        "s8" => Ok(ParamDefType::I8),
        "u8" => Ok(ParamDefType::U8),
        "s16" => Ok(ParamDefType::I16),
        "u16" => Ok(ParamDefType::U16),
        "s32" => Ok(ParamDefType::I32),
        "u32" => Ok(ParamDefType::U32),
        "b32" => Ok(ParamDefType::Bool),
        "f32" => Ok(ParamDefType::F32),
        "angle32" => Ok(ParamDefType::Angle),
        "f64" => Ok(ParamDefType::F64),
        "dummy8" => Ok(ParamDefType::ArrayU8),
        "fixstr" => Ok(ParamDefType::StringShiftJIS),
        "fixstrW" => Ok(ParamDefType::StringUTF16),
        _ => Err(invalid_xml(format!(
            "Unknown ParamDef field type: {type_name}"
        ))),
    }
}

fn type_name(display_type: ParamDefType) -> &'static str {
    match display_type {
        ParamDefType::I8 => "s8",
        ParamDefType::U8 => "u8",
        ParamDefType::I16 => "s16",
        ParamDefType::U16 => "u16",
        ParamDefType::I32 => "s32",
        ParamDefType::U32 => "u32",
        ParamDefType::Bool => "b32",
        ParamDefType::F32 => "f32",
        ParamDefType::Angle => "angle32",
        ParamDefType::F64 => "f64",
        ParamDefType::ArrayU8 => "dummy8",
        ParamDefType::StringShiftJIS => "fixstr",
        ParamDefType::StringUTF16 => "fixstrW",
    }
}

fn parse_edit_flags(text: &str) -> io::Result<EditFlags> {
    let text = text.trim();
    if text == "None" || text.is_empty() {
        return Ok(EditFlags::empty());
    }
    let mut flags = EditFlags::empty();
    for name in text.split([',', '|']) {
        match name.trim() {
            "Wrap" => flags |= EditFlags::Wrap,
            "Lock" => flags |= EditFlags::Lock,
            unknown => return Err(invalid_xml(format!("Unknown EditFlags value: {unknown}"))),
        }
    }
    Ok(flags)
}

fn format_edit_flags(flags: EditFlags) -> io::Result<String> {
    if flags.is_empty() {
        return Ok("None".to_owned());
    }
    if flags.bits() & !EditFlags::all().bits() != 0 {
        return Err(invalid_xml(format!(
            "Unknown EditFlags bits: 0x{:X}",
            flags.bits()
        )));
    }
    let mut values = Vec::new();
    if flags.contains(EditFlags::Wrap) {
        values.push("Wrap");
    }
    if flags.contains(EditFlags::Lock) {
        values.push("Lock");
    }
    Ok(values.join(", "))
}

fn required_child(node: &XmlNode, name: &str) -> io::Result<String> {
    child_text(node, name).ok_or_else(|| invalid_xml(format!("Missing {name} element")))
}

fn child_text(node: &XmlNode, name: &str) -> Option<String> {
    node.child(name).map(XmlNode::content)
}

fn parse_child<T>(node: &XmlNode, name: &str) -> io::Result<Option<T>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    child_text(node, name)
        .map(|text| {
            text.trim()
                .parse()
                .map_err(|error| invalid_xml(format!("Invalid {name} value: {error}")))
        })
        .transpose()
}

fn parse_bool_child(node: &XmlNode, name: &str) -> io::Result<bool> {
    let text = required_child(node, name)?;
    match text.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(invalid_xml(format!("Invalid {name} value: {text}"))),
    }
}

fn parse_version_attribute(node: &XmlNode, name: &str) -> io::Result<Option<u64>> {
    node.attribute(name)
        .map(|text| {
            text.parse()
                .map_err(|error| invalid_xml(format!("Invalid {name} attribute: {error}")))
        })
        .transpose()
}

fn write_default_element(xml: &mut String, tag: &str, value: &str, default: &str, indent: usize) {
    if value != default {
        write_element(xml, tag, value, indent);
    }
}

fn write_element(xml: &mut String, tag: &str, value: &str, indent: usize) {
    xml.push_str(&" ".repeat(indent * 2));
    xml.push('<');
    xml.push_str(tag);
    xml.push('>');
    xml.push_str(&escape_xml(value, false));
    xml.push_str("</");
    xml.push_str(tag);
    xml.push_str(">\n");
}

fn escape_xml(text: &str, attribute: bool) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' if attribute => escaped.push_str("&quot;"),
            '\'' if attribute => escaped.push_str("&apos;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn invalid_xml(message: impl Into<String>) -> io::Error {
    io::Error::new(InvalidData, message.into())
}
