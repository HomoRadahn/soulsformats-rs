use core::fmt;
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

#[derive(Clone, PartialEq, Debug)]
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

impl Default for Light {
    fn default() -> Self {
        Self {
            unk_00: vec![0_u8; 16],
            name: "".to_string(),
            light_type: LightType::default(),
            unk_1c: false,
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
            cast_shadows: false,
            specular_power: 1.0,
            cone_angle: 0.0,
            unk_30: 0.0,
            unk_34: 0.0,
            position: Vector3::default(),
            rotation: Vector3::default(),
            unk_50: 4,
            unk_54: 0.0,
            radius: 10.0,
            unk_5c: -1,
            unk_64: vec![0, 0, 0, 1],
            unk_68: 0.0,
            shadow_color: ByteVector4 {
                x: 0,
                y: 0,
                z: 0,
                w: 100,
            },
            unk_70: 0.0,
            flicker_interval_min: 0.0,
            flicker_interval_max: 0.0,
            flicker_brightness_multiplier: 1.0,
            unk_80: -1,
            unk_84: vec![0, 0, 0, 0],
            unk_88: 0.0,
            unk_90: 0.0,
            unk_98: 1.0,
            near_clip: 1.0,
            unk_a0: vec![1, 0, 2, 1],
            sharpness: 1.0,
            unk_ac: 0.0,
            width: 0.0,
            unk_bc: 0.0,
            unk_c0: vec![0, 0, 0, 0],
            unk_c4: 0.0,
            unk_c8: None,
            unk_cc: None,
            unk_d0: None,
            unk_d4: None,
            unk_d8: None,
            unk_dc: None,
            unk_e0: None,
            unk_e4: None,
        }
    }
}

impl Light {
    /// Reads the `Light` from the supplied `BinaryReader`
    pub(crate) fn read<R>(
        br: &mut BinaryReader<R>,
        names_start: i64,
        version: i32,
    ) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let mut out = Self::default();
        out.unk_00 = br.read_vec_u8(16)?;
        let varint = br.read_varint()?;
        out.name = br.get_utf16(util::convert_num(names_start + varint)?)?;
        out.light_type = br.read_enum_u32::<LightType>()?;
        out.unk_1c = br.read_bool()?;
        out.diffuse_color = br.read_byte_vector_3()?;
        out.diffuse_power = br.read_f32()?;
        out.specular_color = br.read_byte_vector_3()?;
        out.cast_shadows = br.read_bool()?;
        out.specular_power = br.read_f32()?;
        out.cone_angle = br.read_f32()?;
        out.unk_30 = br.read_f32()?;
        out.unk_34 = br.read_f32()?;
        out.position = br.read_vector_3()?;
        out.rotation = br.read_vector_3()?;
        out.unk_50 = br.read_i32()?;
        out.unk_54 = br.read_f32()?;
        out.radius = br.read_f32()?;
        out.unk_5c = br.read_i32()?;
        br.assert_i32(&[0])?;
        out.unk_64 = br.read_vec_u8(4)?;
        out.unk_68 = br.read_f32()?;
        out.shadow_color = br.read_byte_vector_4_rgba()?;
        out.unk_70 = br.read_f32()?;
        out.flicker_interval_min = br.read_f32()?;
        out.flicker_interval_max = br.read_f32()?;
        out.flicker_brightness_multiplier = br.read_f32()?;
        out.unk_80 = br.read_i32()?;
        out.unk_84 = br.read_vec_u8(4)?;
        out.unk_88 = br.read_f32()?;
        br.assert_i32(&[0])?;
        out.unk_90 = br.read_f32()?;
        br.assert_i32(&[0])?;
        out.unk_98 = br.read_f32()?;
        out.near_clip = br.read_f32()?;
        out.unk_a0 = br.read_vec_u8(4)?;
        out.sharpness = br.read_f32()?;
        br.assert_i32(&[0])?;
        out.unk_ac = br.read_f32()?;
        br.assert_varint(&[0])?;
        out.width = br.read_f32()?;
        out.unk_bc = br.read_f32()?;
        out.unk_c0 = br.read_vec_u8(4)?;
        out.unk_c4 = br.read_f32()?;

        if version >= 16 {
            out.unk_c8 = Some(br.read_f32()?);
            out.unk_cc = Some(br.read_f32()?);
            out.unk_d0 = Some(br.read_f32()?);
            out.unk_d4 = Some(br.read_f32()?);
            out.unk_d8 = Some(br.read_f32()?);
            out.unk_dc = Some(br.read_i32()?);
            out.unk_e0 = Some(br.read_f32()?);
            out.unk_e4 = Some(br.read_i32()?);
        } else {
            out.unk_c8 = None;
            out.unk_cc = None;
            out.unk_d0 = None;
            out.unk_d4 = None;
            out.unk_d8 = None;
            out.unk_dc = None;
            out.unk_e0 = None;
            out.unk_e4 = None;
        }

        Ok(out)
    }

    pub(crate) fn write<W>(&self, bw: &mut BinaryWriter<W>, name_offset: i64) -> io::Result<()>
    where
        W: Write + Seek,
    {
        let out = self.clone();
        bw.write_vec_u8(out.unk_00)?;
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
        bw.write_vec_u8(out.unk_64)?;
        bw.write_f32(out.unk_68)?;
        bw.write_byte_vector4_rgba(out.shadow_color)?;
        bw.write_f32(out.unk_70)?;
        bw.write_f32(out.flicker_interval_min)?;
        bw.write_f32(out.flicker_interval_max)?;
        bw.write_f32(out.flicker_brightness_multiplier)?;
        bw.write_i32(out.unk_80)?;
        bw.write_vec_u8(out.unk_84)?;
        bw.write_f32(out.unk_88)?;
        bw.write_i32(0)?;
        bw.write_f32(out.unk_90)?;
        bw.write_i32(0)?;
        bw.write_f32(out.unk_98)?;
        bw.write_f32(out.near_clip)?;
        bw.write_vec_u8(out.unk_a0)?;
        bw.write_f32(out.sharpness)?;
        bw.write_i32(0)?;
        bw.write_f32(out.unk_ac)?;
        bw.write_varint(0)?;
        bw.write_f32(out.width)?;
        bw.write_f32(out.unk_bc)?;
        bw.write_vec_u8(out.unk_c0)?;
        bw.write_f32(out.unk_c4)?;

        if let Some(value) = out.unk_c8 {
            bw.write_f32(value)?
        }

        if let Some(value) = out.unk_cc {
            bw.write_f32(value)?
        }

        if let Some(value) = out.unk_d0 {
            bw.write_f32(value)?
        }

        if let Some(value) = out.unk_d4 {
            bw.write_f32(value)?
        }

        if let Some(value) = out.unk_d8 {
            bw.write_f32(value)?
        }

        if let Some(value) = out.unk_dc {
            bw.write_i32(value)?
        }

        if let Some(value) = out.unk_e0 {
            bw.write_f32(value)?
        }

        if let Some(value) = out.unk_e4 {
            bw.write_i32(value)?
        }

        Ok(())
    }
}

impl fmt::Display for Light {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
