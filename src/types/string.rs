//! MC Protocol String data type.

use std::error::Error;
use std::io;
use std::io::prelude::*;

use packet::Protocol;
use types::VarInt;
use util::ReadExactExt;

/// UTF-8 string prefixed with its length as a VarInt.
impl Protocol for String {
    type Clean = String;

    fn proto_len(value: &String) -> usize {
        let str_len = value.len();
        <VarInt as Protocol>::proto_len(&(str_len as i32)) + str_len
    }

    fn proto_encode(value: &String, dst: &mut Write) -> io::Result<()> {
        let str_len = value.len() as i32;
        try!(<VarInt as Protocol>::proto_encode(&str_len, dst));
        try!(dst.write_all(value.as_bytes()));
        Ok(())
    }

    fn proto_decode(mut src: &mut Read) -> io::Result<String> {
        let len: i32 = try!(<VarInt as Protocol>::proto_decode(src));
        let s = try!(src.read_exact(len as usize));
        String::from_utf8(s).map_err(|utf8_err| io::Error::new(io::ErrorKind::InvalidInput, "invalid String value", Some(format!("UTF-8 error: {}", utf8_err.utf8_error().description()))))
    }
}
