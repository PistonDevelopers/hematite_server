//! 3D position types

use std::io;
use std::io::prelude::*;

use crate::packet::Protocol;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

#[derive(Copy, Clone, Debug)]
pub struct BlockPos;

macro_rules! bounds_check {
    ($name:expr, $value:expr, $size:expr) => {
        if $value < -(1 << $size) || $value >= (1 << $size) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                &format!(
                    "Coordinate out of bounds: expected {} to {}, found {} for {} coord",
                    -(1 << $size),
                    (1 << $size) - 1,
                    $value,
                    $name
                )[..],
            ));
        }
    };
}

impl Protocol for BlockPos {
    type Clean = [i32; 3];

    fn proto_len(_: &[i32; 3]) -> usize {
        8
    }

    fn proto_encode(value: &[i32; 3], dst: &mut dyn Write) -> io::Result<()> {
        let x = value[0];
        let y = value[1];
        let z = value[2];
        bounds_check!("x", x, 25);
        bounds_check!("y", y, 11);
        bounds_check!("z", z, 25);
        dst.write_u64::<BigEndian>(
            (x as u64 & 0x3ff_ffff) << 38 | (y as u64 & 0xfff) << 26 | z as u64 & 0x3ff_ffff,
        )?;
        Ok(())
    }

    fn proto_decode(src: &mut dyn Read) -> io::Result<[i32; 3]> {
        let block_pos = src.read_u64::<BigEndian>()?;
        let x = (block_pos >> 38) as i32;
        let y = (block_pos >> 26 & 0xfff) as i32;
        let z = (block_pos & 0x3ff_ffff) as i32;
        Ok([
            if x >= 1 << 25 { x - (1 << 26) } else { x },
            if y >= 1 << 11 { y - (1 << 12) } else { y },
            if z >= 1 << 25 { z - (1 << 26) } else { z },
        ])
    }
}

impl<T: Protocol> Protocol for [T; 3] {
    type Clean = [T::Clean; 3];

    fn proto_len(value: &[T::Clean; 3]) -> usize {
        value
            .iter()
            .map(|coord| <T as Protocol>::proto_len(coord))
            .sum()
    }

    fn proto_encode(value: &[T::Clean; 3], dst: &mut dyn Write) -> io::Result<()> {
        for coord in value {
            <T as Protocol>::proto_encode(coord, dst)?;
        }
        Ok(())
    }

    fn proto_decode(src: &mut dyn Read) -> io::Result<[T::Clean; 3]> {
        let x = <T as Protocol>::proto_decode(src)?;
        let y = <T as Protocol>::proto_decode(src)?;
        let z = <T as Protocol>::proto_decode(src)?;
        Ok([x, y, z])
    }
}
