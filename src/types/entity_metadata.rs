//! MC Protocol Metadata data type.

use std::collections::HashMap;
use std::io::prelude::*;
use std::io;

use packet::Protocol;
use types::Slot;

/// Entity Metadata Format
///
/// All entities must send at least one item of metadata, in most cases this
/// will be the health item.
///
/// The entity metadata format is quirky dictionary format, where the key and
/// the value's type are packed in a single byte.
///
/// Note that entity metadata is a totally distinct concept from block
/// metadata.
#[derive(Debug)]
pub struct EntityMetadata {
    dict: HashMap<u8, Entry>
}

#[derive(Debug)]
pub enum Entry {
    Byte(u8),
    Short(i16),
    Int(i32),
    Float(f32),
    String(String),
    Slot(Option<Slot>),
    Int3([i32; 3]),
    Float3([f32; 3])
}

impl EntityMetadata {
    pub fn new() -> EntityMetadata {
        EntityMetadata { dict: HashMap::new() }
    }
}

impl Protocol for EntityMetadata {
    type Clean = EntityMetadata;
    fn proto_len(value: &EntityMetadata) -> usize {
        use std::iter::AdditiveIterator;

        fn entry_len(value: &Entry) -> usize {
            match value {
                &Entry::Byte(_) => 1,
                &Entry::Short(_) => 2,
                &Entry::Int(_)
                | &Entry::Float(_) => 4,
                &Entry::String(ref s) => <String as Protocol>::proto_len(s),
                &Entry::Slot(ref s) => <Option<Slot> as Protocol>::proto_len(s),
                &Entry::Int3(_)
                | &Entry::Float3(_) => 12,
            }
        }
        value.dict.values().map(|v| entry_len(v)).sum()
    }
    fn proto_encode(value: &EntityMetadata, dst: &mut Write) -> io::Result<()> {
        fn key(k: u8, idx: u8) -> u8 {
            (k << 5 | idx & 0x1f) & 0xff
        }
        for (idx, value) in value.dict.iter() {
            match value {
                &Entry::Byte(ref b) => {
                    try!(<u8 as Protocol>::proto_encode(&key(0, *idx), dst));
                    try!(<u8 as Protocol>::proto_encode(b, dst));
                }
                &Entry::Short(ref s) => {
                    try!(<u8 as Protocol>::proto_encode(&key(1, *idx), dst));
                    try!(<i16 as Protocol>::proto_encode(s, dst));
                }
                &Entry::Int(ref i) => {
                    try!(<u8 as Protocol>::proto_encode(&key(2, *idx), dst));
                    try!(<i32 as Protocol>::proto_encode(i, dst));
                }
                &Entry::Float(ref f) => {
                    try!(<u8 as Protocol>::proto_encode(&key(3, *idx), dst));
                    try!(<f32 as Protocol>::proto_encode(f, dst));
                }
                &Entry::String(ref s) => {
                    try!(<u8 as Protocol>::proto_encode(&key(4, *idx), dst));
                    try!(<String as Protocol>::proto_encode(s, dst));
                }
                &Entry::Slot(ref s) => {
                    try!(<u8 as Protocol>::proto_encode(&key(5, *idx), dst));
                    try!(<Option<Slot> as Protocol>::proto_encode(s, dst));
                }
                &Entry::Int3(ref xyz) => {
                    try!(<u8 as Protocol>::proto_encode(&key(6, *idx), dst));
                    try!(<[i32; 3] as Protocol>::proto_encode(xyz, dst));
                }
                &Entry::Float3(ref xyz) => {
                    try!(<u8 as Protocol>::proto_encode(&key(7, *idx), dst));
                    try!(<[f32; 3] as Protocol>::proto_encode(xyz, dst));
                }
            };
        }
        try!(<u8 as Protocol>::proto_encode(&0x7f, dst));
        Ok(())
    }
    fn proto_decode(src: &mut Read) -> io::Result<EntityMetadata> {
        let mut dict = HashMap::new();
        loop {
            let item = try!(<u8 as Protocol>::proto_decode(src));
            if item == 0x7F {
                break;
            }
            let idx = item & 0x1F;
            let ty = item >> 5;
            let value = match ty {
                0 => Entry::Byte(try!(<u8 as Protocol>::proto_decode(src))),
                1 => Entry::Short(try!(<i16 as Protocol>::proto_decode(src))),
                2 => Entry::Int(try!(<i32 as Protocol>::proto_decode(src))),
                3 => Entry::Float(try!(<f32 as Protocol>::proto_decode(src))),
                4 => Entry::String(try!(<String as Protocol>::proto_decode(src))),
                5 => Entry::Slot(try!(<Option<Slot> as Protocol>::proto_decode(src))),
                6 => Entry::Int3(try!(<[i32; 3] as Protocol>::proto_decode(src))),
                7 => Entry::Float3(try!(<[f32; 3] as Protocol>::proto_decode(src))),
                ty => {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown type", Some(format!("unknown type {:x}", ty))));
                }
            };
            dict.insert(idx, value);
        }
        Ok(EntityMetadata{ dict: dict })
    }
}
