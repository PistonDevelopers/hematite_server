//! MC Named Binary Tag type.

use byteorder::{ByteOrder, BigEndian, WriteBytesExt, ReadBytesExt};
use byteorder::Error::{UnexpectedEOF, Io};

use std::collections::HashMap;
use std::io;
use std::io::ErrorKind::InvalidInput;
use std::iter::AdditiveIterator;
use std::ops::Index;

use packet::Protocol;
use util::ReadExactExt;

use flate::{ inflate_bytes, inflate_bytes_zlib };

/// Represents a NBT value
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
                5 + vals.iter().map(|x| x.len()).sum()
            },
            NbtValue::Compound(ref vals) => {
                vals.iter().map(|(name, nbt)| {
                    // tag + name + payload for each entry
                    3 + name.len() + nbt.len()
                }).sum() + 1 // + u8 for the Tag_End
            },
            NbtValue::IntArray(ref val)  => 4 + 4 * val.len(),
        }
    }

    pub fn write_header(&self, mut sink: &mut io::Write, title: &String) -> io::Result<()> {
        try!(sink.write_u8(self.id()));
        try!(sink.write_u16::<BigEndian>(title.len() as u16));
        sink.write_all(title.as_slice().as_bytes())
    }

    /// Writes the payload of this NbtValue value to the specified `Write` sink. To
    /// include the name, use `write_with_name()`.
    pub fn write(&self, mut sink: &mut io::Write) -> io::Result<()> {
        let res = match *self {
            NbtValue::Byte(val)   => sink.write_i8(val),
            NbtValue::Short(val)  => sink.write_i16::<BigEndian>(val),
            NbtValue::Int(val)    => sink.write_i32::<BigEndian>(val),
            NbtValue::Long(val)   => sink.write_i64::<BigEndian>(val),
            NbtValue::Float(val)  => sink.write_f32::<BigEndian>(val),
            NbtValue::Double(val) => sink.write_f64::<BigEndian>(val),
            NbtValue::ByteArray(ref vals) => {
                try!(sink.write_i32::<BigEndian>(vals.len() as i32));
                for &byte in vals {
                    try!(sink.write_i8(byte));
                }
                return Ok(());
            },
            NbtValue::String(ref val) => {
                try!(sink.write_u16::<BigEndian>(val.len() as u16));
                return sink.write_all(val.as_slice().as_bytes());
            },
            NbtValue::List(ref vals) => {
                // This is a bit of a trick: if the list is empty, don't bother
                // checking its type.
                if vals.len() == 0 {
                    try!(sink.write_u8(1));
                } else {
                    // Otherwise, use the first element of the list.
                    try!(sink.write_u8(vals[0].id()));
                }
                try!(sink.write_i32::<BigEndian>(vals.len() as i32));
                for nbt in vals {
                    try!(nbt.write(sink));
                }
                return Ok(());
            },
            NbtValue::Compound(ref vals)  => {
                for (name, ref nbt) in vals {
                    // Write the header for the tag.
                    try!(nbt.write_header(sink, &name));
                    try!(nbt.write(sink));
                }
                // Write the marker for the end of the Compound.
                sink.write_u8(0x00)
            }
            NbtValue::IntArray(ref vals) => {
                try!(sink.write_i32::<BigEndian>(vals.len() as i32));
                for &nbt in vals {
                    try!(sink.write_i32::<BigEndian>(nbt));
                }
                return Ok(());
            },
        };
        // Since byteorder has slightly different errors than io, we need to
        // awkwardly wrap the results.
        match res {
            Err(UnexpectedEOF) => Err(io::Error::new(InvalidInput, "invalid byte ordering", None)),
            Err(Io(e)) => Err(e),
            Ok(_) => Ok(())
        }
    }

    pub fn read_header(mut src: &mut io::Read) -> io::Result<(u8, String)> {
        let id = try!(src.read_u8());
        if id == 0x00 { return Ok((0x00, "".to_string())); }
        // Extract the name.
        let name_len = try!(src.read_u16::<BigEndian>());
        let name = if name_len != 0 {
            let bytes = try!(src.read_exact(name_len as usize));
            match String::from_utf8(bytes) {
                Ok(v) => v,
                Err(e) => return Err(io::Error::new(InvalidInput, "string is not UTF-8", Some(format!("{}", e))))
            }
        } else {
            "".to_string()
        };
        Ok((id, name))
    }

    pub fn from_reader(id: u8, mut src: &mut io::Read) -> io::Result<NbtValue> {
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
                for _ in range(0, len) {
                    buf.push(try!(src.read_i8()));
                }
                Ok(NbtValue::ByteArray(buf))
            },
            0x08 => { // String
                let len = try!(src.read_u16::<BigEndian>()) as usize;
                let bytes = try!(src.read_exact(len as usize));
                match String::from_utf8(bytes) {
                    Ok(v)  => Ok(NbtValue::String(v)),
                    Err(e) => return Err(io::Error::new(InvalidInput, "string is not UTF-8", Some(format!("{}", e))))
                }
            },
            0x09 => { // List
                let id = try!(src.read_u8());
                let len = try!(src.read_i32::<BigEndian>()) as usize;
                let mut buf = Vec::with_capacity(len);
                for _ in range(0, len) {
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
                for _ in range(0, len) {
                    buf.push(try!(src.read_i32::<BigEndian>()));
                }
                Ok(NbtValue::IntArray(buf))
            },
            _ => Err(io::Error::new(InvalidInput, "invalid NbtValue id", None))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NbtFile {
    title: String,
    content: NbtValue
}

impl NbtFile {
    pub fn new(title: String) -> NbtFile {
        let map: HashMap<String, NbtValue> = HashMap::new();
        NbtFile { title: title, content: NbtValue::Compound(map) }
    }

    /// Extracts an `NbtFile` from an arbitrary interface to `io::Read`.
    pub fn from_reader(src: &mut io::Read) -> io::Result<NbtFile> {
        let header = try!(NbtValue::read_header(&mut *src));
        // Although it would be possible to read NBT files composed of
        // arbitrary objects using the current API, by convention all files
        // have a top-level Compound.
        if header.0 != 0x0a {
            return Err(io::Error::new(InvalidInput, "invalid NBT file",
                       Some(format!("root value must be a Compound (0x0a)"))));
        }
        let content = try!(NbtValue::from_reader(header.0, &mut *src));
        Ok(NbtFile { title: header.1, content: content })
    }

    /// Extracts an `NbtFile` value from compressed gzip data.
    pub fn from_gzip(data: &[u8]) -> io::Result<NbtFile> {
        // Check for the correct gzip header.
        if &data[..4] != [0x1f, 0x8b, 0x08, 0x00].as_slice() {
            return Err(io::Error::new(InvalidInput, "invalid gzip file", None));
        };
        // flake::inflate_bytes returns None in case of failure; turn this into
        // an io::Error instead.
        let data = match inflate_bytes(&data[10..]) {
            Some(v) => v,
            None    => return Err(io::Error::new(InvalidInput, "invalid gzip file", None))
        };
        NbtFile::from_reader(&mut io::Cursor::new(data.as_slice()))
    }

    /// Extracts an `NbtFile` value from compressed zlib data.
    pub fn from_zlib(data: &[u8]) -> io::Result<NbtFile> {
        // flake::inflate_bytes_zlib returns None in case of failure; turn this
        // into an io::Error instead.
        let data = match inflate_bytes_zlib(data) {
            Some(v) => v,
            None    => return Err(io::Error::new(InvalidInput, "invalid zlib file", None))
        };
        NbtFile::from_reader(&mut io::Cursor::new(data.as_slice()))
    }

    pub fn write(&self, sink: &mut io::Write) -> io::Result<()> {
        try!(self.content.write_header(sink, &self.title));
        self.content.write(sink)
    }

    pub fn insert(&mut self, name: String, value: NbtValue) -> Option<NbtValue> {
        match self.content {
            NbtValue::Compound(ref mut v) => v.insert(name, value),
            _ => unreachable!()
        }
    }

    /// The length of this `NbtFile`, in bytes.
    pub fn len(&self) -> usize {
        // tag + name + content
        1 + 2 + self.title.as_slice().len() + self.content.len()
    }
}

impl<'a> Index<&'a str> for NbtFile {
    type Output = NbtValue;

    fn index<'b>(&'b self, s: &&'a str) -> &'b NbtValue {
        match self.content {
            NbtValue::Compound(ref v) => v.get(*s).unwrap(),
            _ => unreachable!()
        }
    }
}

impl Protocol for NbtFile {
    type Clean = NbtFile;

    fn proto_len(value: &NbtFile) -> usize {
        value.len()
    }

    fn proto_encode(value: &NbtFile, mut dst: &mut io::Write) -> io::Result<()> {
        value.write(dst)
    }

    fn proto_decode(mut src: &mut io::Read) -> io::Result<NbtFile> {
        NbtFile::from_reader(src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::io;

    use packet::Protocol;

    #[test]
    fn nbt_nonempty() {
        let mut nbt = NbtFile::new("".to_string());
        nbt.insert("name".to_string(), NbtValue::String("Herobrine".to_string()));
        nbt.insert("health".to_string(), NbtValue::Byte(100));
        nbt.insert("food".to_string(), NbtValue::Float(20.0));
        nbt.insert("emeralds".to_string(), NbtValue::Short(12345));
        nbt.insert("timestamp".to_string(), NbtValue::Int(1424778774));

        let bytes = vec![
            0x0a,
                0x00, 0x00,
                0x08,
                    0x00, 0x04,
                    0x6e, 0x61, 0x6d, 0x65,
                    0x00, 0x09,
                    0x48, 0x65, 0x72, 0x6f, 0x62, 0x72, 0x69, 0x6e, 0x65,
                0x01,
                    0x00, 0x06,
                    0x68, 0x65, 0x61, 0x6c, 0x74, 0x68,
                    0x64,
                0x05,
                    0x00, 0x04,
                    0x66, 0x6f, 0x6f, 0x64,
                    0x41, 0xa0, 0x00, 0x00,
                0x02,
                    0x00, 0x08,
                    0x65, 0x6d, 0x65, 0x72, 0x61, 0x6c, 0x64, 0x73,
                    0x30, 0x39,
                0x03,
                    0x00, 0x09,
                    0x74, 0x69, 0x6d, 0x65, 0x73, 0x74, 0x61, 0x6d, 0x70,
                    0x54, 0xec, 0x66, 0x16,
            0x00
        ];

        // Test correct length.
        assert_eq!(bytes.len(), nbt.len());

        // We can only test if the decoded bytes match, since the HashMap does
        // not guarantee order (and so encoding is likely to be different, but
        // still correct).
        let mut src = io::Cursor::new(bytes);
        let file = <NbtFile as Protocol>::proto_decode(&mut src).unwrap();
        assert_eq!(&file, &nbt);
    }

    #[test]
    fn nbt_empty_nbtfile() {
        let nbt = NbtFile::new("".to_string());

        let bytes = vec![
            0x0a,
                0x00, 0x00,
            0x00
        ];

        // Test correct length.
        assert_eq!(bytes.len(), nbt.len());

        // Test encoding.
        let mut dst = Vec::new();
        <NbtFile as Protocol>::proto_encode(&nbt, &mut dst).unwrap();
        assert_eq!(&dst, &bytes);

        // Test decoding.
        let mut src = io::Cursor::new(bytes);
        let file = <NbtFile as Protocol>::proto_decode(&mut src).unwrap();
        assert_eq!(&file, &nbt);
    }

    #[test]
    fn nbt_nested_compound() {
        let mut inner = HashMap::new();
        inner.insert("test".to_string(), NbtValue::Byte(123));
        let mut nbt = NbtFile::new("".to_string());
        nbt.insert("inner".to_string(), NbtValue::Compound(inner));

        let bytes = vec![
            0x0a,
                0x00, 0x00,
                0x0a,
                    0x00, 0x05,
                    0x69, 0x6e, 0x6e, 0x65, 0x72,
                    0x01,
                    0x00, 0x04,
                    0x74, 0x65, 0x73, 0x74,
                    0x7b,
                0x00,
            0x00
        ];

        // Test correct length.
        assert_eq!(bytes.len(), nbt.len());

        // Test encoding.
        let mut dst = Vec::new();
        <NbtFile as Protocol>::proto_encode(&nbt, &mut dst).unwrap();
        assert_eq!(&dst, &bytes);

        // Test decoding.
        let mut src = io::Cursor::new(bytes);
        let file = <NbtFile as Protocol>::proto_decode(&mut src).unwrap();
        assert_eq!(&file, &nbt);
    }

    #[test]
    fn nbt_empty_list() {
        let mut nbt = NbtFile::new("".to_string());
        nbt.insert("list".to_string(), NbtValue::List(Vec::new()));

        let bytes = vec![
            0x0a,
                0x00, 0x00,
                0x09,
                    0x00, 0x04,
                    0x6c, 0x69, 0x73, 0x74,
                    0x01,
                    0x00, 0x00, 0x00, 0x00,
            0x00
        ];

        // Test correct length.
        assert_eq!(bytes.len(), nbt.len());

        // Test encoding.
        let mut dst = Vec::new();
        <NbtFile as Protocol>::proto_encode(&nbt, &mut dst).unwrap();
        assert_eq!(&dst, &bytes);

        // Test decoding.
        let mut src = io::Cursor::new(bytes);
        let file = <NbtFile as Protocol>::proto_decode(&mut src).unwrap();
        assert_eq!(&file, &nbt);
    }

    #[test]
    fn nbt_no_root() {
        let bytes = vec![0x00];
        // Will fail, because the root is not a compound.
        assert!(NbtFile::from_reader(&mut io::Cursor::new(bytes.as_slice())).is_err());
    }

    #[test]
    fn nbt_bad_compression() {
        // These aren't in the zlib or gzip format, so they'll fail.
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(NbtFile::from_gzip(bytes.as_slice()).is_err());
        assert!(NbtFile::from_zlib(bytes.as_slice()).is_err());
    }
}
