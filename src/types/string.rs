//! MC Protocol String data type.

use std::error::Error;
use std::old_io::{ IoError, IoErrorKind, IoResult };

use packet::Protocol;
use types::VarInt;

/// UTF-8 string prefixed with its length as a VarInt.
impl Protocol for String {
    type Clean = String;
    fn proto_len(value: &String) -> usize {
        let str_len = value.len();
        <VarInt as Protocol>::proto_len(&(str_len as i32)) + str_len
    }
    fn proto_encode(value: String, dst: &mut Writer) -> IoResult<()> {
        let str_len = value.len() as i32;
        try!(<VarInt as Protocol>::proto_encode(str_len, dst));
        try!(dst.write_str(value.as_slice()));
        Ok(())
    }
    fn proto_decode(src: &mut Reader, plen: usize) -> IoResult<String> {
        let len: i32 = try!(<VarInt as Protocol>::proto_decode(src, plen));
        let s = try!(src.read_exact(len as usize));
        let utf8s = try!(String::from_utf8(s).map_err(|utf8_err| IoError {
            kind: IoErrorKind::InvalidInput,
            desc: "invalid String value",
            detail: Some(format!("UTF-8 error: {}", utf8_err.utf8_error().description()))
        }));
        Ok(utf8s)
    }
}
