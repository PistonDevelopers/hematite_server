//! MC Named Binary Tag type.

use byteorder::{BigEndian, WriteBytesExt};

use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use std::iter::AdditiveIterator;
use std::ops::Index;

use packet::{Protocol, ReadExactExt};

use flate::{ inflate_bytes, inflate_bytes_zlib };

/// Represents a NBT value
#[derive(Clone, Debug, PartialEq)]
pub enum Nbt {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(List),
    Compound(Compound),
    IntArray(Vec<i32>),
}

/// An ordered list of NBT values.
#[derive(Clone, Debug, PartialEq)]
pub enum List {
    Byte(Vec<i8>),
    Short(Vec<i16>),
    Int(Vec<i32>),
    Long(Vec<i64>),
    Float(Vec<f32>),
    Double(Vec<f64>),
    ByteArray(Vec<Vec<i8>>),
    String(Vec<String>),
    List(Vec<List>),
    Compound(Vec<Compound>),
    IntArray(Vec<Vec<i32>>),
}

/// An unordered list of named NBT values.
pub type Compound = HashMap<String, Nbt>;

impl Nbt {
    /// Decodes a NBT value from `r`
    ///
    /// Every NBT file will always begin with a TAG_COMPOUND (0x0a). No
    /// exceptions.
    pub fn from_reader(src: &mut Read) -> io::Result<Nbt> {
        <Nbt as Protocol>::proto_decode(src)
    }
    pub fn from_gzip(data: &[u8]) -> io::Result<Nbt> {
        assert_eq!(&data[..4], [0x1f, 0x8b, 0x08, 0x00].as_slice());
        let data = inflate_bytes(&data[10..]).expect("inflate failed");
        Nbt::from_reader(&mut io::Cursor::new(data.as_slice()))
    }
    pub fn from_zlib(data: &[u8]) -> io::Result<Nbt> {
        let data = inflate_bytes_zlib(data).expect("inflate failed");
        Nbt::from_reader(&mut io::Cursor::new(data.as_slice()))
    }
    pub fn as_byte(&self) -> Option<i8> {
        match *self { Nbt::Byte(b) => Some(b), _ => None }
    }
    pub fn into_compound(self) -> Result<Compound, Nbt> {
        match self { Nbt::Compound(c) => Ok(c), x => Err(x) }
    }
    pub fn into_compound_list(self) -> Result<Vec<Compound>, Nbt> {
        match self { Nbt::List(List::Compound(c)) => Ok(c), x => Err(x) }
    }
    pub fn as_bytearray<'a>(&'a self) -> Option<&'a [i8]> {
        match *self { Nbt::ByteArray(ref b) => Some(b.as_slice()), _ => None }
    }
    pub fn into_bytearray(self) -> Result<Vec<i8>, Nbt> {
        match self { Nbt::ByteArray(b) => Ok(b), x => Err(x) }
    }
    // pub fn as_float_list<'a>(&'a self) -> Option<&'a [f32]> {
    //     match *self { NbtList(FloatList(ref f)) => Some(f.as_slice()), _ => None }
    // }
    // pub fn as_double_list<'a>(&'a self) -> Option<&'a [f64]> {
    //     match *self { NbtList(DoubleList(ref d)) => Some(d.as_slice()), _ => None }
    // }

    fn id(&self) -> u8 {
        match *self {
            Nbt::End => 0,
            Nbt::Byte(_) => 1,
            Nbt::Short(_) => 2,
            Nbt::Int(_) => 3,
            Nbt::Long(_) => 4,
            Nbt::Float(_) => 5,
            Nbt::Double(_) => 6,
            Nbt::ByteArray(_) => 7,
            Nbt::String(_) => 8,
            Nbt::List(_) => 9,
            Nbt::Compound(_) => 10,
            Nbt::IntArray(_) => 11
        }
    }

    fn write_str<'a>(name: &'a str, dst: &mut Write) -> io::Result<()> {
        let len = name.len() as u16;
        try!(<u16 as Protocol>::proto_encode(&len, dst));
        if len != 0 { try!(dst.write_all(name.as_bytes())); }
        Ok(())
    }

    fn read_str(mut src: &mut Read) -> io::Result<String> {
        let len = try!(<u16 as Protocol>::proto_decode(src));
        if len == 0 { return Ok("".to_string()); }
        let bytes = try!(src.read_exact(len as usize));
        let utf8_str = String::from_utf8(bytes).unwrap();
        Ok(utf8_str)
    }

    fn write_i8_array(array: &Vec<i8>, dst: &mut Write) -> io::Result<()> {
        let len = array.len() as i32;
        try!(<i32 as Protocol>::proto_encode(&len, dst));
        for value in array.iter() {
            try!(<i8 as Protocol>::proto_encode(value, dst));
        }
        Ok(())
    }

    fn read_i8_array(src: &mut Read) -> io::Result<Vec<i8>> {
        let length = try!(<i32 as Protocol>::proto_decode(src)) as usize;
        let mut v = Vec::with_capacity(length);
        for _ in range(0, length) {
            v.push(try!(<i8 as Protocol>::proto_decode(src)));
        }
        Ok(v)
    }

    fn write_list<T: Protocol>(id: i8, xs: &Vec<<T as Protocol>::Clean>, dst: &mut Write) -> io::Result<()> {
        try!(<i8 as Protocol>::proto_encode(&id, dst));
        let len = xs.len() as i32;
        try!(<i32 as Protocol>::proto_encode(&len, dst));
        for value in xs.iter() {
            try!(<T as Protocol>::proto_encode(value, dst));
        }
        Ok(())
    }

    fn read_list<T: Protocol>(length: usize, src: &mut Read) -> io::Result<Vec<<T as Protocol>::Clean>> {
        let mut v = Vec::with_capacity(length);
        for _ in range(0, length) {
            v.push(try!(<T as Protocol>::proto_decode(src)));
        }
        Ok(v)
    }

    fn write_compound(compound: &Compound, dst: &mut Write) -> io::Result<()> {
        for (name, value) in compound.iter() {
            try!(<u8 as Protocol>::proto_encode(&value.id(), dst));
            try!(Nbt::write_str(name.as_slice(), dst));
            match value {
                &Nbt::End                    => {}
                &Nbt::Byte(x)                => { try!(<i8 as Protocol>::proto_encode(&x, dst)); }
                &Nbt::Short(x)               => { try!(<i16 as Protocol>::proto_encode(&x, dst)); }
                &Nbt::Int(x)                 => { try!(<i32 as Protocol>::proto_encode(&x, dst)); }
                &Nbt::Long(x)                => { try!(<i64 as Protocol>::proto_encode(&x, dst)); }
                &Nbt::Float(x)               => { try!(<f32 as Protocol>::proto_encode(&x, dst)); }
                &Nbt::Double(x)              => { try!(<f64 as Protocol>::proto_encode(&x, dst)); }
                &Nbt::ByteArray(ref array)   => { try!(Nbt::write_i8_array(array, dst)); }
                &Nbt::String(ref value)      => { try!(Nbt::write_str(value.as_slice(), dst)); }
                &Nbt::List(ref list)         => { try!(<List as Protocol>::proto_encode(list, dst)); }
                &Nbt::Compound(ref compound) => { try!(Nbt::write_compound(compound, dst)); }
                &Nbt::IntArray(ref array)    => { try!(Nbt::write_i32_array(array, dst)); }
            }
        }
        try!(<i8 as Protocol>::proto_encode(&0, dst)); // TAG_END
        Ok(())
    }

    fn read_compound(src: &mut Read) -> io::Result<Compound> {
        let mut map = HashMap::new();
        loop {
            let tag = try!(<i8 as Protocol>::proto_decode(src));
            if tag == 0x00 { break }
            let key = try!(Nbt::read_str(src));
            let value = match tag {
                0x00 => unreachable!(),
                0x01 => Nbt::Byte(try!(<i8 as Protocol>::proto_decode(src))),
                0x02 => Nbt::Short(try!(<i16 as Protocol>::proto_decode(src))),
                0x03 => Nbt::Int(try!(<i32 as Protocol>::proto_decode(src))),
                0x04 => Nbt::Long(try!(<i64 as Protocol>::proto_decode(src))),
                0x05 => Nbt::Float(try!(<f32 as Protocol>::proto_decode(src))),
                0x06 => Nbt::Double(try!(<f64 as Protocol>::proto_decode(src))),
                0x07 => Nbt::ByteArray(try!(Nbt::read_i8_array(src))),
                0x08 => Nbt::String(try!(Nbt::read_str(src))),
                0x09 => Nbt::List(try!(<List as Protocol>::proto_decode(src))),
                0x0a => Nbt::Compound(try!(Nbt::read_compound(src))),
                0x0b => Nbt::IntArray(try!(Nbt::read_i32_array(src))),
                value => panic!("Invalid NBT tag value {}", value)
            };
            map.insert(key, value);
        }
        Ok(map)
    }

    fn write_i32_array(array: &Vec<i32>, dst: &mut Write) -> io::Result<()> {
        let len = array.len() as i32;
        try!(<i32 as Protocol>::proto_encode(&len, dst));
        for value in array.iter() {
            try!(<i32 as Protocol>::proto_encode(value, dst));
        }
        Ok(())
    }

    fn read_i32_array(src: &mut Read) -> io::Result<Vec<i32>> {
        let length = try!(<i32 as Protocol>::proto_decode(src)) as usize;
        let mut array = Vec::with_capacity(length);
        for _ in range(0, length) {
            array.push(try!(<i32 as Protocol>::proto_decode(src)));
        }
        Ok(array)
    }
}

impl<'a> Index<&'a str> for Nbt {
    type Output = Nbt;

    fn index<'b>(&'b self, s: &&'a str) -> &'b Nbt {
        match *self {
            Nbt::Compound(ref compound) => compound.get(*s).unwrap(),
            _ => panic!("cannot index non-compound Nbt ({:?}) with '{}'", self, s)
        }
    }
}

impl Protocol for Nbt {
    type Clean = Nbt;

    fn proto_len(value: &Nbt) -> usize {
        let size = match *value {
            Nbt::End => 0,
            Nbt::Byte(_) => 1,
            Nbt::Short(_) => 2,
            Nbt::Int(_) => 4,
            Nbt::Long(_) => 8,
            Nbt::Float(_) => 4,
            Nbt::Double(_) => 8,
            Nbt::ByteArray(ref value) => 4 + value.len(),
            Nbt::String(ref value) => 2 + value.len(),
            Nbt::List(ref value) => <List as Protocol>::proto_len(value),
            Nbt::Compound(ref value) => {
                1 + value.iter().map(|(name, nbt)| {
                    2 + name.len() + <Nbt as Protocol>::proto_len(nbt)
                }).sum()
            }
            Nbt::IntArray(ref value) => 4 + 4 * value.len(),
        };
        1 + size // All tags are preceded by TypeId
    }

    fn proto_encode(value: &Nbt, dst: &mut Write) -> io::Result<()> {
        // Write root compound
        try!(<u8 as Protocol>::proto_encode(&0x0a, dst));
        try!(Nbt::write_str("", dst));
        // Write `value` contents
        match value {
            &Nbt::Compound(ref compound) => try!(Nbt::write_compound(compound, dst)),
            _ => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid NBT file", Some(format!("root value must be NBT Compound"))));
            }
        }
        Ok(())
    }

    fn proto_decode(src: &mut Read) -> io::Result<Nbt> {
        // Read root compound
        let id = try!(<u8 as Protocol>::proto_decode(src));
        if id != 0x0a {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid NBT file", Some(format!("root value must be NBT Compound"))));
        }
        try!(Nbt::read_str(src)); // compound name
        // Read root contents
        let nbt = try!(Nbt::read_compound(src));
        Ok(Nbt::Compound(nbt))
    }
}

impl Protocol for List {
    type Clean = List;

    fn proto_len(value: &List) -> usize {
        match *value {
            List::Byte(ref value) => 1 * value.len(),
            List::Short(ref value) => 2 * value.len(),
            List::Int(ref value) => 4 * value.len(),
            List::Long(ref value) => 8 * value.len(),
            List::Float(ref value) => 4 * value.len(),
            List::Double(ref value) => 8 * value.len(),
            List::ByteArray(ref value) => 4 + value.len(),
            List::String(ref value) => 2 + value.len(),
            List::List(ref value) => {
                5 + value.iter().map(|c| <List as Protocol>::proto_len(c)).sum()
            }
            List::Compound(ref value) => {
                1 + value.iter().map(|c| {
                    c.iter().map(|(name, nbt)| {
                        2 + name.len() + <Nbt as Protocol>::proto_len(nbt)
                    }).sum()
                }).sum()
            }
            List::IntArray(ref value) => 4 + 4 * value.len(),
        }
    }

    fn proto_encode(value: &List, mut dst: &mut Write) -> io::Result<()> {
        match value {
            &List::Byte(ref xs) =>      try!(Nbt::write_list::<i8>(0x01, xs, dst)),
            &List::Short(ref xs) =>     try!(Nbt::write_list::<i16>(0x02, xs, dst)),
            &List::Int(ref xs) =>       try!(Nbt::write_list::<i32>(0x03, xs, dst)),
            &List::Long(ref xs) =>      try!(Nbt::write_list::<i64>(0x04, xs, dst)),
            &List::Float(ref xs) =>     try!(Nbt::write_list::<f32>(0x05, xs, dst)),
            &List::Double(ref xs) =>    try!(Nbt::write_list::<f64>(0x06, xs, dst)),
            &List::ByteArray(ref xs) => {
                try!(dst.write_i8(0x07));
                try!(dst.write_i32::<BigEndian>(xs.len() as i32));
                for array in xs.iter() {
                    try!(Nbt::write_i8_array(array, dst));
                }
            }
            &List::String(ref xs) => {
                try!(dst.write_i8(0x08));
                try!(dst.write_i32::<BigEndian>(xs.len() as i32));
                for value in xs.iter() {
                    try!(Nbt::write_str(value.as_slice(), dst));
                }
            }
            &List::List(ref xs) => try!(Nbt::write_list::<List>(0x09, xs, dst)),
            &List::Compound(ref xs) => {
                try!(dst.write_i8(0x0a));
                try!(dst.write_i32::<BigEndian>(xs.len() as i32));
                for compound in xs.iter() {
                    try!(Nbt::write_compound(compound, dst));
                }
            }
            &List::IntArray(ref xs) => {
                try!(dst.write_i8(0x0b));
                try!(dst.write_i32::<BigEndian>(xs.len() as i32));
                for array in xs.iter() {
                    try!(Nbt::write_i32_array(array, dst));
                }
            }
        }
        Ok(())
    }

    fn proto_decode(src: &mut Read) -> io::Result<List> {
        let tag = try!(<i8 as Protocol>::proto_decode(src));
        let length = try!(<i32 as Protocol>::proto_decode(src)) as usize;
        match tag {
            1 => Ok(List::Byte(try!(Nbt::read_list::<i8>(length, src)))),
            2 => Ok(List::Short(try!(Nbt::read_list::<i16>(length, src)))),
            3 => Ok(List::Int(try!(Nbt::read_list::<i32>(length, src)))),
            4 => Ok(List::Long(try!(Nbt::read_list::<i64>(length, src)))),
            5 => Ok(List::Float(try!(Nbt::read_list::<f32>(length, src)))),
            6 => Ok(List::Double(try!(Nbt::read_list::<f64>(length, src)))),
            7 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    v.push(try!(Nbt::read_i8_array(src)));
                }
                Ok(List::ByteArray(v))
            }
            8 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    v.push(try!(Nbt::read_str(src)));
                }
                Ok(List::String(v))
            }
            9 => Ok(List::List(try!(Nbt::read_list::<List>(length, src)))),
            10 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    v.push(try!(Nbt::read_compound(src)));
                }
                Ok(List::Compound(v))
            }
            11 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    v.push(try!(Nbt::read_i32_array(src)));
                }
                Ok(List::IntArray(v))
            }
            value => panic!("Unknown NBT tag value {}", value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::io;

    use packet::Protocol;

    #[test]
    fn nbt_encode_decode() {
        let mut c = HashMap::new();
        c.insert("name".to_string(), Nbt::String("Herobrine".to_string()));
        c.insert("health".to_string(), Nbt::Byte(100));
        c.insert("food".to_string(), Nbt::Float(20.0));
        c.insert("emeralds".to_string(), Nbt::Short(12345));
        c.insert("timestamp".to_string(), Nbt::Int(1424778774));
        let nbt = Nbt::Compound(c);

        // let bytes = vec![
        //     0x0a,
        //         0x00, 0x00,
        //         0x08,
        //             0x00, 0x04,
        //             0x6e, 0x61, 0x6d, 0x65,
        //             0x00, 0x09,
        //             0x48, 0x65, 0x72, 0x6f, 0x62, 0x72, 0x69, 0x6e, 0x65,
        //         0x01,
        //             0x00, 0x06,
        //             0x68, 0x65, 0x61, 0x6c, 0x74, 0x68,
        //             0x64,
        //         0x05,
        //             0x00, 0x04,
        //             0x66, 0x6f, 0x6f, 0x64,
        //             0x41, 0xa0, 0x00, 0x00,
        //         0x02,
        //             0x00, 0x08,
        //             0x65, 0x6d, 0x65, 0x72, 0x61, 0x6c, 0x64, 0x73,
        //             0x30, 0x39,
        //         0x03,
        //             0x00, 0x09,
        //             0x74, 0x69, 0x6d, 0x65, 0x73, 0x74, 0x61, 0x6d, 0x70,
        //             0x54, 0xec, 0x66, 0x16,
        //     0x00
        // ];

        let mut dst = Vec::new();
        <Nbt as Protocol>::proto_encode(&nbt, &mut dst).unwrap();
        // assert_eq!(&dst, &bytes);

        let mut src = io::Cursor::new(dst);
        let nbt2 = <Nbt as Protocol>::proto_decode(&mut src).unwrap();
        assert_eq!(nbt2, nbt);
    }

    #[test]
    fn nbt_empty_compound() {
        let nbt = Nbt::Compound(HashMap::new());

        let bytes = vec![
            0x0a,
                0x00, 0x00,
            0x00
        ];

        let mut dst = Vec::new();
        <Nbt as Protocol>::proto_encode(&nbt, &mut dst).unwrap();
        assert_eq!(&dst, &bytes);
    }

    #[test]
    fn nbt_nested_compound() {
        let mut c2 = HashMap::new();
        c2.insert("test".to_string(), Nbt::Byte(123));
        let mut c1 = HashMap::new();
        c1.insert("inner".to_string(), Nbt::Compound(c2));
        let nbt = Nbt::Compound(c1);

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

        let mut dst = Vec::new();
        <Nbt as Protocol>::proto_encode(&nbt, &mut dst).unwrap();
        assert_eq!(&dst, &bytes);
    }

    #[test]
    fn nbt_empty_list() {
        let mut c = HashMap::new();
        c.insert("list".to_string(), Nbt::List(List::Byte(Vec::new())));
        let nbt = Nbt::Compound(c);

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

        let mut dst = Vec::new();
        <Nbt as Protocol>::proto_encode(&nbt, &mut dst).unwrap();
        assert_eq!(&dst, &bytes);
    }
}
