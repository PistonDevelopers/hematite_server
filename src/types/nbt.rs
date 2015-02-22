//! MC Named Binary Tag type.

use std::collections::HashMap;
use std::iter::AdditiveIterator;
use std::old_io::{ BufReader, IoError, IoErrorKind, IoResult };
use std::ops::Index;

use packet::Protocol;

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
    pub fn from_reader(r: &mut Reader) -> IoResult<Nbt> {
        let id = try!(r.read_i8());
        if id != 0x0a {
            return Err(IoError {
                kind: IoErrorKind::InvalidInput,
                desc: "invalid NBT file",
                detail: Some(format!("Invalid NBT file, must begin with a TAG_COMPOUND (0x0a), got {:02x}.", id))
            });
        }
        try!(Nbt::read_str(r)); // Root compound name is not used?
        let compound = try!(Nbt::read_compound(r));
        Ok(Nbt::Compound(compound))
    }

    pub fn from_gzip(data: &[u8]) -> IoResult<Nbt> {
        assert_eq!(&data[..4], [0x1f, 0x8b, 0x08, 0x00].as_slice());
        let data = inflate_bytes(&data[10..]).expect("inflate failed");
        Nbt::from_reader(&mut BufReader::new(data.as_slice()))
    }

    pub fn from_zlib(data: &[u8]) -> IoResult<Nbt> {
        let data = inflate_bytes_zlib(data).expect("inflate failed");
        Nbt::from_reader(&mut BufReader::new(data.as_slice()))
    }

    /// Writes a tag `name` into `dst`
    fn write_str<'a>(name: &'a str, dst: &mut Writer) -> IoResult<()> {
        try!(dst.write_be_u16(name.len() as u16));
        try!(dst.write_str(name));
        Ok(())
    }

    /// Reads a tag `name` from `src`
    fn read_str(src: &mut Reader) -> IoResult<String> {
        let length = try!(src.read_be_u16());
        let bytes = try!(src.read_exact(length as usize));
        let utf8_str = String::from_utf8(bytes).unwrap();
        Ok(utf8_str)
    }

    fn write_i8_array(array: &Vec<i8>, dst: &mut Writer) -> IoResult<()> {
        try!(dst.write_be_i32(array.len() as i32));
        for b in array.iter() {
            try!(dst.write_i8(*b));
        }
        Ok(())
    }

    fn read_i8_array(src: &mut Reader) -> IoResult<Vec<i8>> {
        let length = try!(src.read_be_i32()) as usize;
        let mut v = Vec::with_capacity(length);
        for _ in range(0, length) {
            v.push(try!(src.read_i8()));
        }
        Ok(v)
    }

    fn write_compound(compound: &Compound, dst: &mut Writer) -> IoResult<()> {
        try!(dst.write_be_i32(compound.len() as i32));
        for (name, value) in compound.iter() {
            try!(Nbt::write_str(name.as_slice(), dst));
            try!(<Nbt as Protocol>::proto_encode(value, dst));
        }
        try!(dst.write_i8(0x00)); // TAG_END
        Ok(())
    }

    fn read_compound(src: &mut Reader) -> IoResult<Compound> {
        let mut compound = HashMap::new();
        loop {
            let tag = try!(src.read_i8());
            if tag == 0x00 { break }
            let key = try!(Nbt::read_str(src));
            println!("read_compound {:02x} {}", tag, key);
            let value = match tag {
                0x00 => unreachable!(),
                0x01 => Nbt::Byte(try!(src.read_i8())),
                0x02 => Nbt::Short(try!(src.read_be_i16())),
                0x03 => Nbt::Int(try!(src.read_be_i32())),
                0x04 => Nbt::Long(try!(src.read_be_i64())),
                0x05 => Nbt::Float(try!(src.read_be_f32())),
                0x06 => Nbt::Double(try!(src.read_be_f64())),
                0x07 => Nbt::ByteArray(try!(Nbt::read_i8_array(src))),
                0x08 => Nbt::String(try!(Nbt::read_str(src))),
                0x09 => Nbt::List(try!(<List as Protocol>::proto_decode(src))),
                0x0a => Nbt::Compound(try!(Nbt::read_compound(src))),
                0x0b => Nbt::IntArray(try!(Nbt::read_i32_array(src))),
                value => panic!("Invalid NBT tag value {}", value)
            };
            compound.insert(key, value);
        }
        Ok(compound)
    }

    fn write_i32_array(array: &Vec<i32>, dst: &mut Writer) -> IoResult<()> {
        try!(dst.write_be_i32(array.len() as i32));
        for value in array.iter() {
            try!(dst.write_be_i32(*value));
        }
        Ok(())
    }

    fn read_i32_array(src: &mut Reader) -> IoResult<Vec<i32>> {
        let length = try!(src.read_be_i32()) as usize;
        let mut array = Vec::with_capacity(length);
        for _ in range(0, length) {
            array.push(try!(src.read_be_i32()));
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
                let mut size = 0;
                for (name, nbt) in value.iter() {
                    size += 2 + name.len(); // Basically a Nbt::String without tag id
                    size += <Nbt as Protocol>::proto_len(nbt);
                }
                size + 1
            },
            Nbt::IntArray(ref value) => 4 + 4 * value.len(),
        };
        println!("Nbt.proto_len for {:?} is {}", *value, 1 + size);
        // All tags are preceded by TypeId
        1 + size
    }

    fn proto_encode(value: &Nbt, dst: &mut Writer) -> IoResult<()> {
        match value {
            &Nbt::End                    => try!(dst.write_i8(0x00)),
            &Nbt::Byte(x)                => { try!(dst.write_i8(0x01)); try!(dst.write_i8(x)); },
            &Nbt::Short(x)               => { try!(dst.write_i8(0x02)); try!(dst.write_be_i16(x)); },
            &Nbt::Int(x)                 => { try!(dst.write_i8(0x03)); try!(dst.write_be_i32(x)); },
            &Nbt::Long(x)                => { try!(dst.write_i8(0x04)); try!(dst.write_be_i64(x)); },
            &Nbt::Float(x)               => { try!(dst.write_i8(0x05)); try!(dst.write_be_f32(x)); },
            &Nbt::Double(x)              => { try!(dst.write_i8(0x06)); try!(dst.write_be_f64(x)); },
            &Nbt::ByteArray(ref array)   => { try!(dst.write_i8(0x07)); try!(Nbt::write_i8_array(array, dst)); },
            &Nbt::String(ref value)      => { try!(dst.write_i8(0x08)); try!(Nbt::write_str(value.as_slice(), dst)); },
            &Nbt::List(ref list)         => { try!(dst.write_i8(0x09)); try!(<List as Protocol>::proto_encode(list, dst)); },
            &Nbt::Compound(ref compound) => { try!(dst.write_i8(0x0a)); try!(Nbt::write_compound(compound, dst)); },
            &Nbt::IntArray(ref array)    => { try!(dst.write_i8(0x0b)); try!(Nbt::write_i32_array(array, dst)); },
        }
        Ok(())
    }

    fn proto_decode(src: &mut Reader) -> IoResult<Nbt> {
        let tag = try!(src.read_i8());
        println!("Nbt.proto_decode {:02x}", tag);
        match tag {
            0x00 => Ok(Nbt::End),
            0x01 => Ok(Nbt::Byte( try!(src.read_i8()) )),
            0x02 => Ok(Nbt::Short( try!(src.read_be_i16()) )),
            0x03 => Ok(Nbt::Int( try!(src.read_be_i32()) )),
            0x04 => Ok(Nbt::Long( try!(src.read_be_i64()) )),
            0x05 => Ok(Nbt::Float( try!(src.read_be_f32()) )),
            0x06 => Ok(Nbt::Double( try!(src.read_be_f64()) )),
            0x07 => Ok(Nbt::ByteArray( try!(Nbt::read_i8_array(src)) )),
            0x08 => Ok(Nbt::String( try!(Nbt::read_str(src)) )),
            0x09 => Ok(Nbt::List( try!(<List as Protocol>::proto_decode(src)) )),
            0x0a => Ok(Nbt::Compound( try!(Nbt::read_compound(src)) )),
            0x0b => Ok(Nbt::IntArray( try!(Nbt::read_i32_array(src)) )),
            value => panic!("Unknown NBT tag value {}", value),
        }
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
            },
            List::Compound(ref value) => {
                let mut size = 0;
                for compound in value.iter() {
                    for (name, nbt) in compound.iter() {
                        size += 2 + name.len(); // Basically a Nbt::String without tag id
                        size += <Nbt as Protocol>::proto_len(nbt);
                    }
                    size += 1; // END
                }
                size
            },
            List::IntArray(ref value) => 4 + 4 * value.len(),
        }
    }

    fn proto_encode(value: &List, dst: &mut Writer) -> IoResult<()> {
        match value {
            &List::Byte(ref xs) => {
                try!(dst.write_i8(0x01));
                try!(dst.write_be_i32(xs.len() as i32));
                for value in xs.iter() {
                    try!(dst.write_i8(*value));
                }
            }
            &List::Short(ref xs) => {
                try!(dst.write_i8(0x02));
                try!(dst.write_be_i32(xs.len() as i32));
                for value in xs.iter() {
                    try!(dst.write_be_i16(*value));
                }
            }
            &List::Int(ref xs) => {
                try!(dst.write_i8(0x03));
                try!(dst.write_be_i32(xs.len() as i32));
                for value in xs.iter() {
                    try!(dst.write_be_i32(*value));
                }
            }
            &List::Long(ref xs) => {
                try!(dst.write_i8(0x04));
                try!(dst.write_be_i32(xs.len() as i32));
                for value in xs.iter() {
                    try!(dst.write_be_i64(*value));
                }
            }
            &List::Float(ref xs) => {
                try!(dst.write_i8(0x05));
                try!(dst.write_be_i32(xs.len() as i32));
                for value in xs.iter() {
                    try!(dst.write_be_f32(*value));
                }
            }
            &List::Double(ref xs) => {
                try!(dst.write_i8(0x06));
                try!(dst.write_be_i32(xs.len() as i32));
                for value in xs.iter() {
                    try!(dst.write_be_f64(*value));
                }
            }
            &List::ByteArray(ref xs) => {
                try!(dst.write_i8(0x07));
                try!(dst.write_be_i32(xs.len() as i32));
                for array in xs.iter() {
                    try!(Nbt::write_i8_array(array, dst));
                }
            }
            &List::String(ref xs) => {
                try!(dst.write_i8(0x08));
                try!(dst.write_be_i32(xs.len() as i32));
                for value in xs.iter() {
                    try!(Nbt::write_str(value.as_slice(), dst));
                }
            }
            &List::List(ref xs) => {
                try!(dst.write_i8(0x09));
                try!(dst.write_be_i32(xs.len() as i32));
                for value in xs.iter() {
                    try!(<List as Protocol>::proto_encode(value, dst));
                }
            }
            &List::Compound(ref xs) => {
                try!(dst.write_i8(0x0a));
                try!(dst.write_be_i32(xs.len() as i32));
                for compound in xs.iter() {
                    try!(Nbt::write_compound(compound, dst));
                }
            }
            &List::IntArray(ref xs) => {
                try!(dst.write_i8(0x0b));
                try!(dst.write_be_i32(xs.len() as i32));
                for array in xs.iter() {
                    try!(Nbt::write_i32_array(array, dst));
                }
            }
        }
        Ok(())
    }

    fn proto_decode(src: &mut Reader) -> IoResult<List> {
        let tag = try!(src.read_i8());
        let length = try!(src.read_be_i32()) as usize;
        match tag {
            1 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    v.push(try!(src.read_i8()));
                }
                Ok(List::Byte(v))
            }
            2 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    v.push(try!(src.read_be_i16()));
                }
                Ok(List::Short(v))
            }
            3 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    v.push(try!(src.read_be_i32()));
                }
                Ok(List::Int(v))
            }
            4 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    v.push(try!(src.read_be_i64()));
                }
                Ok(List::Long(v))
            }
            5 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    v.push(try!(src.read_be_f32()));
                }
                Ok(List::Float(v))
            }
            6 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    v.push(try!(src.read_be_f64()));
                }
                Ok(List::Double(v))
            }
            7 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    let value = try!(<Nbt as Protocol>::proto_decode(src));
                    let array = match value {
                        Nbt::ByteArray(arr) => arr,
                        other => panic!("Expecting Nbt::ByteArray, got {:?}", other),
                    };
                    v.push(array);
                }
                Ok(List::ByteArray(v))
            }
            8 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    let length = try!(src.read_be_u16()) as usize;
                    let sb = try!(src.read_exact(length));
                    let utf8s = String::from_utf8(sb).unwrap();
                    v.push(utf8s);
                }
                Ok(List::String(v))
            }
            9 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    let value = try!(<List as Protocol>::proto_decode(src));
                    v.push(value);
                }
                Ok(List::List(v))
            }
            10 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    // let name = try!(Nbt::read_str(src)); // Not used?
                    let compound = try!(Nbt::read_compound(src));
                    v.push(compound);
                }
                Ok(List::Compound(v))
            }
            11 => {
                let mut v = Vec::with_capacity(length);
                for _ in range(0, length) {
                    let value = try!(Nbt::read_i32_array(src));
                    v.push(value);
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

    use std::old_io::File;

    use packet::Protocol;

    #[test]
    fn nbt_small1() {
        let path = &Path::new("tests/small1.nbt");
        let mut file = File::open(path).unwrap();
        let nbt = Nbt::from_reader(&mut file).unwrap();
        println!("nbt_small1 {:?}", nbt);
    }

    #[test]
    fn nbt_small2() {
        let path = &Path::new("tests/small2.nbt");
        let mut file = File::open(path).unwrap();
        let nbt = Nbt::from_reader(&mut file).unwrap();
        println!("nbt_small2 {:?}", nbt);
    }

    #[test]
    fn nbt_small3() {
        let path = &Path::new("tests/small3.nbt");
        let mut file = File::open(path).unwrap();
        let nbt = Nbt::from_reader(&mut file).unwrap();
        println!("nbt_small3 {:?}", nbt);
    }

    #[test]
    fn nbt_small4() {
        let path = &Path::new("tests/small4.nbt");
        let mut file = File::open(path).unwrap();
        let nbt = Nbt::from_reader(&mut file).unwrap();
        println!("nbt_small4 {:?}", nbt);
    }

    #[test]
    fn nbt_big1() {
        let path = &Path::new("tests/big1.nbt");
        let mut file = File::open(path).unwrap();
        let data = file.read_to_end().unwrap();
        let nbt = Nbt::from_gzip(data.as_slice()).unwrap();
        println!("nbt_big1 {:?}", nbt);
    }
}
