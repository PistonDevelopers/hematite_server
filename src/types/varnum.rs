//! Protocol Buffer Varints.

use std::old_io::IoResult;
use std::iter::range_step;

use packet::Protocol;

/// Protocol Buffer varint, encoding a two's complement signed 32-bit integer.
#[derive(Copy, Debug)]
pub struct VarInt;

impl Protocol for VarInt {
    type Clean = i32;
    /// Size in bytes of `value` as a VarInt
    fn proto_len(value: &i32) -> usize {
        let value = *value as u32;
        if (value & (0xffffffffu32 <<  7)) == 0 { return 1; }
        if (value & (0xffffffffu32 << 14)) == 0 { return 2; }
        if (value & (0xffffffffu32 << 21)) == 0 { return 3; }
        if (value & (0xffffffffu32 << 28)) == 0 { return 4; }
        5
    }
    /// Writes `value` as a VarInt into `dst`, it can be up to 5 bytes.
    fn proto_encode(value: i32, dst: &mut Writer) -> IoResult<()> {
        let mut temp = value as u32;
        loop {
            if (temp & !0x7fu32) == 0 {
                try!(dst.write_u8(temp as u8));
                return Ok(());
            } else {
                try!(dst.write_u8(((temp & 0x7F) | 0x80) as u8));
                temp >>= 7;
            }
        }
    }
    /// Reads up to 5 bytes from `src`, until a valid VarInt is found.
    #[allow(unused_variables)]
    fn proto_decode(src: &mut Reader, len: usize) -> IoResult<i32> {
        let mut x = 0i32;

        for shift in range_step(0, 32, 7) {
            let b = try!(src.read_u8()) as i32;
            x |= (b & 0x7F) << shift;
            if (b & 0x80) == 0 {
                return Ok(x)
            }
        }

        // The number is too large to represent in a 32-bit value.
        panic!("VarInt too big")
    }
}

/// Protocol Buffer varint, encoding a two's complement signed 64-bit integer.
#[derive(Copy, Debug)]
pub struct VarLong;

impl Protocol for VarLong {
    type Clean = i64;
    /// Size in bytes of `value` as a VarLong
    fn proto_len(value: &i64) -> usize {
        let value = *value as u64;
        if (value & (0xffffffffffffffffu64 <<  7)) == 0 { return 1; }
        if (value & (0xffffffffffffffffu64 << 14)) == 0 { return 2; }
        if (value & (0xffffffffffffffffu64 << 21)) == 0 { return 3; }
        if (value & (0xffffffffffffffffu64 << 28)) == 0 { return 4; }
        if (value & (0xffffffffffffffffu64 << 35)) == 0 { return 5; }
        if (value & (0xffffffffffffffffu64 << 42)) == 0 { return 6; }
        if (value & (0xffffffffffffffffu64 << 49)) == 0 { return 7; }
        if (value & (0xffffffffffffffffu64 << 56)) == 0 { return 8; }
        if (value & (0xffffffffffffffffu64 << 63)) == 0 { return 9; }
        10
    }
    /// Writes `value` as a VarLong into `dst`, it can be up to 10 bytes.
    fn proto_encode(value: i64, dst: &mut Writer) -> IoResult<()> {
        let mut temp = value as u64;
        loop {
            if (temp & !0x7fu64) == 0 {
                try!(dst.write_u8(temp as u8));
                return Ok(());
            } else {
                try!(dst.write_u8(((temp & 0x7F) | 0x80) as u8));
                temp >>= 7;
            }
        }
    }
    /// Reads up to 10 bytes from `dst`, until a valid VarLong is found.
    #[allow(unused_variables)]
    fn proto_decode(dst: &mut Reader, len: usize) -> IoResult<i64> {
        let mut x = 0i64;

        for shift in range_step(0, 64, 7) {
            let b = try!(dst.read_u8()) as i64;
            x |= (b & 0x7F) << shift;
            if (b & 0x80) == 0 {
                return Ok(x)
            }
        }

        // The number is too large to represent in a 64-bit value.
        panic!("VarLong too big")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::old_io::MemReader;

    use packet::Protocol;

    // Table driven tests
    struct TestCase<T> {
        value: T,
        bytes: Vec<u8>
    }

    fn varint_tests() -> Vec<TestCase<i32>> {
        vec![
            TestCase{value: -1,    bytes: vec![0xff, 0xff, 0xff, 0xff, 0xf]},
            TestCase{value: 0,     bytes: vec![0x00]},
            TestCase{value: 1,     bytes: vec![0x01]},
            TestCase{value: 127,   bytes: vec![0x7f]},
            TestCase{value: 300,   bytes: vec![0xac, 0x02]},
            TestCase{value: 14882, bytes: vec![0xa2, 0x74]},
        ]
    }

    fn varlong_tests() -> Vec<TestCase<i64>> {
        vec![
            TestCase{
                value: -1,
                bytes: vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01]
            },
            TestCase{value: 0,     bytes: vec![0x00]},
            TestCase{value: 1,     bytes: vec![0x01]},
            TestCase{value: 127,   bytes: vec![0x7f]},
            TestCase{value: 300,   bytes: vec![0xac, 0x02]},
            TestCase{value: 14882, bytes: vec![0xa2, 0x74]},
            TestCase{
                value: 2961488830i64,
                bytes: vec![0xbe, 0xf7, 0x92, 0x84, 0x0b]
            },
            TestCase{
                value: 7256456126i64,
                bytes: vec![0xbe, 0xf7, 0x92, 0x84, 0x1b]
            },
            TestCase{
                value: 41256202580718336i64,
                bytes: vec![0x80, 0xe6, 0xeb, 0x9c, 0xc3, 0xc9, 0xa4, 0x49]
            },
        ]
    }

    #[test]
    fn varint_read() {
        let tests = varint_tests();
        for test in tests.iter() {
            let mut r = MemReader::new(test.bytes.clone());
            let value = <VarInt as Protocol>::proto_decode(&mut r, 0).unwrap();
            assert_eq!(test.value, value);
        }
    }

    #[test]
    fn varint_write() {
        let tests = varint_tests();
        for test in tests.iter() {
            let mut w = Vec::new();
            <VarInt as Protocol>::proto_encode(test.value, &mut w).unwrap();
            assert_eq!(&w, &test.bytes);
        }
    }

    #[test]
    fn varlong_read() {
        let tests = varlong_tests();
        for test in tests.iter() {
            let mut r = MemReader::new(test.bytes.clone());
            let value = <VarLong as Protocol>::proto_decode(&mut r, 0).unwrap();
            assert_eq!(test.value, value);
        }
    }

    #[test]
    fn varlong_write() {
        let tests = varlong_tests();
        for test in tests.iter() {
            let mut w = Vec::new();
            <VarLong as Protocol>::proto_encode(test.value, &mut w).unwrap();
            assert_eq!(&w, &test.bytes);
        }
    }
}
