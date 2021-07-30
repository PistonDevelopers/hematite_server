//! MC Protocol Metadata data type.

use std::collections::HashMap;
use std::io;
use std::io::prelude::*;

use crate::packet::Protocol;
use crate::types::Slot;

/// Entity Metadata Format
///
/// All entities must send at least one item of metadata, in most cases this
/// will be the health item.
///
/// The entity metadata format is quirky dictionary format, where the key, and
/// the value's type are packed in a single byte.
///
/// Note that entity metadata is a totally distinct concept from block
/// metadata.
#[derive(Debug)]
pub struct EntityMetadata {
    dict: HashMap<u8, Entry>,
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
    Float3([f32; 3]),
}

impl EntityMetadata {
    #[must_use]
    pub fn new() -> EntityMetadata {
        EntityMetadata {
            dict: HashMap::new(),
        }
    }
}

impl Protocol for EntityMetadata {
    type Clean = EntityMetadata;
    fn proto_len(value: &EntityMetadata) -> usize {
        fn entry_len(value: &Entry) -> usize {
            match value {
                Entry::Byte(_) => 1,
                Entry::Short(_) => 2,
                Entry::Int(_) | Entry::Float(_) => 4,
                Entry::String(ref s) => <String as Protocol>::proto_len(s),
                Entry::Slot(ref s) => <Option<Slot> as Protocol>::proto_len(s),
                Entry::Int3(_) | Entry::Float3(_) => 12,
            }
        }
        value.dict.values().map(entry_len).sum()
    }
    fn proto_encode(value: &EntityMetadata, dst: &mut dyn Write) -> io::Result<()> {
        fn key(k: u8, idx: u8) -> u8 {
            (k << 5 | idx & 0x1f) & 0xff
        }
        for (idx, value) in &value.dict {
            match value {
                Entry::Byte(ref b) => {
                    <u8 as Protocol>::proto_encode(&key(0, *idx), dst)?;
                    <u8 as Protocol>::proto_encode(b, dst)?;
                }
                Entry::Short(ref s) => {
                    <u8 as Protocol>::proto_encode(&key(1, *idx), dst)?;
                    <i16 as Protocol>::proto_encode(s, dst)?;
                }
                Entry::Int(ref i) => {
                    <u8 as Protocol>::proto_encode(&key(2, *idx), dst)?;
                    <i32 as Protocol>::proto_encode(i, dst)?;
                }
                Entry::Float(ref f) => {
                    <u8 as Protocol>::proto_encode(&key(3, *idx), dst)?;
                    <f32 as Protocol>::proto_encode(f, dst)?;
                }
                Entry::String(ref s) => {
                    <u8 as Protocol>::proto_encode(&key(4, *idx), dst)?;
                    <String as Protocol>::proto_encode(s, dst)?;
                }
                Entry::Slot(ref s) => {
                    <u8 as Protocol>::proto_encode(&key(5, *idx), dst)?;
                    <Option<Slot> as Protocol>::proto_encode(s, dst)?;
                }
                Entry::Int3(ref xyz) => {
                    <u8 as Protocol>::proto_encode(&key(6, *idx), dst)?;
                    <[i32; 3] as Protocol>::proto_encode(xyz, dst)?;
                }
                Entry::Float3(ref xyz) => {
                    <u8 as Protocol>::proto_encode(&key(7, *idx), dst)?;
                    <[f32; 3] as Protocol>::proto_encode(xyz, dst)?;
                }
            };
        }
        <u8 as Protocol>::proto_encode(&0x7f, dst)?;
        Ok(())
    }
    fn proto_decode(src: &mut dyn Read) -> io::Result<EntityMetadata> {
        let mut dict = HashMap::new();
        loop {
            let item = <u8 as Protocol>::proto_decode(src)?;
            if item == 0x7F {
                break;
            }
            let idx = item & 0x1F;
            let ty = item >> 5;
            let value = match ty {
                0 => Entry::Byte(<u8 as Protocol>::proto_decode(src)?),
                1 => Entry::Short(<i16 as Protocol>::proto_decode(src)?),
                2 => Entry::Int(<i32 as Protocol>::proto_decode(src)?),
                3 => Entry::Float(<f32 as Protocol>::proto_decode(src)?),
                4 => Entry::String(<String as Protocol>::proto_decode(src)?),
                5 => Entry::Slot(<Option<Slot> as Protocol>::proto_decode(src)?),
                6 => Entry::Int3(<[i32; 3] as Protocol>::proto_decode(src)?),
                7 => Entry::Float3(<[f32; 3] as Protocol>::proto_decode(src)?),
                ty => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        &format!("Unknown type {:x}", ty)[..],
                    ));
                }
            };
            dict.insert(idx, value);
        }
        Ok(EntityMetadata { dict })
    }
}
