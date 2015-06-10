use std::io;
use std::io::ErrorKind::InvalidInput;
use std::string;

use byteorder;

/// Errors that may be encountered when constructing, parsing, or encoding
/// `NbtValue` and `NbtBlob` objects.
///
/// `NbtError`s can be seamlessly converted to more general `io::Error` objects
/// using the `FromError` trait.
#[derive(Debug)]
pub enum NbtError {
    /// Wraps errors emitted by methods during I/O operations.
    IoError(io::Error),
    /// An error for when an unknown type ID is encountered in decoding NBT
    /// binary representations. Includes the ID in question.
    InvalidTypeId(u8),
    /// An error emitted when trying to create `NbtBlob`s with incorrect lists.
    HeterogeneousList,
    /// An error for when NBT binary representations do not begin with an
    /// `NbtValue::Compound`.
    NoRootCompound,
    /// An error for when NBT binary representations contain invalid UTF-8
    /// strings.
    InvalidUtf8,
    /// An error for when NBT binary representations are missing end tags,
    /// contain fewer bytes than advertised, or are otherwise incomplete.
    IncompleteNbtValue,
}

// Implement PartialEq manually, since std::io::Error is not PartialEq.
impl PartialEq<NbtError> for NbtError {
    fn eq(&self, other: &NbtError) -> bool {
        use NbtError::{IoError, InvalidTypeId, HeterogeneousList, NoRootCompound,
                       InvalidUtf8, IncompleteNbtValue};

        match (self, other) {
            (&IoError(_), &IoError(_))                 => true,
            (&InvalidTypeId(a), &InvalidTypeId(b))     => a == b,
            (&HeterogeneousList, &HeterogeneousList)   => true,
            (&NoRootCompound, &NoRootCompound)         => true,
            (&InvalidUtf8, &InvalidUtf8)               => true,
            (&IncompleteNbtValue, &IncompleteNbtValue) => true,
            _ => false
        }
    }
}

impl From<io::Error> for NbtError {
    fn from(e: io::Error) -> NbtError {
        NbtError::IoError(e)
    }
}

impl From<string::FromUtf8Error> for NbtError {
    fn from(_: string::FromUtf8Error) -> NbtError {
        NbtError::InvalidUtf8
    }
}

impl From<byteorder::Error> for NbtError {
    fn from(err: byteorder::Error) -> NbtError {
        match err {
            // Promote byteorder's I/O errors to NbtError's I/O errors.
            byteorder::Error::Io(e) => NbtError::IoError(e),
            // Anything else is really an incomplete value.
            byteorder::Error::UnexpectedEOF => NbtError::IncompleteNbtValue
        }
    }
}

impl From<NbtError> for io::Error {
    fn from(e: NbtError) -> io::Error {
        match e {
            NbtError::IoError(e) => e,
            NbtError::InvalidTypeId(id) =>
                io::Error::new(InvalidInput, &format!("invalid NBT value type: {}", id)[..]),
            NbtError::HeterogeneousList =>
                io::Error::new(InvalidInput, "List values must be homogeneous"),
            NbtError::NoRootCompound =>
                io::Error::new(InvalidInput, "root value must be a Compound (0x0a)"),
            NbtError::InvalidUtf8 =>
                io::Error::new(InvalidInput, "string is not UTF-8"),
            NbtError::IncompleteNbtValue =>
                io::Error::new(InvalidInput, "data does not represent a complete NbtValue"),
        }
    }
}
