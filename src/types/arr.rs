//! Minecraft's protocol length-prefixed array data type

use std::io;
use std::io::prelude::*;
use std::iter::FromIterator;
use std::marker::PhantomData;

use num::{NumCast, ToPrimitive};

use crate::packet::Protocol;

#[derive(Debug)]
pub struct Arr<L, T>(PhantomData<(fn() -> L, T)>);

impl<L: Protocol, T: Protocol> Protocol for Arr<L, T>
where
    L::Clean: NumCast,
{
    type Clean = Vec<T::Clean>;

    fn proto_len(value: &Vec<T::Clean>) -> usize {
        let len_len = <L as Protocol>::proto_len(
            &(<<L as Protocol>::Clean as NumCast>::from(value.len()).unwrap()),
        );
        let len_values = value
            .iter()
            .map(<T as Protocol>::proto_len)
            .fold(0, |acc, item| acc + item);
        len_len + len_values
    }

    fn proto_encode(value: &Vec<T::Clean>, dst: &mut dyn Write) -> io::Result<()> {
        let len = <L::Clean as NumCast>::from(value.len()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "could not convert length of vector to Array length type",
            )
        })?;
        <L as Protocol>::proto_encode(&len, dst)?;
        for elt in value {
            <T as Protocol>::proto_encode(elt, dst)?;
        }
        Ok(())
    }

    fn proto_decode(src: &mut dyn Read) -> io::Result<Vec<T::Clean>> {
        let len = <L as Protocol>::proto_decode(src)?
            .to_usize()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "could not read length of vector from Array length type",
                )
            })?;
        io::Result::from_iter((0..len).map(|_| <T as Protocol>::proto_decode(src)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;

    use crate::packet::Protocol;
    use crate::types::Var;

    #[test]
    fn arr_encode_i8_varint() {
        let mut dst = Vec::new();
        let value = vec![0_i32, -1_i32];
        <Arr<i8, Var<i32>> as Protocol>::proto_encode(&value, &mut dst).unwrap();
        let bytes = vec![2, 0, 0xff, 0xff, 0xff, 0xff, 0xf];
        assert_eq!(&dst, &bytes);
    }

    #[test]
    fn arr_decode_i8_varint() {
        let bytes = vec![2, 0, 0xff, 0xff, 0xff, 0xff, 0xf];
        let arr = vec![0_i32, -1_i32];
        let mut src = io::Cursor::new(bytes);
        let value = <Arr<i8, Var<i32>> as Protocol>::proto_decode(&mut src).unwrap();
        assert_eq!(arr, value);
    }

    #[test]
    fn arr_encode_i32_i32() {
        let mut dst = Vec::new();
        let value = vec![0_i32, -1_i32];
        <Arr<i32, i32> as Protocol>::proto_encode(&value, &mut dst).unwrap();
        let bytes = vec![
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
        ];
        assert_eq!(&dst, &bytes);
    }

    #[test]
    fn arr_decode_i32_i32() {
        let bytes = vec![
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
        ];
        let arr = vec![0_i32, -1_i32];
        let mut src = io::Cursor::new(bytes);
        let value = <Arr<i32, i32> as Protocol>::proto_decode(&mut src).unwrap();
        assert_eq!(arr, value);
    }
}
