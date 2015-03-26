//! 3D position types

use std::io;
use std::io::prelude::*;
use std::iter::AdditiveIterator;
use std::num::Int;

use packet::Protocol;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub struct BlockPos;

macro_rules! bounds_check {
    ($name:expr, $value:expr, $size:expr) => {
        if $value < -(1 << $size) || $value >= (1 << $size) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "coordinate out of bounds", Some(format!("expected {} to {}, found {} for {} coord", -(1 << $size), (1 << $size) - 1, $value, $name))));
        }
    }
}

impl Protocol for BlockPos {
    type Clean = [i32; 3];

    #[allow(unused_variables)]
    fn proto_len(value: &[i32; 3]) -> usize { 8 }

    fn proto_encode(value: &[i32; 3], mut dst: &mut Write) -> io::Result<()> {
        let x = value[0].clone();
        let y = value[1].clone();
        let z = value[2].clone();
        bounds_check!("x", x, 25);
        bounds_check!("y", y, 11);
        bounds_check!("z", z, 25);
        try!(dst.write_u64::<BigEndian>((x as u64 & 0x3ffffff) << 38 | (y as u64 & 0xfff) << 26 | z as u64 & 0x3ffffff));
        Ok(())
    }

    fn proto_decode(mut src: &mut Read) -> io::Result<[i32; 3]> {
        let block_pos = try!(src.read_u64::<BigEndian>());
        let x = (block_pos >> 38) as i32;
        let y = (block_pos >> 26 & 0xfff) as i32;
        let z = (block_pos & 0x3ffffff) as i32;
        Ok([
            if x >= 1 << 25 { x - (1 << 26) } else { x },
            if y >= 1 << 11 { y - (1 << 12) } else { y },
            if z >= 1 << 25 { z - (1 << 26) } else { z }
        ])
    }
}

impl<T: Protocol> Protocol for [T; 3] {
    type Clean = [T::Clean; 3];

    fn proto_len(value: &[T::Clean; 3]) -> usize {
        value.iter().map(|coord| <T as Protocol>::proto_len(coord)).sum()
    }

    fn proto_encode(value: &[T::Clean; 3], dst: &mut Write) -> io::Result<()> {
        for coord in value {
            try!(<T as Protocol>::proto_encode(coord, dst));
        }
        Ok(())
    }

    fn proto_decode(src: &mut Read) -> io::Result<[T::Clean; 3]> {
        let x = try!(<T as Protocol>::proto_decode(src));
        let y = try!(<T as Protocol>::proto_decode(src));
        let z = try!(<T as Protocol>::proto_decode(src));
        Ok([x, y, z])
    }
}
