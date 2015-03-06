//! MC Protocol constants.

use std::io::prelude::*;
use std::io;
use std::num::FromPrimitive;

use packet::Protocol;

macro_rules! enum_protocol_impl {
    ($name:ty, $repr:ty, $dec_repr:ident) => {
        impl Protocol for $name {
            type Clean = $name;

            #[allow(unused_variables)]
            fn proto_len(value: &$name) -> usize { <$repr as Protocol>::proto_len(&(*value as $repr)) }

            fn proto_encode(value: &$name, mut dst: &mut Write) -> io::Result<()> {
                let repr = *value as $repr;
                try!(<$repr as Protocol>::proto_encode(&repr, dst));
                Ok(())
            }

            fn proto_decode(mut src: &mut Read) -> io::Result<$name> {
                let value = try!(<$repr as Protocol>::proto_decode(src));
                match FromPrimitive::$dec_repr(value) {
                    Some(x) => Ok(x),
                    None => Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid enum", None))
                }
            }
        }
    }
}

enum_protocol_impl!(Dimension, i8, from_i8);

#[repr(i8)]
#[derive(Copy, Debug, FromPrimitive, PartialEq)]
pub enum Dimension {
    Nether = -1,
    Overworld = 0,
    End = 1
}
