//! MC Protocol UUID data type.

use std::old_io::{ IoError, IoErrorKind, IoResult, MemReader };

use packet::Protocol;
use uuid::Uuid;

/// UUID read/write wrapper, two signed 64-bit integers.
impl Protocol for Uuid {
    type Clean = Uuid;
    #[allow(unused_variables)]
    fn proto_len(value: &Uuid) -> usize { 16 }
    /// Writes `value` as two i64 into `dst`
    fn proto_encode(value: Uuid, dst: &mut Writer) -> IoResult<()> {
        let mut mr = MemReader::new(value.as_bytes().to_vec());
        let a = try!(mr.read_be_i64());
        let b = try!(mr.read_be_i64());
        try!(dst.write_be_i64(a));
        try!(dst.write_be_i64(b));
        Ok(())
    }
    /// Reads two i64 (16 bytes) from `src` and returns an `Uuid`
    #[allow(unused_variables)]
    fn proto_decode(src: &mut Reader, plen: usize) -> IoResult<Uuid> {
        let a = try!(src.read_be_i64());
        let b = try!(src.read_be_i64());
        let mut v = Vec::new();
        try!(v.write_be_i64(a));
        try!(v.write_be_i64(b));
        match Uuid::from_bytes(v.as_slice()) {
            Some(u) => Ok(u),
            None => Err(IoError {
                kind: IoErrorKind::InvalidInput,
                desc: "invalid UUID value",
                detail: Some(format!("value {:?} can't be used to create UUID", v))
            }),
        }
    }
}

