use std::collections::HashMap;
use std::fmt;
use std::io;

use byteorder::{ByteOrder, BigEndian, WriteBytesExt, ReadBytesExt};

use error::NbtError;

/// A value which can be represented in the Named Binary Tag (NBT) file format.
#[derive(Clone, Debug, PartialEq)]
pub enum NbtValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NbtValue>),
    Compound(HashMap<String, NbtValue>),
    IntArray(Vec<i32>),
}

impl NbtValue {
    /// The type ID of this `NbtValue`, which is a single byte in the range
    /// `0x01` to `0x0b`.
    pub fn id(&self) -> u8 {
        match *self {
            NbtValue::Byte(_)      => 0x01,
            NbtValue::Short(_)     => 0x02,
            NbtValue::Int(_)       => 0x03,
            NbtValue::Long(_)      => 0x04,
            NbtValue::Float(_)     => 0x05,
            NbtValue::Double(_)    => 0x06,
            NbtValue::ByteArray(_) => 0x07,
            NbtValue::String(_)    => 0x08,
            NbtValue::List(_)      => 0x09,
            NbtValue::Compound(_)  => 0x0a,
            NbtValue::IntArray(_)  => 0x0b
        }
    }

    /// A string representation of this tag.
    fn tag_name(&self) -> &str {
        match *self {
            NbtValue::Byte(_)      => "TAG_Byte",
            NbtValue::Short(_)     => "TAG_Short",
            NbtValue::Int(_)       => "TAG_Int",
            NbtValue::Long(_)      => "TAG_Long",
            NbtValue::Float(_)     => "TAG_Float",
            NbtValue::Double(_)    => "TAG_Double",
            NbtValue::ByteArray(_) => "TAG_ByteArray",
            NbtValue::String(_)    => "TAG_String",
            NbtValue::List(_)      => "TAG_List",
            NbtValue::Compound(_)  => "TAG_Compound",
            NbtValue::IntArray(_)  => "TAG_IntArray"
        }
    }

    /// The length of the payload of this `NbtValue`, in bytes.
    pub fn len(&self) -> usize {
        match *self {
            NbtValue::Byte(_)            => 1,
            NbtValue::Short(_)           => 2,
            NbtValue::Int(_)             => 4,
            NbtValue::Long(_)            => 8,
            NbtValue::Float(_)           => 4,
            NbtValue::Double(_)          => 8,
            NbtValue::ByteArray(ref val) => 4 + val.len(), // size + bytes
            NbtValue::String(ref val)    => 2 + val.len(), // size + bytes
            NbtValue::List(ref vals)     => {
                // tag + size + payload for each element
                5 + vals.iter().map(|x| x.len()).sum::<usize>()
            },
            NbtValue::Compound(ref vals) => {
                vals.iter().map(|(name, nbt)| {
                    // tag + name + payload for each entry
                    3 + name.len() + nbt.len()
                }).sum::<usize>() + 1 // + u8 for the Tag_End
            },
            NbtValue::IntArray(ref val)  => 4 + 4 * val.len(),
        }
    }

    /// Writes the header (that is, the value's type ID and optionally a title)
    /// of this `NbtValue` to an `io::Write` destination.
    pub fn write_header(&self, mut dst: &mut io::Write, title: &str) -> Result<(), NbtError> {
        try!(dst.write_u8(self.id()));
        try!(dst.write_u16::<BigEndian>(title.len() as u16));
        try!(dst.write_all(title.as_bytes()));
        Ok(())
    }

    /// Writes the payload of this `NbtValue` to an `io::Write` destination.
    pub fn write(&self, mut dst: &mut io::Write) -> Result<(), NbtError> {
        match *self {
            NbtValue::Byte(val)   => try!(dst.write_i8(val)),
            NbtValue::Short(val)  => try!(dst.write_i16::<BigEndian>(val)),
            NbtValue::Int(val)    => try!(dst.write_i32::<BigEndian>(val)),
            NbtValue::Long(val)   => try!(dst.write_i64::<BigEndian>(val)),
            NbtValue::Float(val)  => try!(dst.write_f32::<BigEndian>(val)),
            NbtValue::Double(val) => try!(dst.write_f64::<BigEndian>(val)),
            NbtValue::ByteArray(ref vals) => {
                try!(dst.write_i32::<BigEndian>(vals.len() as i32));
                for &byte in vals {
                    try!(dst.write_i8(byte));
                }
            },
            NbtValue::String(ref val) => {
                try!(dst.write_u16::<BigEndian>(val.len() as u16));
                try!(dst.write_all(val.as_bytes()));
            },
            NbtValue::List(ref vals) => {
                // This is a bit of a trick: if the list is empty, don't bother
                // checking its type.
                if vals.len() == 0 {
                    try!(dst.write_u8(1));
                    try!(dst.write_i32::<BigEndian>(0));
                } else {
                    // Otherwise, use the first element of the list.
                    let first_id = vals[0].id();
                    try!(dst.write_u8(first_id));
                    try!(dst.write_i32::<BigEndian>(vals.len() as i32));
                    for nbt in vals {
                        // Ensure that all of the tags are the same type.
                        if nbt.id() != first_id {
                            return Err(NbtError::HeterogeneousList);
                        }
                        try!(nbt.write(dst));
                    }
                }
            },
            NbtValue::Compound(ref vals)  => {
                for (name, ref nbt) in vals {
                    // Write the header for the tag.
                    try!(nbt.write_header(dst, &name));
                    try!(nbt.write(dst));
                }
                // Write the marker for the end of the Compound.
                try!(dst.write_u8(0x00))
            }
            NbtValue::IntArray(ref vals) => {
                try!(dst.write_i32::<BigEndian>(vals.len() as i32));
                for &nbt in vals {
                    try!(dst.write_i32::<BigEndian>(nbt));
                }
            },
        };
        Ok(())
    }

    /// Reads any valid `NbtValue` header (that is, a type ID and a title of
    /// arbitrary UTF-8 bytes) from an `io::Read` source.
    pub fn read_header(mut src: &mut io::Read) -> Result<(u8, String), NbtError> {
        let id = try!(src.read_u8());
        if id == 0x00 { return Ok((0x00, "".to_string())); }
        // Extract the name.
        let name_len = try!(src.read_u16::<BigEndian>());
        let name = if name_len != 0 {
            try!(read_utf8(src, name_len as usize))
        } else {
            "".to_string()
        };
        Ok((id, name))
    }

    /// Reads the payload of an `NbtValue` with a given type ID from an
    /// `io::Read` source.
    pub fn from_reader(id: u8, mut src: &mut io::Read) -> Result<NbtValue, NbtError> {
        match id {
            0x01 => Ok(NbtValue::Byte(try!(src.read_i8()))),
            0x02 => Ok(NbtValue::Short(try!(src.read_i16::<BigEndian>()))),
            0x03 => Ok(NbtValue::Int(try!(src.read_i32::<BigEndian>()))),
            0x04 => Ok(NbtValue::Long(try!(src.read_i64::<BigEndian>()))),
            0x05 => Ok(NbtValue::Float(try!(src.read_f32::<BigEndian>()))),
            0x06 => Ok(NbtValue::Double(try!(src.read_f64::<BigEndian>()))),
            0x07 => { // ByteArray
                let len = try!(src.read_i32::<BigEndian>()) as usize;
                let mut buf = Vec::with_capacity(len);
                for _ in 0..len {
                    buf.push(try!(src.read_i8()));
                }
                Ok(NbtValue::ByteArray(buf))
            },
            0x08 => { // String
                let len = try!(src.read_u16::<BigEndian>()) as usize;
                Ok(NbtValue::String(try!(read_utf8(src, len))))
            },
            0x09 => { // List
                let id = try!(src.read_u8());
                let len = try!(src.read_i32::<BigEndian>()) as usize;
                let mut buf = Vec::with_capacity(len);
                for _ in 0..len {
                    buf.push(try!(NbtValue::from_reader(id, src)));
                }
                Ok(NbtValue::List(buf))
            },
            0x0a => { // Compound
                let mut buf = HashMap::new();
                loop {
                    let (id, name) = try!(NbtValue::read_header(src));
                    if id == 0x00 { break; }
                    let tag = try!(NbtValue::from_reader(id, src));
                    buf.insert(name, tag);
                }
                Ok(NbtValue::Compound(buf))
            },
            0x0b => { // IntArray
                let len = try!(src.read_i32::<BigEndian>()) as usize;
                let mut buf = Vec::with_capacity(len);
                for _ in 0..len {
                    buf.push(try!(src.read_i32::<BigEndian>()));
                }
                Ok(NbtValue::IntArray(buf))
            },
            e => Err(NbtError::InvalidTypeId(e))
        }
    }
}

impl fmt::Display for NbtValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            NbtValue::Byte(v)   => write!(f, "{}", v),
            NbtValue::Short(v)  => write!(f, "{}", v),
            NbtValue::Int(v)    => write!(f, "{}", v),
            NbtValue::Long(v)   => write!(f, "{}", v),
            NbtValue::Float(v)  => write!(f, "{}", v),
            NbtValue::Double(v) => write!(f, "{}", v),
            NbtValue::ByteArray(ref v) => write!(f, "{:?}", v),
            NbtValue::String(ref v) => write!(f, "{}", v),
            NbtValue::List(ref v) => {
                if v.len() == 0 {
                    write!(f, "zero entries")
                } else {
                    try!(write!(f, "{} entries of type {}\n{{\n", v.len(), v[0].tag_name()));
                    for tag in v {
                        try!(write!(f, "{}(None): {}\n", tag.tag_name(), tag));
                    }
                    try!(write!(f, "}}"));
                    Ok(())
                }
            }
            NbtValue::Compound(ref v) => {
                try!(write!(f, "{} entry(ies)\n{{\n", v.len()));
                for (name, tag) in v {
                    try!(write!(f, "{}(\"{}\"): {}\n", tag.tag_name(), name, tag));
                }
                try!(write!(f, "}}"));
                Ok(())
            }
            NbtValue::IntArray(ref v) => write!(f, "{:?}", v)
        }
    }
}

impl From<i8> for NbtValue {
    fn from(t: i8) -> NbtValue { NbtValue::Byte(t) }
}

impl From<i16> for NbtValue {
    fn from(t: i16) -> NbtValue { NbtValue::Short(t) }
}

impl From<i32> for NbtValue {
    fn from(t: i32) -> NbtValue { NbtValue::Int(t) }
}

impl From<i64> for NbtValue {
    fn from(t: i64) -> NbtValue { NbtValue::Long(t) }
}

impl From<f32> for NbtValue {
    fn from(t: f32) -> NbtValue { NbtValue::Float(t) }
}

impl From<f64> for NbtValue {
    fn from(t: f64) -> NbtValue { NbtValue::Double(t) }
}

impl<'a> From<&'a str> for NbtValue {
    fn from(t: &'a str) -> NbtValue { NbtValue::String(t.into()) }
}

impl From<String> for NbtValue {
    fn from(t: String) -> NbtValue { NbtValue::String(t) }
}

impl From<Vec<i8>> for NbtValue {
    fn from(t: Vec<i8>) -> NbtValue { NbtValue::ByteArray(t) }
}

impl<'a> From<&'a [i8]> for NbtValue {
    fn from(t: &'a [i8]) -> NbtValue { NbtValue::ByteArray(t.into()) }
}

impl From<Vec<i32>> for NbtValue {
    fn from(t: Vec<i32>) -> NbtValue { NbtValue::IntArray(t) }
}

impl<'a> From<&'a [i32]> for NbtValue {
    fn from(t: &'a [i32]) -> NbtValue { NbtValue::IntArray(t.into()) }
}

/// Returns a `Vec<u8>` containing the next `len` bytes in the reader.
///
/// Adapted from `byteorder::read_full`.
fn read_utf8(mut src: &mut io::Read, len: usize) -> Result<String, NbtError> {
    let mut bytes = vec![0; len];
    let mut n_read = 0usize;
    while n_read < bytes.len() {
        match try!(src.read(&mut bytes[n_read..])) {
            0 => return Err(NbtError::IncompleteNbtValue),
            n => n_read += n
        }
    }
    Ok(try!(String::from_utf8(bytes)))
}
