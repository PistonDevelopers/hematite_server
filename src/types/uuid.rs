//! MC Protocol UUID data type.

use std::old_io::{ IoError, IoErrorKind, IoResult };

use packet::Protocol;
use uuid::Uuid;

/// UUID read/write wrapper.
impl Protocol for Uuid {
    type Clean = Uuid;

    #[allow(unused_variables)]
    fn proto_len(value: &Uuid) -> usize { 16 }

    /// Writes `value` into `dst`
    fn proto_encode(value: Uuid, dst: &mut Writer) -> IoResult<()> {
        dst.write_all(value.as_bytes())
    }

    /// Reads 16 bytes from `src` and returns a `Uuid`
    #[allow(unused_variables)]
    fn proto_decode(src: &mut Reader) -> IoResult<Uuid> {
        let v = try!(src.read_exact(16));
        Uuid::from_bytes(&v).ok_or(IoError {
            kind: IoErrorKind::InvalidInput,
            desc: "invalid UUID value",
            detail: Some(format!("value {:?} can't be used to create UUID", v))
        })
    }
}

