use soulsformats_rs::io::{BinaryReader, Endian};
use std::{fs, io, vec};

    macro_rules! define_enum_fixture {
        ($name:ident, $type:ty) => {
            #[repr($type)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            enum $name {
                Zero = 0,
                One = 1,
            }

            impl TryFrom<$type> for $name {
                type Error = io::Error;

                fn try_from(value: $type) -> Result<Self, Self::Error> {
                    match value {
                        0 => Ok(Self::Zero),
                        1 => Ok(Self::One),
                        _ => Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "invalid enum value",
                        )),
                    }
                }
            }
        };
    }

    define_enum_fixture!(EnumU8, u8);
    define_enum_fixture!(EnumU16, u16);
    define_enum_fixture!(EnumU32, u32);
    define_enum_fixture!(EnumU64, u64);
    define_enum_fixture!(EnumI8, i8);
    define_enum_fixture!(EnumI16, i16);
    define_enum_fixture!(EnumI32, i32);
    define_enum_fixture!(EnumI64, i64);

    #[test]
    fn read_boolean() {
        for endian in [Endian::Little, Endian::Big] {
            let byte_true = vec![0x1];
            let byte_false = vec![0x0];
            let mut reader_true = BinaryReader::from_bytes(byte_true, endian, true);
            let mut reader_false = BinaryReader::from_bytes(byte_false, endian, true);
            assert!(reader_true.read_bool().unwrap());
            assert!(!reader_false.read_bool().unwrap());

            let bytes = vec![0x1, 0x0, 0x1];
            let mut vec_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(vec_reader.read_bool_vec(3).unwrap(), vec![true, false, true]);

            let mut get_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert!(!get_reader.get_bool(1).unwrap());
            assert_eq!(0, get_reader.position().unwrap());
            assert_eq!(get_reader.get_bool_vec(1, 2).unwrap(), vec![false, true]);
            assert_eq!(0, get_reader.position().unwrap());
        }
    }

    #[test]
    fn read_from_file() {
        let path = std::env::temp_dir().join(format!(
            "soulsformats-rs-reader-{}.bin",
            std::process::id()
        ));
        fs::write(&path, [0x12, 0x13]).unwrap();

        let mut reader = BinaryReader::from_file(&path, Endian::Little, true).unwrap();
        assert_eq!(reader.read_u16().unwrap(), 0x1312);
        assert_eq!(reader.position().unwrap(), 2);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn read_uint8() {
        for endian in [Endian::Little, Endian::Big] {
            let byte = vec![0x12];

            let mut reader = BinaryReader::from_bytes(byte, endian, true);
            assert_eq!(reader.read_u8().unwrap(), 0x12);

            let bytes = vec![0x12, 0x13, 0x14];
            let mut vec_reader = BinaryReader::from_bytes(bytes.clone(), endian,true);
            assert_eq!(vec_reader.read_u8_vec(3).unwrap(), vec![0x12, 0x13, 0x14]);

            let mut get_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(get_reader.get_u8(2).unwrap(), 0x14);
            assert_eq!(0, get_reader.position().unwrap());
            assert_eq!(get_reader.get_u8_vec(1, 2).unwrap(), vec![0x13, 0x14]);
            assert_eq!(0, get_reader.position().unwrap());
        }
    }

    #[test]
    fn read_uint16() {
        let single = vec![0x12, 0x13];
        let many = vec![0x12, 0x13, 0x14, 0x15, 0x16, 0x17];

        let mut little_reader = BinaryReader::from_bytes(single.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_u16().unwrap(), 0x1312);

        let mut little_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(little_vec_reader.read_u16_vec(2).unwrap(), vec![0x1312, 0x1514]);

        let mut little_get_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_u16(2).unwrap(), 0x1514);
        assert_eq!(0, little_get_reader.position().unwrap());
        assert_eq!(little_get_reader.get_u16_vec(1, 2).unwrap(), vec![0x1413, 0x1615]);
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(single.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_u16().unwrap(), 0x1213);

        let mut big_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Big, true);
        assert_eq!(big_vec_reader.read_u16_vec(2).unwrap(), vec![0x1213, 0x1415]);

        let mut big_get_reader = BinaryReader::from_bytes(many.clone(), Endian::Big, true);
        assert_eq!(big_get_reader.get_u16(2).unwrap(), 0x1415);
        assert_eq!(0, big_get_reader.position().unwrap());
        assert_eq!(big_get_reader.get_u16_vec(1, 2).unwrap(), vec![0x1314, 0x1516]);
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_uint32() {
        let single = vec![0x12, 0x13, 0x14, 0x15];
        let many = vec![0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x20, 0x21, 0x22, 0x23];

        let mut little_reader = BinaryReader::from_bytes(single.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_u32().unwrap(), 0x15141312);

        let mut little_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(little_vec_reader.read_u32_vec(2).unwrap(), vec![0x15141312, 0x19181716]);

        let mut little_get_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_u32(2).unwrap(), 0x17161514);
        assert_eq!(0, little_get_reader.position().unwrap());
        assert_eq!(little_get_reader.get_u32_vec(1, 2).unwrap(), vec![0x16151413, 0x20191817]);
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(single.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_u32().unwrap(), 0x12131415);

        let mut big_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Big, true);
        assert_eq!(big_vec_reader.read_u32_vec(2).unwrap(), vec![0x12131415, 0x16171819]);

        let mut big_get_reader = BinaryReader::from_bytes(many.clone(), Endian::Big, true);
        assert_eq!(big_get_reader.get_u32(2).unwrap(), 0x14151617);
        assert_eq!(0, big_get_reader.position().unwrap());
        assert_eq!(big_get_reader.get_u32_vec(1, 2).unwrap(), vec![0x13141516, 0x17181920]);
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_uint64() {
        let single = vec![0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19];
        let many = vec![
            0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19,
            0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27,
        ];

        let mut little_reader = BinaryReader::from_bytes(single.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_u64().unwrap(), 0x1918171615141312);

        let mut little_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(
            little_vec_reader.read_u64_vec(2).unwrap(),
            vec![0x1918171615141312, 0x2726252423222120]
        );

        let mut little_get_reader = BinaryReader::from_bytes(many.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_u64(8).unwrap(), 0x2726252423222120);
        assert_eq!(0, little_get_reader.position().unwrap());
        assert_eq!(
            little_get_reader.get_u64_vec(0, 2).unwrap(),
            vec![0x1918171615141312, 0x2726252423222120]
        );
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(single.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_u64().unwrap(), 0x1213141516171819);

        let mut big_vec_reader = BinaryReader::from_bytes(many.clone(), Endian::Big, true);
        assert_eq!(
            big_vec_reader.read_u64_vec(2).unwrap(),
            vec![0x1213141516171819, 0x2021222324252627]
        );

        let mut big_get_reader = BinaryReader::from_bytes(many, Endian::Big, true);
        assert_eq!(big_get_reader.get_u64(8).unwrap(), 0x2021222324252627);
        assert_eq!(0, big_get_reader.position().unwrap());
        assert_eq!(
            big_get_reader.get_u64_vec(0, 2).unwrap(),
            vec![0x1213141516171819, 0x2021222324252627]
        );
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_int8() {
        let bytes = vec![0xFE, 0x02, 0x7F];

        for endian in [Endian::Little, Endian::Big] {
            let mut reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(reader.read_i8().unwrap(), -2);

            let mut vec_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(vec_reader.read_i8_vec(3).unwrap(), vec![-2, 2, 127]);

            let mut get_reader = BinaryReader::from_bytes(bytes.clone(), endian, true);
            assert_eq!(get_reader.get_i8(1).unwrap(), 2);
            assert_eq!(0, get_reader.position().unwrap());
            assert_eq!(get_reader.get_i8_vec(0, 3).unwrap(), vec![-2, 2, 127]);
            assert_eq!(0, get_reader.position().unwrap());
        }
    }

    #[test]
    fn read_int16() {
        let bytes = vec![0xFE, 0xFF, 0x00, 0x02, 0xFF, 0x7F];

        let mut little_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_i16().unwrap(), -2);
        let mut little_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_vec_reader.read_i16_vec(3).unwrap(), vec![-2, 512, 32767]);
        let mut little_get_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_i16(2).unwrap(), 512);
        assert_eq!(little_get_reader.get_i16_vec(0, 3).unwrap(), vec![-2, 512, 32767]);
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_i16().unwrap(), -257);
        let mut big_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(big_vec_reader.read_i16_vec(3).unwrap(), vec![-257, 2, -129]);
        let mut big_get_reader = BinaryReader::from_bytes(bytes, Endian::Big, true);
        assert_eq!(big_get_reader.get_i16(2).unwrap(), 2);
        assert_eq!(big_get_reader.get_i16_vec(0, 3).unwrap(), vec![-257, 2, -129]);
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_int32() {
        let bytes = vec![0xFE, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x02, 0x00];

        let mut little_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_i32().unwrap(), -2);
        let mut little_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_vec_reader.read_i32_vec(2).unwrap(), vec![-2, 131072]);
        let mut little_get_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_i32(4).unwrap(), 131072);
        assert_eq!(little_get_reader.get_i32_vec(0, 2).unwrap(), vec![-2, 131072]);
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_i32().unwrap(), -16777217);
        let mut big_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(big_vec_reader.read_i32_vec(2).unwrap(), vec![-16777217, 512]);
        let mut big_get_reader = BinaryReader::from_bytes(bytes, Endian::Big, true);
        assert_eq!(big_get_reader.get_i32(4).unwrap(), 512);
        assert_eq!(big_get_reader.get_i32_vec(0, 2).unwrap(), vec![-16777217, 512]);
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_int64() {
        let bytes = vec![
            0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00,
        ];

        let mut little_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_i64().unwrap(), -2);
        let mut little_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_vec_reader.read_i64_vec(2).unwrap(), vec![-2, 562949953421312]);
        let mut little_get_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Little, true);
        assert_eq!(little_get_reader.get_i64(8).unwrap(), 562949953421312);
        assert_eq!(little_get_reader.get_i64_vec(0, 2).unwrap(), vec![-2, 562949953421312]);
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_i64().unwrap(), -72057594037927937);
        let mut big_vec_reader = BinaryReader::from_bytes(bytes.clone(), Endian::Big, true);
        assert_eq!(big_vec_reader.read_i64_vec(2).unwrap(), vec![-72057594037927937, 512]);
        let mut big_get_reader = BinaryReader::from_bytes(bytes, Endian::Big, true);
        assert_eq!(big_get_reader.get_i64(8).unwrap(), 512);
        assert_eq!(big_get_reader.get_i64_vec(0, 2).unwrap(), vec![-72057594037927937, 512]);
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_float32() {
        let little_bytes = vec![
            0x00, 0x00, 0x80, 0x3F,
            0x00, 0x00, 0x20, 0xC0,
        ];
        let big_bytes = vec![
            0x3F, 0x80, 0x00, 0x00,
            0xC0, 0x20, 0x00, 0x00,
        ];

        let mut little_reader = BinaryReader::from_bytes(little_bytes.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_f32().unwrap(), 1.0);
        let mut little_vec_reader = BinaryReader::from_bytes(little_bytes.clone(), Endian::Little, true);
        assert_eq!(little_vec_reader.read_f32_vec(2).unwrap(), vec![1.0, -2.5]);
        let mut little_get_reader = BinaryReader::from_bytes(little_bytes, Endian::Little, true);
        assert_eq!(little_get_reader.get_f32(4).unwrap(), -2.5);
        assert_eq!(little_get_reader.get_f32_vec(0, 2).unwrap(), vec![1.0, -2.5]);
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(big_bytes.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_f32().unwrap(), 1.0);
        let mut big_vec_reader = BinaryReader::from_bytes(big_bytes.clone(), Endian::Big, true);
        assert_eq!(big_vec_reader.read_f32_vec(2).unwrap(), vec![1.0, -2.5]);
        let mut big_get_reader = BinaryReader::from_bytes(big_bytes, Endian::Big, true);
        assert_eq!(big_get_reader.get_f32(4).unwrap(), -2.5);
        assert_eq!(big_get_reader.get_f32_vec(0, 2).unwrap(), vec![1.0, -2.5]);
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_float64() {
        let little_bytes = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xC0,
        ];
        let big_bytes = vec![
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xC0, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut little_reader = BinaryReader::from_bytes(little_bytes.clone(), Endian::Little, true);
        assert_eq!(little_reader.read_f64().unwrap(), 1.0);
        let mut little_vec_reader = BinaryReader::from_bytes(little_bytes.clone(), Endian::Little, true);
        assert_eq!(little_vec_reader.read_f64_vec(2).unwrap(), vec![1.0, -2.5]);
        let mut little_get_reader = BinaryReader::from_bytes(little_bytes, Endian::Little, true);
        assert_eq!(little_get_reader.get_f64(8).unwrap(), -2.5);
        assert_eq!(little_get_reader.get_f64_vec(0, 2).unwrap(), vec![1.0, -2.5]);
        assert_eq!(0, little_get_reader.position().unwrap());

        let mut big_reader = BinaryReader::from_bytes(big_bytes.clone(), Endian::Big, true);
        assert_eq!(big_reader.read_f64().unwrap(), 1.0);
        let mut big_vec_reader = BinaryReader::from_bytes(big_bytes.clone(), Endian::Big, true);
        assert_eq!(big_vec_reader.read_f64_vec(2).unwrap(), vec![1.0, -2.5]);
        let mut big_get_reader = BinaryReader::from_bytes(big_bytes, Endian::Big, true);
        assert_eq!(big_get_reader.get_f64(8).unwrap(), -2.5);
        assert_eq!(big_get_reader.get_f64_vec(0, 2).unwrap(), vec![1.0, -2.5]);
        assert_eq!(0, big_get_reader.position().unwrap());
    }

    #[test]
    fn read_varint_vectors() {
        let short_bytes = vec![0xFE, 0xFF, 0xFF, 0xFF, 0x02, 0x00, 0x00, 0x00];
        let mut short_reader = BinaryReader::from_bytes(short_bytes.clone(), Endian::Little, false);
        assert_eq!(short_reader.read_varint_vec(2).unwrap(), vec![-2, 2]);
        let mut short_get_reader = BinaryReader::from_bytes(short_bytes, Endian::Little, false);
        assert_eq!(short_get_reader.get_varint_vec(0, 2).unwrap(), vec![-2, 2]);
        assert_eq!(0, short_get_reader.position().unwrap());

        let long_bytes = vec![
            0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut long_reader = BinaryReader::from_bytes(long_bytes.clone(), Endian::Little, true);
        assert_eq!(long_reader.read_varint_vec(2).unwrap(), vec![-2, 2]);
        let mut long_get_reader = BinaryReader::from_bytes(long_bytes, Endian::Little, true);
        assert_eq!(long_get_reader.get_varint_vec(0, 2).unwrap(), vec![-2, 2]);
        assert_eq!(0, long_get_reader.position().unwrap());
    }

    #[test]
    fn read_enum_functions() {
        let mut reader = BinaryReader::from_bytes(vec![1], Endian::Little, true);
        assert_eq!(reader.read_enum8::<EnumU8>().unwrap(), EnumU8::One);

        let mut reader = BinaryReader::from_bytes(vec![0x01, 0x00], Endian::Little, true);
        assert_eq!(reader.read_enum_u16::<EnumU16>().unwrap(), EnumU16::One);

        let mut reader = BinaryReader::from_bytes(vec![1, 0, 0, 0], Endian::Little, true);
        assert_eq!(reader.read_enum_u32::<EnumU32>().unwrap(), EnumU32::One);

        let mut reader = BinaryReader::from_bytes(
            vec![1, 0, 0, 0, 0, 0, 0, 0],
            Endian::Little,
            true,
        );
        assert_eq!(reader.read_enum_u64::<EnumU64>().unwrap(), EnumU64::One);

        let mut reader = BinaryReader::from_bytes(vec![1], Endian::Little, true);
        assert_eq!(reader.read_enum_i8::<EnumI8>().unwrap(), EnumI8::One);

        let mut reader = BinaryReader::from_bytes(vec![1, 0], Endian::Little, true);
        assert_eq!(reader.read_enum_i16::<EnumI16>().unwrap(), EnumI16::One);

        let mut reader = BinaryReader::from_bytes(vec![1, 0, 0, 0], Endian::Little, true);
        assert_eq!(reader.read_enum_i32::<EnumI32>().unwrap(), EnumI32::One);

        let mut reader = BinaryReader::from_bytes(
            vec![1, 0, 0, 0, 0, 0, 0, 0],
            Endian::Little,
            true,
        );
        assert_eq!(reader.read_enum_i64::<EnumI64>().unwrap(), EnumI64::One);

        macro_rules! assert_get_enum {
            ($bytes:expr, $method:ident, $type:ty) => {
                let mut reader = BinaryReader::from_bytes($bytes, Endian::Little, true);
                assert_eq!(reader.$method::<$type>(0).unwrap(), <$type>::One);
                assert_eq!(reader.position().unwrap(), 0);
            };
        }

        assert_get_enum!(vec![1], get_enum8, EnumU8);
        assert_get_enum!(vec![1, 0], get_enum_u16, EnumU16);
        assert_get_enum!(vec![1, 0, 0, 0], get_enum_u32, EnumU32);
        assert_get_enum!(vec![1, 0, 0, 0, 0, 0, 0, 0], get_enum_u64, EnumU64);
        assert_get_enum!(vec![1], get_enum_i8, EnumI8);
        assert_get_enum!(vec![1, 0], get_enum_i16, EnumI16);
        assert_get_enum!(vec![1, 0, 0, 0], get_enum_i32, EnumI32);
        assert_get_enum!(vec![1, 0, 0, 0, 0, 0, 0, 0], get_enum_i64, EnumI64);

        let mut invalid_reader = BinaryReader::from_bytes(vec![0x02, 0x00], Endian::Little, true);
        assert!(invalid_reader.read_enum_u16::<EnumU16>().is_err());
    }

    #[test]
    fn read_vectors() {
        let mut vector2_reader = BinaryReader::from_bytes(vec![0x00, 0x00, 0x80, 0x3F, 0xCD, 0xCC, 0x2C, 0x40], Endian::Little, true);
        let vector2 = vector2_reader.read_vector_2().unwrap();
        assert_eq!(vector2.x, 1.0);
        assert_eq!(vector2.y, 2.7);
        
        let mut vector3_reader = BinaryReader::from_bytes(vec![0x00, 0x00, 0x80, 0x3F, 0xCD, 0xCC, 0x2C, 0x40, 0x00, 0x00, 0x00, 0x00], Endian::Little, true);
        let vector3 = vector3_reader.read_vector_3().unwrap();
        assert_eq!(vector3.x, 1.0);
        assert_eq!(vector3.y, 2.7);
        assert_eq!(vector3.z, 0.0);

        let mut vector4_reader = BinaryReader::from_bytes(vec![0x00, 0x00, 0x80, 0x3F, 0xCD, 0xCC, 0x2C, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], Endian::Little, true);
        let vector4 = vector4_reader.read_vector_4().unwrap();
        assert_eq!(vector4.x, 1.0);
        assert_eq!(vector4.y, 2.7);
        assert_eq!(vector4.z, 0.0);
        assert_eq!(vector4.w, 0.0);
    }

    #[test]
    fn read_strings() {
        let mut ascii_reader = BinaryReader::from_bytes(b"hello\0world".to_vec(), Endian::Little, true);
        assert_eq!(ascii_reader.read_ascii().unwrap(), "hello");
        assert_eq!(ascii_reader.read_ascii_len(5).unwrap(), "world");

        let mut ascii_get_reader = BinaryReader::from_bytes(b"xMAGIC\0".to_vec(), Endian::Little, true);
        assert_eq!(ascii_get_reader.get_ascii_len(1, 5).unwrap(), "MAGIC");
        assert_eq!(ascii_get_reader.position().unwrap(), 0);
        assert_eq!(ascii_get_reader.assert_ascii(&["xMAGIC", "OTHER"]).unwrap(), "xMAGIC");

        let (shift_jis_bytes, _, _) = encoding_rs::SHIFT_JIS.encode("テスト");
        let mut shift_jis_data = shift_jis_bytes.into_owned();
        shift_jis_data.push(0);
        let mut shift_jis_reader = BinaryReader::from_bytes(shift_jis_data, Endian::Little, true);
        assert_eq!(shift_jis_reader.read_shift_jis().unwrap(), "テスト");

        let mut utf16_reader = BinaryReader::from_bytes(
            vec![0x48, 0x00, 0x69, 0x00, 0x00, 0x00],
            Endian::Little,
            true,
        );
        assert_eq!(utf16_reader.read_utf16().unwrap(), "Hi");

        let mut utf16_get_reader = BinaryReader::from_bytes(
            vec![0x00, 0x00, 0x00, 0x48, 0x00, 0x69, 0x00, 0x00],
            Endian::Big,
            true,
        );
        assert_eq!(utf16_get_reader.get_utf16(2).unwrap(), "Hi");
        assert_eq!(utf16_get_reader.position().unwrap(), 0);

        let mut fixed_reader = BinaryReader::from_bytes(
            b"name\0padding".to_vec(),
            Endian::Little,
            true,
        );
        assert_eq!(fixed_reader.read_fix_str(8).unwrap(), "name");

        let mut fixed_w_reader = BinaryReader::from_bytes(
            vec![0x6E, 0x00, 0x61, 0x00, 0x00, 0x00, 0xFF, 0xFF],
            Endian::Little,
            true,
        );
        assert_eq!(fixed_w_reader.read_fix_str_w(8).unwrap(), "na");
    }

    #[test]
    fn assert_values() {
        let mut bool_reader = BinaryReader::from_bytes(vec![1], Endian::Little, true);
        assert_eq!(bool_reader.assert_bool(&[false, true]).unwrap(), true);

        let mut integer_reader = BinaryReader::from_bytes(
            vec![1, 0, 0, 0, 0, 0, 0, 0],
            Endian::Little,
            true,
        );
        assert_eq!(integer_reader.assert_u8(&[0, 1]).unwrap(), 1);
        assert_eq!(integer_reader.assert_u16(&[0, 1]).unwrap(), 0);
        assert_eq!(integer_reader.assert_u32(&[0, 1]).unwrap(), 0);

        let mut signed_reader = BinaryReader::from_bytes(vec![0xFF], Endian::Little, true);
        assert_eq!(signed_reader.assert_i8(&[-1, 1]).unwrap(), -1);

        let mut float_reader = BinaryReader::from_bytes(
            vec![0x00, 0x00, 0x80, 0x3F],
            Endian::Little,
            true,
        );
        assert_eq!(float_reader.assert_f32(&[0.0, 1.0]).unwrap(), 1.0);

        let mut varint_reader = BinaryReader::from_bytes(
            vec![0xFF, 0xFF, 0xFF, 0xFF],
            Endian::Little,
            false,
        );
        assert_eq!(varint_reader.assert_varint(&[0, -1]).unwrap(), -1);

        let mut invalid_reader = BinaryReader::from_bytes(vec![2], Endian::Little, true);
        assert!(invalid_reader.assert_bool(&[false, true]).is_err());
    }

    #[test]
    fn stream_operations() {
        let mut reader = BinaryReader::from_bytes(vec![0x0, 0x0, 0x0, 0x0, 0x0], Endian::Little, true);

        assert_eq!(reader.position().unwrap(), 0);
        assert_eq!(reader.length().unwrap(), 5);
        reader.seek(1).unwrap();
        assert_eq!(reader.remaining().unwrap(), 4);
        reader.skip(3).unwrap();
        assert_eq!(reader.position().unwrap(), 4);
        reader.skip(-2).unwrap();
        assert_eq!(reader.position().unwrap(), 2);
    }

    #[test]
    fn pad_leaves_next_data_untouched() {
        let mut data = vec![0u8; 0x10];
        data.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

        let mut reader = BinaryReader::from_bytes(data, Endian::Little, true);

        reader.seek(0x0D).unwrap();
        reader.pad(0x10).unwrap();

        let value = reader.read_u32().unwrap();

        assert_eq!(value, 0xEFBEADDE);
    }

    #[test]
    fn pad_supports_different_alignments() {
        let cases = [
            (0x00, 0x04, 0x00),
            (0x01, 0x04, 0x04),
            (0x03, 0x04, 0x04),
            (0x04, 0x04, 0x04),
            (0x07, 0x08, 0x08),
            (0x11, 0x10, 0x20),
            (0x21, 0x20, 0x40),
        ];

        for (position, alignment, expected) in cases {
            let data = vec![0u8; 128];
            let mut reader = BinaryReader::from_bytes(data.clone(), Endian::Little, true);

            reader.seek(position).unwrap();
            reader.pad(alignment).unwrap();

            assert_eq!(reader.position().unwrap(), expected);
        }
    }

    #[test]
    fn pad_rejects_zero_alignment() {
        let data = vec![0u8; 16];
        let mut reader = BinaryReader::from_bytes(data.clone(), Endian::Little, true);

        assert!(reader.pad(0).is_err());
    }