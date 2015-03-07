//! Minecraft item stack (inventory slot) data type

use std::io;
use std::io::prelude::*;

use packet::Protocol;
use types::NbtFile;

#[derive(Debug)]
pub struct Slot {
    id: u16,
    count: u8,
    damage: i16,
    tag: NbtFile
}

impl Protocol for Option<Slot> {
    type Clean = Option<Slot>;

    fn proto_len(value: &Option<Slot>) -> usize {
        match *value {
            Some(ref slot) => 2 + 1 + 2 + <NbtFile as Protocol>::proto_len(&slot.tag), // id, count, damage, tag
            None => 2
        }
    }

    fn proto_encode(value: &Option<Slot>, dst: &mut Write) -> io::Result<()> {
        match *value {
            Some(Slot { id, count, damage, ref tag }) => {
                try!(<i16 as Protocol>::proto_encode(&(id as i16), dst));
                try!(<u8 as Protocol>::proto_encode(&count, dst));
                try!(<i16 as Protocol>::proto_encode(&damage, dst));
                try!(<NbtFile as Protocol>::proto_encode(tag, dst));
            }
            None => { try!(<i16 as Protocol>::proto_encode(&-1, dst)) }
        }
        Ok(())
    }

    fn proto_decode(src: &mut Read) -> io::Result<Option<Slot>> {
        let id = try!(<i16 as Protocol>::proto_decode(src));
        Ok(if id == -1 {
            None
        } else {
            Some(Slot {
                id: id as u16,
                count: try!(<u8 as Protocol>::proto_decode(src)),
                damage: try!(<i16 as Protocol>::proto_decode(src)),
                tag: try!(<NbtFile as Protocol>::proto_decode(src))
            })
        })
    }
}
