//! MC Protocol String data type.

use std::io;
use std::io::prelude::*;

use crate::packet::Protocol;
use crate::types::Var;

/// UTF-8 string prefixed with its length as a `VarInt`.
impl Protocol for String {
    type Clean = String;

    fn proto_len(value: &String) -> usize {
        let str_len = value.len();
        <Var<i32> as Protocol>::proto_len(&(str_len as i32)) + str_len
    }

    fn proto_encode(value: &String, dst: &mut dyn Write) -> io::Result<()> {
        let str_len = value.len() as i32;
        <Var<i32> as Protocol>::proto_encode(&str_len, dst)?;
        dst.write_all(value.as_bytes())?;
        Ok(())
    }

    fn proto_decode(src: &mut dyn Read) -> io::Result<String> {
        let len: i32 = <Var<i32> as Protocol>::proto_decode(src)?;
        let mut s = vec![0_u8; len as usize];
        src.read_exact(&mut s)?;
        String::from_utf8(s).map_err(|utf8_err| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                &format!("UTF-8 error: {}", utf8_err.utf8_error().to_string())[..],
            )
        })
    }
}
