//! MC Protocol UUID data type.

use std::io;
use std::io::prelude::*;

use packet::Protocol;
use util::ReadExactExt;

use uuid::Uuid;

/// UUID read/write wrapper.
impl Protocol for Uuid {
    type Clean = Uuid;

    #[allow(unused_variables)]
    fn proto_len(value: &Uuid) -> usize { 16 }

    /// Writes `value` into `dst`
    fn proto_encode(value: &Uuid, dst: &mut Write) -> io::Result<()> {
        dst.write_all(value.as_bytes())
    }

    /// Reads 16 bytes from `src` and returns a `Uuid`
    #[allow(unused_variables)]
    fn proto_decode(mut src: &mut Read) -> io::Result<Uuid> {
        let v = try!(src.read_exact(16));
        Uuid::from_bytes(&v).ok_or(io::Error::new(io::ErrorKind::InvalidInput, "invalid UUID value", Some(format!("value {:?} can't be used to create UUID", v))))
    }
}

