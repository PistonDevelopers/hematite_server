use std::collections::HashMap;
use std::fmt;
use std::io;
use std::ops::Index;

use flate2::Compression;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};

use error::NbtError;
use value::NbtValue;

/// An object in the Named Binary Tag (NBT) file format.
///
/// This is essentially a map of names to `NbtValue`s, with an optional top-
/// level name of its own. It can be created in a similar way to a `HashMap`,
/// or read from an `io::Read` source, and its binary representation can be
/// written to an `io::Write` destination.
///
/// These read and write methods support both uncompressed and compressed
/// (through Gzip or zlib compression) methods.
///
/// ```rust
/// use nbt::{NbtBlob, NbtValue};
///
/// // Create a `NbtBlob` from key/value pairs.
/// let mut nbt = NbtBlob::new("".to_string());
/// nbt.insert("name".to_string(), "Herobrine").unwrap();
/// nbt.insert("health".to_string(), 100i8).unwrap();
/// nbt.insert("food".to_string(), 20.0f32).unwrap();
///
/// // Write a compressed binary representation to a byte array.
/// let mut dst = Vec::new();
/// nbt.write_zlib(&mut dst).unwrap();
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct NbtBlob {
    title: String,
    content: NbtValue
}

impl NbtBlob {
    /// Create a new NBT file format representation with the given name.
    pub fn new(title: String) -> NbtBlob {
        let map: HashMap<String, NbtValue> = HashMap::new();
        NbtBlob { title: title, content: NbtValue::Compound(map) }
    }

    /// Extracts an `NbtBlob` object from an `io::Read` source.
    pub fn from_reader(mut src: &mut io::Read) -> Result<NbtBlob, NbtError> {
        let header = try!(NbtValue::read_header(src));
        // Although it would be possible to read NBT format files composed of
        // arbitrary objects using the current API, by convention all files
        // have a top-level Compound.
        if header.0 != 0x0a {
            return Err(NbtError::NoRootCompound);
        }
        let content = try!(NbtValue::from_reader(header.0, src));
        Ok(NbtBlob { title: header.1, content: content })
    }

    /// Extracts an `NbtBlob` object from an `io::Read` source that is
    /// compressed using the Gzip format.
    pub fn from_gzip(src: &mut io::Read) -> Result<NbtBlob, NbtError> {
        // Reads the gzip header, and fails if it is incorrect.
        let mut data = try!(GzDecoder::new(src));
        NbtBlob::from_reader(&mut data)
    }

    /// Extracts an `NbtBlob` object from an `io::Read` source that is
    /// compressed using the zlib format.
    pub fn from_zlib(src: &mut io::Read) -> Result<NbtBlob, NbtError> {
        NbtBlob::from_reader(&mut ZlibDecoder::new(src))
    }

    /// Writes the binary representation of this `NbtBlob` to an `io::Write`
    /// destination.
    pub fn write(&self, dst: &mut io::Write) -> Result<(), NbtError> {
        try!(self.content.write_header(dst, &self.title));
        self.content.write(dst)
    }

    /// Writes the binary representation of this `NbtBlob`, compressed using
    /// the Gzip format, to an `io::Write` destination.
    pub fn write_gzip(&self, dst: &mut io::Write) -> Result<(), NbtError> {
        self.write(&mut GzEncoder::new(dst, Compression::Default))
    }

    /// Writes the binary representation of this `NbtBlob`, compressed using
    /// the Zlib format, to an `io::Write` dst.
    pub fn write_zlib(&self, dst: &mut io::Write) -> Result<(), NbtError> {
        self.write(&mut ZlibEncoder::new(dst, Compression::Default))
    }

    /// Insert an `NbtValue` with a given name into this `NbtBlob` object. This
    /// method is just a thin wrapper around the underlying `HashMap` method of
    /// the same name.
    ///
    /// This method will also return an error if a `NbtValue::List` with
    /// heterogeneous elements is passed in, because this is illegal in the NBT
    /// file format.
    pub fn insert<V>(&mut self, name: String, value: V) -> Result<(), NbtError>
           where V: Into<NbtValue> {
        // The follow prevents `List`s with heterogeneous tags from being
        // inserted into the file. It would be nicer to return an error, but
        // this would depart from the `HashMap` API for `insert`.
        let nvalue = value.into();
        if let NbtValue::List(ref vals) = nvalue {
            if vals.len() != 0 {
                let first_id = vals[0].id();
                for nbt in vals {
                    if nbt.id() != first_id {
                        return Err(NbtError::HeterogeneousList)
                    }
                }
            }
        }
        if let NbtValue::Compound(ref mut v) = self.content {
            v.insert(name, nvalue);
        } else {
            unreachable!();
        }
        Ok(())
    }

    /// The uncompressed length of this `NbtBlob`, in bytes.
    pub fn len(&self) -> usize {
        // tag + name + content
        1 + 2 + self.title.len() + self.content.len()
    }
}

impl<'a> Index<&'a str> for NbtBlob {
    type Output = NbtValue;

    fn index<'b>(&'b self, s: &'a str) -> &'b NbtValue {
        match self.content {
            NbtValue::Compound(ref v) => v.get(s).unwrap(),
            _ => unreachable!()
        }
    }
}

impl fmt::Display for NbtBlob {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "TAG_Compound(\"{}\"): {}", self.title, self.content)
    }
}
