//! MC Protocol UUID data type.

use std::io;
use std::io::prelude::*;
use std::io::ErrorKind::InvalidInput;
use std::str::FromStr;

use crate::packet::Protocol;

use uuid::{ParseError, Uuid};

/// UUID read/write wrapper.
impl Protocol for Uuid {
    type Clean = Uuid;

    fn proto_len(_: &Uuid) -> usize {
        16
    }
    fn proto_encode(value: &Uuid, dst: &mut dyn Write) -> io::Result<()> {
        dst.write_all(value.as_bytes())
    }
    /// Reads 16 bytes from `src` and returns a `Uuid`
    fn proto_decode(src: &mut dyn Read) -> io::Result<Uuid> {
        let mut v = [0_u8; 16];
        src.read_exact(&mut v)?;
        Uuid::from_bytes(&v).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                &format!("Invalid UUID value: {:?} can't be used to create UUID", v)[..],
            )
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct UuidString;

impl Protocol for UuidString {
    type Clean = Uuid;

    fn proto_len(value: &Uuid) -> usize {
        <String as Protocol>::proto_len(&value.to_hyphenated_string())
    }

    fn proto_encode(value: &Uuid, dst: &mut dyn Write) -> io::Result<()> {
        <String as Protocol>::proto_encode(&value.to_hyphenated_string(), dst)
    }

    fn proto_decode(src: &mut dyn Read) -> io::Result<Uuid> {
        // Unfortunately we can't implement `impl FromError<ParseError> for io::Error`
        let s = <String as Protocol>::proto_decode(src)?;
        Uuid::from_str(&s).map_err(|err| match err {
            ParseError::InvalidLength(length) => {
                io::Error::new(InvalidInput, &format!("Invalid length: {}", length)[..])
            }
            ParseError::InvalidCharacter(_, _) => io::Error::new(InvalidInput, "invalid character"),
            ParseError::InvalidGroups(_) => io::Error::new(InvalidInput, "invalid groups"),
            ParseError::InvalidGroupLength(_, _, _) => {
                io::Error::new(InvalidInput, "invalid group length")
            }
        })
    }
}
