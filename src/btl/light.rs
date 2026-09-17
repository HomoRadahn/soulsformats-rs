use std::io::{self, Read, Seek, Write};

use crate::{
    io::{BinaryReader, BinaryWriter, ByteVector3, ByteVector4, Vector3},
    util,
};

#[derive(Default, Clone, PartialEq, Debug)]
/// Type of a light source
pub enum LightType {
    #[default]
    /// Omnidirectional light
    Point = 0,
    /// Cone of light
    Spot = 1,
    /// Light at a constant angle
    Directional = 2,
}

impl TryFrom<u32> for LightType {
    type Error = io::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Point),
            1 => Ok(Self::Spot),
            2 => Ok(Self::Directional),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid enum value",
            )),
        }
    }
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Light {
    pub unk_00: Vec<u8>,
    /// Name of this light
    pub name: String,
    /// Type of this light
    pub light_type: LightType,
    pub unk_1c: bool,
    /// Color of the light on diffuse surfaces
    pub diffuse_color: ByteVector3,
    /// Intensity of diffuse light
    pub diffuse_power: f32,
    /// Color of the light on reflective surfaces
    pub specular_color: ByteVector3,
    /// Whether the light casts shadows
    pub cast_shadows: bool,
    /// Intensity of specular lighting
    pub specular_power: f32,
    /// Tightness of the spot light beam
    pub cone_angle: f32,
    pub unk_30: f32,
    pub unk_34: f32,
    /// Center of the light
    pub position: Vector3,
    /// Rotation of a spot light
    pub rotation: Vector3,
    pub unk_50: i32,
    pub unk_54: f32,
    /// Distance the light shines
    pub radius: f32,
    pub unk_5c: i32,
    pub unk_64: Vec<u8>,
    pub unk_68: f32,
    /// Color of shadows cast by the light
    pub shadow_color: ByteVector4,
    pub unk_70: f32,
    /// Minimum time between flickers
    pub flicker_interval_min: f32,
    /// Maximum time between flickers
    pub flicker_interval_max: f32,
    /// Multiplies the brightness of the light while flickering
    pub flicker_brightness_multiplier: f32,
    pub unk_80: i32,
    pub unk_84: Vec<u8>,
    pub unk_88: f32,
    pub unk_90: f32,
    pub unk_98: f32,
    /// Distance at which spot light beam starts
    pub near_clip: f32,
    pub unk_a0: Vec<u8>,
    /// Unknown
    pub sharpness: f32,
    pub unk_ac: f32,
    /// Stretches the spot light beam
    pub width: f32,
    pub unk_bc: f32,
    pub unk_c0: Vec<u8>,
    pub unk_c4: f32,
    /// Not present before Sekiro
    pub unk_c8: Option<f32>,
    /// Not present before Sekiro
    pub unk_cc: Option<f32>,
    /// Not present before Sekiro
    pub unk_d0: Option<f32>,
    /// Not present before Sekiro
    pub unk_d4: Option<f32>,
    /// Not present before Sekiro
    pub unk_d8: Option<f32>,
    /// Not present before Sekiro
    pub unk_dc: Option<i32>,
    /// Not present before Sekiro
    pub unk_e0: Option<f32>,
    /// Not present before Sekiro
    pub unk_e4: Option<i32>,
}

impl Light {
    /// Creates a new light with default values
    pub fn new() -> Self {
        Self {
            unk_00: vec![0 as u8; 16],
            name: "".to_string(),
            diffuse_color: ByteVector3 {
                x: 255,
                y: 255,
                z: 255,
            },
            diffuse_power: 1.0,
            specular_color: ByteVector3 {
                x: 255,
                y: 255,
                z: 255,
            },
            specular_power: 1.0,
            unk_50: 4,
            radius: 10.0,
            unk_5c: -1,
            unk_64: vec![0, 0, 0, 1],
            shadow_color: ByteVector4 {
                x: 0,
                y: 0,
                z: 0,
                w: 100,
            },
            flicker_brightness_multiplier: 1.0,
            unk_80: -1,
            unk_84: vec![0, 0, 0, 0],
            unk_98: 1.0,
            near_clip: 1.0,
            unk_a0: vec![1, 0, 2, 1],
            sharpness: 1.0,
            unk_c0: vec![0, 0, 0, 0],
            ..Default::default()
        }
    }

    /// Reads the `Light` from the supplied `BinaryReader`
    pub(crate) fn read<R>(
        br: &mut BinaryReader<R>,
        names_start: i64,
        version: i32,
    ) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let mut output = Self::new();
        output.unk_00 = br.read_u8_vec(16)?;
        let varint = br.read_varint()?;
        output.name = br.get_utf16(util::try_from_to_io_result(names_start + varint)?)?;
        output.light_type = br.read_enum_u32::<LightType>()?;
        output.unk_1c = br.read_bool()?;
        output.diffuse_color = br.read_byte_vector_3()?;
        output.diffuse_power = br.read_f32()?;
        output.specular_color = br.read_byte_vector_3()?;
        output.cast_shadows = br.read_bool()?;
        output.specular_power = br.read_f32()?;
        output.cone_angle = br.read_f32()?;
        output.unk_30 = br.read_f32()?;
        output.unk_34 = br.read_f32()?;
        output.position = br.read_vector_3()?;
        output.rotation = br.read_vector_3()?;
        output.unk_50 = br.read_i32()?;
        output.unk_54 = br.read_f32()?;
        output.radius = br.read_f32()?;
        output.unk_5c = br.read_i32()?;
        br.assert_i32(&[0])?;
        output.unk_64 = br.read_u8_vec(4)?;
        output.unk_68 = br.read_f32()?;
        output.shadow_color = br.read_byte_vector_4_rgba()?;
        output.unk_70 = br.read_f32()?;
        output.flicker_interval_min = br.read_f32()?;
        output.flicker_interval_max = br.read_f32()?;
        output.flicker_brightness_multiplier = br.read_f32()?;
        output.unk_80 = br.read_i32()?;
        output.unk_84 = br.read_u8_vec(4)?;
        output.unk_88 = br.read_f32()?;
        br.assert_i32(&[0])?;
        output.unk_90 = br.read_f32()?;
        br.assert_i32(&[0])?;
        output.unk_98 = br.read_f32()?;
        output.near_clip = br.read_f32()?;
        output.unk_a0 = br.read_u8_vec(4)?;
        output.sharpness = br.read_f32()?;
        br.assert_i32(&[0])?;
        output.unk_ac = br.read_f32()?;
        br.assert_varint(&[0])?;
        output.width = br.read_f32()?;
        output.unk_bc = br.read_f32()?;
        output.unk_c0 = br.read_u8_vec(4)?;
        output.unk_c4 = br.read_f32()?;

        if version >= 16 {
            output.unk_c8 = Some(br.read_f32()?);
            output.unk_cc = Some(br.read_f32()?);
            output.unk_d0 = Some(br.read_f32()?);
            output.unk_d4 = Some(br.read_f32()?);
            output.unk_d8 = Some(br.read_f32()?);
            output.unk_dc = Some(br.read_i32()?);
            output.unk_e0 = Some(br.read_f32()?);
            output.unk_e4 = Some(br.read_i32()?);
        } else {
            output.unk_c8 = None;
            output.unk_cc = None;
            output.unk_d0 = None;
            output.unk_d4 = None;
            output.unk_d8 = None;
            output.unk_dc = None;
            output.unk_e0 = None;
            output.unk_e4 = None;
        }

        Ok(output)
    }

    pub(crate) fn write<W>(&self, bw: &mut BinaryWriter<W>, name_offset: i64) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let out = self.clone();
        bw.write_u8_vec(out.unk_00)?;
        bw.write_varint(name_offset)?;
        bw.write_u32(out.light_type as u32)?;
        bw.write_bool(out.unk_1c)?;
        bw.write_byte_vector3(out.diffuse_color)?;
        bw.write_f32(out.diffuse_power)?;
        bw.write_byte_vector3(out.specular_color)?;
        bw.write_bool(out.cast_shadows)?;
        bw.write_f32(out.specular_power)?;
        bw.write_f32(out.cone_angle)?;
        bw.write_f32(out.unk_30)?;
        bw.write_f32(out.unk_34)?;
        bw.write_vector3(out.position)?;
        bw.write_vector3(out.rotation)?;
        bw.write_i32(out.unk_50)?;
        bw.write_f32(out.unk_54)?;
        bw.write_f32(out.radius)?;
        bw.write_i32(out.unk_5c)?;
        bw.write_i32(0)?;
        bw.write_u8_vec(out.unk_64)?;
        bw.write_f32(out.unk_68)?;
        bw.write_byte_vector4_rgba(out.shadow_color)?;
        bw.write_f32(out.unk_70)?;
        bw.write_f32(out.flicker_interval_min)?;
        bw.write_f32(out.flicker_interval_max)?;
        bw.write_f32(out.flicker_brightness_multiplier)?;
        bw.write_i32(out.unk_80)?;
        bw.write_u8_vec(out.unk_84)?;
        bw.write_f32(out.unk_88)?;
        bw.write_i32(0)?;
        bw.write_f32(out.unk_90)?;
        bw.write_i32(0)?;
        bw.write_f32(out.unk_98)?;
        bw.write_f32(out.near_clip)?;
        bw.write_u8_vec(out.unk_a0)?;
        bw.write_f32(out.sharpness)?;
        bw.write_i32(0)?;
        bw.write_f32(out.unk_ac)?;
        bw.write_varint(0)?;
        bw.write_f32(out.width)?;
        bw.write_f32(out.unk_bc)?;
        bw.write_u8_vec(out.unk_c0)?;
        bw.write_f32(out.unk_c4)?;

        match out.unk_c8 {
            Some(value) => bw.write_f32(value)?,
            None => (),
        };

        match out.unk_cc {
            Some(value) => bw.write_f32(value)?,
            None => (),
        };

        match out.unk_d0 {
            Some(value) => bw.write_f32(value)?,
            None => (),
        };

        match out.unk_d4 {
            Some(value) => bw.write_f32(value)?,
            None => (),
        };

        match out.unk_d8 {
            Some(value) => bw.write_f32(value)?,
            None => (),
        };

        match out.unk_dc {
            Some(value) => bw.write_i32(value)?,
            None => (),
        };

        match out.unk_e0 {
            Some(value) => bw.write_f32(value)?,
            None => (),
        };

        match out.unk_e4 {
            Some(value) => bw.write_i32(value)?,
            None => (),
        };

        Ok(())
    }
}
