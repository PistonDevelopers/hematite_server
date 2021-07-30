//! Protocol Buffer Varints.

use byteorder::{ReadBytesExt, WriteBytesExt};

use std::io;
use std::io::prelude::*;
use std::marker::PhantomData;

use crate::packet::Protocol;

/// Protocol Buffer varint.
#[derive(Debug)]
pub struct Var<T>(PhantomData<T>);

impl Protocol for Var<i32> {
    type Clean = i32;

    /// Size in bytes of `value` as a `Var<i32>`
    fn proto_len(value: &i32) -> usize {
        let value = *value as u32;
        for i in 1..5 {
            if (value & (0xffff_ffff_u32 << (7 * i))) == 0 {
                return i;
            }
        }
        5
    }

    /// Writes `value` as a `VarInt` into `dst`, it can be up to 5 bytes.
    fn proto_encode(value: &i32, dst: &mut dyn Write) -> io::Result<()> {
        let mut temp = *value as u32;
        loop {
            if (temp & !0x7f_u32) == 0 {
                dst.write_u8(temp as u8)?;
                return Ok(());
            }
            dst.write_u8(((temp & 0x7F) | 0x80) as u8)?;
            temp >>= 7;
        }
    }

    /// Reads up to 5 bytes from `src`, until a valid `Var<i32>` is found.
    fn proto_decode(src: &mut dyn Read) -> io::Result<i32> {
        let mut x = 0_i32;

        for shift in &[0_u32, 7, 14, 21, 28] {
            // (0..32).step_by(7)
            let b = i32::from(src.read_u8()?);
            x |= (b & 0x7F) << shift;
            if (b & 0x80) == 0 {
                return Ok(x);
            }
        }

        // The number is too large to represent in a 32-bit value.
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "VarInt too big",
        ))
    }
}

impl Protocol for Var<i64> {
    type Clean = i64;

    /// Size in bytes of `value` as a `Var<i64>`
    fn proto_len(value: &i64) -> usize {
        let value = *value as u64;
        for i in 1..10 {
            if (value & (0xffff_ffff_ffff_ffff_u64 << (7 * i))) == 0 {
                return i;
            }
        }
        10
    }

    /// Writes `value` as a `VarLong` into `dst`, it can be up to 10 bytes.
    fn proto_encode(value: &i64, dst: &mut dyn Write) -> io::Result<()> {
        let mut temp = *value as u64;
        loop {
            if (temp & !0x7f_u64) == 0 {
                dst.write_u8(temp as u8)?;
                return Ok(());
            }
            dst.write_u8(((temp & 0x7F) | 0x80) as u8)?;
            temp >>= 7;
        }
    }

    /// Reads up to 10 bytes from `src`, until a valid `Var<i64>` is found.
    fn proto_decode(src: &mut dyn Read) -> io::Result<i64> {
        let mut x = 0_i64;

        for shift in &[0_u32, 7, 14, 21, 28, 35, 42, 49, 56, 63] {
            let b = i64::from(src.read_u8()?);
            x |= (b & 0x7F) << shift;
            if (b & 0x80) == 0 {
                return Ok(x);
            }
        }

        // The number is too large to represent in a 64-bit value.
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "VarLong too big",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;

    use crate::packet::Protocol;

    // Table driven tests
    struct TestCase<T> {
        value: T,
        bytes: Vec<u8>,
    }

    fn varint_tests() -> Vec<TestCase<i32>> {
        vec![
            TestCase {
                value: -1,
                bytes: vec![0xff, 0xff, 0xff, 0xff, 0xf],
            },
            TestCase {
                value: 0,
                bytes: vec![0x00],
            },
            TestCase {
                value: 1,
                bytes: vec![0x01],
            },
            TestCase {
                value: 127,
                bytes: vec![0x7f],
            },
            TestCase {
                value: 300,
                bytes: vec![0xac, 0x02],
            },
            TestCase {
                value: 14882,
                bytes: vec![0xa2, 0x74],
            },
        ]
    }

    fn varlong_tests() -> Vec<TestCase<i64>> {
        vec![
            TestCase {
                value: -1,
                bytes: vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01],
            },
            TestCase {
                value: 0,
                bytes: vec![0x00],
            },
            TestCase {
                value: 1,
                bytes: vec![0x01],
            },
            TestCase {
                value: 127,
                bytes: vec![0x7f],
            },
            TestCase {
                value: 300,
                bytes: vec![0xac, 0x02],
            },
            TestCase {
                value: 14882,
                bytes: vec![0xa2, 0x74],
            },
            TestCase {
                value: 2_961_488_830_i64,
                bytes: vec![0xbe, 0xf7, 0x92, 0x84, 0x0b],
            },
            TestCase {
                value: 7_256_456_126_i64,
                bytes: vec![0xbe, 0xf7, 0x92, 0x84, 0x1b],
            },
            TestCase {
                value: 41_256_202_580_718_336_i64,
                bytes: vec![0x80, 0xe6, 0xeb, 0x9c, 0xc3, 0xc9, 0xa4, 0x49],
            },
        ]
    }

    #[test]
    fn varint_read() {
        let tests = varint_tests();
        for test in &tests {
            let mut r = io::Cursor::new(test.bytes.clone());
            let value = <Var<i32> as Protocol>::proto_decode(&mut r).unwrap();
            assert_eq!(test.value, value);
        }
    }

    #[test]
    fn varint_write() {
        let tests = varint_tests();
        for test in &tests {
            let mut w = Vec::new();
            <Var<i32> as Protocol>::proto_encode(&test.value, &mut w).unwrap();
            assert_eq!(&w, &test.bytes);
        }
    }

    #[test]
    fn varlong_read() {
        let tests = varlong_tests();
        for test in &tests {
            let mut r = io::Cursor::new(test.bytes.clone());
            let value = <Var<i64> as Protocol>::proto_decode(&mut r).unwrap();
            assert_eq!(test.value, value);
        }
    }

    #[test]
    fn varlong_write() {
        let tests = varlong_tests();
        for test in &tests {
            let mut w = Vec::new();
            <Var<i64> as Protocol>::proto_encode(&test.value, &mut w).unwrap();
            assert_eq!(&w, &test.bytes);
        }
    }
}
