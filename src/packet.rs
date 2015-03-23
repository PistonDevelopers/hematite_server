//! MC Protocol packets

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use std::error::FromError;
use std::io;
use std::io::prelude::*;

use types::Var;

/// A trait used for data which can be encoded/decoded as is.
pub trait Protocol {
    type Clean = Self;

    fn proto_len(value: &Self::Clean) -> usize;
    fn proto_encode(value: &Self::Clean, dst: &mut Write) -> io::Result<()>;
    fn proto_decode(src: &mut Read) -> io::Result<Self::Clean>;
}

/// A trait for encoding the body of a single packet type.
pub trait PacketWrite {
    fn inner_len(&self) -> usize;
    fn inner_encode(&self, dst: &mut Write) -> io::Result<()>;

    /// Writes a full packet to a writer, including length.
    ///
    /// **TODO:** add support for compression.
    fn write(&self, dst: &mut Write) -> io::Result<()> {
        let len = self.inner_len();
        try!(<Var<i32> as Protocol>::proto_encode(&(len as i32), dst));
        self.inner_encode(dst)
    }
}

/// A trait for decoding any of the packet types in one ID namespace.
pub trait PacketRead: Sized {
    fn inner_decode(src: &mut Read) -> io::Result<Self>;

    /// Reads a new packet from a reader, including length.
    ///
    /// **TODO:** add support for compression.
    fn read<R: Read>(src: &mut R) -> io::Result<Self> {
        let proto_len = try!(<Var<i32> as Protocol>::proto_decode(src));
        Self::inner_decode(&mut src.take(proto_len as u64))
    }
}

#[derive(Debug)]
pub enum Direction {
    Clientbound,
    Serverbound
}

#[derive(Debug)]
pub enum NextState {
    Status,
    Login
}

mod prelude {
    pub use packet::{BlockChangeRecord, ChunkMeta, Protocol, PacketRead, PacketWrite, Stat, NextState};
    pub use proto::slp;
    pub use types::consts::*;
    pub use types::{Arr, BlockPos, ChunkColumn, NbtBlob, Slot, UuidString, Var};

    pub use std::io;
    pub use std::io::prelude::*;

    pub use uuid::Uuid;
}

macro_rules! packets {
    ($($id:expr => $name:ident { $($packet:tt)* })*) => {
        use packet::prelude::*;

        $(proto_struct!{ $name { $($packet)* } })*

        #[derive(Debug)]
        pub enum Packet {
            $($name($name)),*
        }

        impl PacketRead for Packet {
            fn inner_decode(src: &mut Read) -> io::Result<Self> {
                match try!(<Var<i32> as Protocol>::proto_decode(src)) {
                    $($id => <$name as Protocol>::proto_decode(src).map(Packet::$name),)*
                    _ => Err(io::Error::new(io::ErrorKind::InvalidInput,
                                             "unknown packet id", None))
                }
            }
        }

        $(impl PacketWrite for $name {
            fn inner_len(&self) -> usize {
                let id_len = <Var<i32> as Protocol>::proto_len(&$id);
                id_len + <Self as Protocol>::proto_len(self)
            }

            fn inner_encode(&self, dst: &mut Write) -> io::Result<()> {
                try!(<Var<i32> as Protocol>::proto_encode(&$id, dst));
                <Self as Protocol>::proto_encode(self, dst)
            }
        })*
    }
}

macro_rules! impl_protocol {
    ($name:ty, 1, $enc_name:ident, $dec_name:ident) => {
        impl Protocol for $name {
            type Clean = Self;

            #[allow(unused_variables)]
            fn proto_len(value: &$name) -> usize { 1 }

            fn proto_encode(value: &$name, mut dst: &mut Write) -> io::Result<()> {
                try!(dst.$enc_name(*value));
                Ok(())
            }

            fn proto_decode(mut src: &mut Read) -> io::Result<$name> {
                src.$dec_name().map_err(|err| FromError::from_error(err))
            }
        }
    };
    ($name:ty, $len:expr, $enc_name:ident, $dec_name:ident) => {
        impl Protocol for $name {
            type Clean = Self;

            #[allow(unused_variables)]
            fn proto_len(value: &$name) -> usize { $len }

            fn proto_encode(value: &$name, mut dst: &mut Write) -> io::Result<()> {
                try!(dst.$enc_name::<BigEndian>(*value));
                Ok(())
            }

            fn proto_decode(mut src: &mut Read) -> io::Result<$name> {
                src.$dec_name::<BigEndian>().map_err(|err| FromError::from_error(err))
            }
        }
    }
}

macro_rules! proto_struct {
    // Regular structs.
    ($name:ident { $($fname:ident: $fty:ty),+ }) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $fname: <$fty as Protocol>::Clean),*
        }

        impl Protocol for $name {
            type Clean = Self;

            fn proto_len(value: &$name) -> usize {
                0 $(+ <$fty as Protocol>::proto_len(&value.$fname))*
            }

            fn proto_encode(value: &$name, dst: &mut Write) -> io::Result<()> {
                $(try!(<$fty as Protocol>::proto_encode(&value.$fname, dst));)*
                Ok(())
            }

            fn proto_decode(mut src: &mut Read) -> io::Result<$name> {
                Ok($name {
                    $($fname: try!(<$fty as Protocol>::proto_decode(src))),*
                })
            }
        }
    };
    // No field structs (unit values).
    ($name:ident {}) => {
        #[derive(Debug)]
        pub struct $name;

        impl Protocol for $name {
            type Clean = Self;

            fn proto_len(_: &Self) -> usize { 0 }

            fn proto_encode(_: &Self, _: &mut Write) -> io::Result<()> {
                Ok(())
            }

            fn proto_decode(_: &mut Read) -> io::Result<$name> {
                Ok($name)
            }
        }
    };
    // Custom encode/decode structs.
    ($name:ident { $($fname:ident: $fty:ty),+; $impl_struct:item }) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $fname: $fty),*
        }

        $impl_struct
    }
}

macro_rules! proto_structs {
    ($($name:ident { $($fields:tt)+ })+) => {
        $(proto_struct!($name { $($fields)* });)*
    }
}

impl_protocol!(i8,  1, write_i8,  read_i8);
impl_protocol!(u8,  1, write_u8,  read_u8);
impl_protocol!(i16, 2, write_i16, read_i16);
impl_protocol!(u16, 2, write_u16, read_u16);
impl_protocol!(i32, 4, write_i32, read_i32);
impl_protocol!(u32, 4, write_u32, read_u32);
impl_protocol!(i64, 8, write_i64, read_i64);
impl_protocol!(u64, 8, write_u64, read_u64);
impl_protocol!(f32, 4, write_f32, read_f32);
impl_protocol!(f64, 8, write_f64, read_f64);

impl Protocol for bool {
    type Clean = bool;

    #[allow(unused_variables)]
    fn proto_len(value: &bool) -> usize { 1 }

    fn proto_encode(value: &bool, mut dst: &mut Write) -> io::Result<()> {
        try!(dst.write_u8(if *value { 1 } else { 0 }));
        Ok(())
    }

    fn proto_decode(mut src: &mut Read) -> io::Result<bool> {
        let value = try!(src.read_u8());
        if value > 1 {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid bool value", Some(format!("Invalid bool value, expecting 0 or 1, got {}", value))))
        } else {
            Ok(value == 1)
        }
    }
}

/// Optional values encoded as a bool-prefixed value.
impl<T: Protocol> Protocol for Option<T> {
    type Clean = Option<T::Clean>;

    fn proto_len(value: &Option<T::Clean>) -> usize {
        match *value {
            Some(ref inner) => 1 + <T as Protocol>::proto_len(inner),
            None => 1
        }
    }

    fn proto_encode(value: &Option<T::Clean>, dst: &mut Write) -> io::Result<()> {
        match *value {
            Some(ref inner) => {
                try!(<bool as Protocol>::proto_encode(&true, dst));
                try!(<T as Protocol>::proto_encode(inner, dst));
            }
            None => {
                try!(<bool as Protocol>::proto_encode(&false, dst));
            }
        }
        Ok(())
    }

    fn proto_decode(src: &mut Read) -> io::Result<Option<T::Clean>> {
        if try!(<bool as Protocol>::proto_decode(src)) {
            Ok(Some(try!(<T as Protocol>::proto_decode(src))))
        } else {
            Ok(None)
        }
    }
}

/// Encodes the `NextState` based on the values in the `Handshake` packet.
///
/// Only intended for encoding `Status` and `Login` states.
impl Protocol for NextState {
    type Clean = Self;

    fn proto_len(_: &Self) -> usize { 1 }

    fn proto_encode(value: &Self, dst: &mut Write) -> io::Result<()> {
        let i = match *value {
            NextState::Status => 1,
            NextState::Login => 2
        };
        <Var<i32> as Protocol>::proto_encode(&i, dst)
    }

    fn proto_decode(src: &mut Read) -> io::Result<Self> {
        match try!(<Var<i32> as Protocol>::proto_decode(src)) {
            1 => Ok(NextState::Status),
            2 => Ok(NextState::Login),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid state", None))
        }
    }
}

proto_structs! {
    BlockChangeRecord {
        xz: u8,
        y: u8,
        block_id: Var<i32>
    }

    ChunkMeta {
        x: i32,
        z: i32,
        mask: u16
    }

    Stat {
        name: String,
        value: Var<i32>
    }
}

pub mod handshake {
    packets! {
        0x00 => Handshake { proto_version: Var<i32>, server_address: String, server_port: u16, next_state: NextState }
    }
}
pub mod play {
    pub mod clientbound { packets! {
        0x00 => KeepAlive { keep_alive_id: Var<i32> }
        0x01 => JoinGame { entity_id: i32, gamemode: u8, dimension: Dimension, difficulty: u8, max_players: u8, level_type: String, reduced_debug_info: bool }
        // 0x02 => ChatMessage { data: Chat, position: i8 }
        0x03 => TimeUpdate { world_age: i64, time_of_day: i64 }
        0x04 => EntityEquipment { entity_id: Var<i32>, slot: i16, item: Option<Slot> }
        0x05 => WorldSpawn { location: BlockPos }
        0x06 => UpdateHealth { health: f32, food: Var<i32>, saturation: f32 }
        0x07 => Respawn { dimension: Dimension, difficulty: u8, gamemode: u8, level_type: String }
        0x08 => PlayerPositionAndLook { position: [f64; 3], yaw: f32, pitch: f32, flags: i8 }
        0x09 => HeldItemChange { slot: i8 }
        0x0a => UseBed { entity_id: Var<i32>, location: BlockPos }
        0x0b => Animation { entity_id: Var<i32>, animation: u8 }
        // 0x0c => SpawnPlayer { entity_id: Var<i32>, player_uuid: Uuid, position: [i32; 3], yaw: u8, pitch: u8, current_item: i16, metadata: Metadata }
        0x0d => CollectItem { collected_eid: Var<i32>, collector_eid: Var<i32> }
        // 0x0e => SpawnObject { entity_id: Var<i32>, type_: i8, position: [i32; 3], pitch: u8, yaw: u8, data: ObjectData }
        // 0x0f => SpawnMob { entity_id: Var<i32>, type_: u8, position: [i32; 3], yaw: u8, pitch: u8, head_pitch: u8, velocity: [i16; 3], metadata: Metadata }
        0x10 => SpawnPainting { entity_id: Var<i32>, title: String, location: BlockPos, direction: u8 }
        0x11 => SpawnExperienceOrb { entity_id: Var<i32>, position: [i32; 3], count: i16 }
        0x12 => EntityVelocity { entity_id: Var<i32>, velocity: [i16; 3] }
        0x13 => DestroyEntities { entity_ids: Arr<Var<i32>, Var<i32>> }
        0x14 => EntityIdle { entity_id: Var<i32> }
        0x15 => EntityRelativeMove { entity_id: Var<i32>, delta: [i8; 3], on_ground: bool }
        0x16 => EntityLook { entity_id: Var<i32>, yaw: u8, pitch: u8, on_ground: bool }
        0x17 => EntityLookAndRelativeMove { entity_id: Var<i32>, delta: [i8; 3], yaw: u8, pitch: u8, on_ground: bool }
        0x18 => EntityTeleport { entity_id: Var<i32>, position: [i32; 3], yaw: u8, pitch: u8, on_ground: bool }
        0x19 => EntityHeadLook { entity_id: Var<i32>, head_yaw: u8 }
        0x1A => EntityStatus { entity_id: i32, entity_status: i8 }
        0x1B => AttachEntity { riding_eid: i32, vehicle_eid: i32, leash: bool }
        // 0x1C => EntityMetadata { entity_id: Var<i32>, metadata: Metadata }
        0x1D => EntityEffect { entity_id: Var<i32>, effect_id: i8, amplifier: i8, duration: Var<i32>, hide_particles: bool }
        0x1E => RemoveEntityEffect { entity_id: Var<i32>, effect_id: i8 }
        0x1F => SetExperience { xp_bar: f32, level: Var<i32>, xp_total: Var<i32> }
        // 0x20 => EntityProperties { entity_id: Var<i32>, properties: Arr<i32, Property> }
        0x21 => ChunkData { x: i32, z: i32, continuous: bool, mask: u16, chunk_data: Arr<Var<i32>, u8> }
        0x22 => MultiBlockChange { chunk_x: i32, chunk_z: i32, records: Arr<Var<i32>, BlockChangeRecord> }
        0x23 => BlockChange { location: BlockPos, block_id: Var<i32> }
        0x24 => BlockAction { location: BlockPos, byte1: u8, byte2: u8, block_type: Var<i32> }
        0x25 => BlockBreakAnimation { entity_id: Var<i32>, location: BlockPos, destroy_stage: i8 }
        0x26 => ChunkDataBulk { sky_light_sent: bool, chunk_meta: Vec<ChunkMeta>, chunk_data: Vec<ChunkColumn>;
            impl Protocol for ChunkDataBulk {
                type Clean = Self;
                fn proto_len(this: &Self) -> usize {
                    use std::iter::AdditiveIterator;

                    let columns = this.chunk_meta.len() as i32;
                    1 // sky_light_sent(bool) len is constant
                    + <Var<i32> as Protocol>::proto_len(&columns)
                    + this.chunk_meta.iter().map(|cm| <ChunkMeta as Protocol>::proto_len(cm)).sum()
                    + this.chunk_data.iter().map(|cd| cd.len()).sum()
                }
                fn proto_encode(this: &Self, mut dst: &mut Write) -> io::Result<()> {
                    try!(<bool as Protocol>::proto_encode(&this.sky_light_sent, dst));
                    let columns = this.chunk_meta.len() as i32;
                    try!(<Var<i32> as Protocol>::proto_encode(&columns, dst));
                    for cm in &this.chunk_meta {
                        try!(<ChunkMeta as Protocol>::proto_encode(cm, dst));
                    }
                    for cd in &this.chunk_data {
                        let chunk_column = try!(cd.encode());
                        try!(dst.write_all(&chunk_column));
                    }
                    Ok(())
                }
                fn proto_decode(mut src: &mut Read) -> io::Result<ChunkDataBulk> {
                    let sky_light_sent = try!(<bool as Protocol>::proto_decode(src));
                    let columns = try!(<Var<i32> as Protocol>::proto_decode(src));
                    let mut chunk_meta = Vec::with_capacity(columns as usize);
                    for cm in &mut chunk_meta {
                        *cm = try!(<ChunkMeta as Protocol>::proto_decode(src));
                    }
                    // Read all encoded ChunkColumns, buffer size starts at 4KB, probably will get bigger
                    let mut data = Vec::with_capacity(1 << 12);
                    try!(src.read_to_end(&mut data));
                    let mut src = io::Cursor::new(data);
                    let mut chunk_data = Vec::with_capacity(columns as usize);
                    for (cd, cm) in chunk_data.iter_mut().zip(chunk_meta.iter()) {
                        // chunk_data, mask, continuous, sky_light
                        *cd = try!(ChunkColumn::decode(&mut src, cm.mask, true, true));
                    }
                    Ok(ChunkDataBulk{
                        sky_light_sent: sky_light_sent,
                        chunk_meta: chunk_meta,
                        chunk_data: chunk_data,
                    })
                }
            }
        }
        0x27 => Explosion { position: [f32; 3], radius: f32, records: Arr<i32, [i8; 3]>, player_motion: [f32; 3] }
        0x28 => Effect { effect_id: i32, location: BlockPos, data: i32, disable_relative_volume: bool }
        0x29 => SoundEffect { name: String, position: [i32; 3], volume: f32, pitch: u8 }
        // 0x2a => Particle { particle_id: i32, long_distance: bool, position: [f32; 3], offset: [f32; 3], particle_data: f32, particle_count: i32, data: Vec<i32>; impl Protocol for Particle { ... } } // PROBLEM: length of data depends on particle_id
        0x2b => ChangeGameState { reason: u8, value: f32 }
        0x2c => SpawnGlobalEntity { entity_id: Var<i32>, type_: i8, position: [i32; 3] }
        // 0x2d => OpenWindow { window_id: u8, window_type: String, window_title: Chat, slots: u8, entity_id: Option<i32>; impl Protocol for OpenWindow { ... } } // PROBLEM: entity_id depends on window_type
        0x2e => CloseWindow { window_id: u8 }
        0x2f => SetSlot { window_id: u8, slot: i16, data: Option<Slot> }
        0x30 => WindowItems { window_id: u8, slots: Arr<i16, Option<Slot>> }
        0x31 => WindowProperty { window_id: u8, property: i16, value: i16 }
        0x32 => ConfirmTransaction { window_id: u8, action_number: i16, accepted: bool }
        // 0x33 => UpdateSign { location: BlockPos, line0: Chat, line1: Chat, line2: Chat, line3: Chat }
        // 0x34 => UpdateMap { map_id: Var<i32>, scale: i8, icons: Arr<Var<i32>, MapIcon>, data: MapData } // MapData is a quirky format holding optional pixel data for an arbitrary rectangle on the map
        // 0x35 => UpdateBlockEntity { location: [i32; 3], action: u8, nbt_data: Nbt; impl Protocol for UpdateBlockEntity { ... } } // PROBLEM: nbt_data is omitted entirely if it encodes an empty NBT tag
        0x36 => SignEditorOpen { location: BlockPos }
        0x37 => Statistics { stats: Arr<Var<i32>, Stat> }
        // 0x38 => UpdatePlayerList { action: Var<i32>, players: Arr<Var<i32>, PlayerListItem>; impl Protocol for UpdatePlayerList { ... } } // PROBLEM: suructure of `players` elements depends on `action`
        0x39 => PlayerAbilities { flags: i8, flying_speed: f32, walking_speed: f32 }
        0x3a => TabComplete { matches: Arr<Var<i32>, String> }
        // 0x3b => ScoreboardObjective { objective_name: String, mode: ObjectiveAction }
        // 0x3c => UpdateScore { score_name: String, action: ScoreAction }
        0x3d => DisplayScoreboard { position: i8, score_name: String }
        // 0x3e => UpdateTeam { team_name: String, action: TeamAction }
        0x3f => PluginMessage { channel: String, data: Vec<u8>;
            impl Protocol for PluginMessage {
                type Clean = Self;
                fn proto_len(this: &Self) -> usize {
                    <String as Protocol>::proto_len(&this.channel) + this.data.len()
                }
                fn proto_encode(this: &Self, mut dst: &mut Write) -> io::Result<()> {
                    try!(<String as Protocol>::proto_encode(&this.channel, dst));
                    try!(dst.write_all(&this.data));
                    Ok(())
                }
                fn proto_decode(mut src: &mut Read) -> io::Result<PluginMessage> {
                    Ok(PluginMessage{
                        channel: try!(<String as Protocol>::proto_decode(src)),
                        data:  { let mut data = vec![]; try!(src.read_to_end(&mut data)); data },
                    })
                }
            }
        }
        // 0x40 => Disconnect { reason: Chat }
        0x41 => ServerDifficulty { difficulty: u8 }
        // 0x42 => PlayCombatEvent { event: CombatEvent }
        0x43 => Camera { camera_id: Var<i32> }
        // 0x44 => WorldBorder { action: WorldBorderAction }
        // 0x45 => Title { action: TitleAction }
        0x46 => SetCompression { threshold: Var<i32> }
        // 0x47 => PlayerListHeaderFooter { header: Chat, footer: Chat }
        0x48 => ResourcePackSend { url: String, hash: String }
        0x49 => UpdateEntityNbt { entity_id: Var<i32>, tag: NbtBlob }
    } }
    pub mod serverbound { packets! {
        0x00 => KeepAlive { keep_alive_id: i32 }
        0x01 => ChatMessage { message: String }
        // 0x02 => UseEntity { target_eid: i32, use_type: EntityUseAction }
        0x03 => PlayerIdle { on_ground: bool }
        0x04 => PlayerPosition { position: [f64; 3], on_ground: bool }
        0x05 => PlayerLook { yaw: f32, pitch: f32, on_ground: bool }
        0x06 => PlayerPositionAndLook { position: [f64; 3], yaw: f32, pitch: f32, on_ground: bool }
        0x07 => PlayerDigging { status: i8, location: BlockPos, face: i8 }
        0x08 => PlayerBlockPlacement { location: BlockPos, direction: i8, held_item: Option<Slot>, cursor: [i8; 3] }
        0x09 => HeldItemChange { slot: i16 }
        0x0a => Animation {}
        0x0b => EntityAction { entity_id: Var<i32>, action_id: Var<i32>, jump_boost: Var<i32> }
        0x0c => SteerVehicle { sideways: f32, forward: f32, flags: u8 }
        0x0d => CloseWindow { window_id: u8 }
        0x0e => ClickWindow { window_id: u8, slot: i16, button: i8, action_number: i16, mode: i8, clicked_item: Option<Slot> }
        0x0f => ConfirmTransaction { window_id: u8, action_number: i16, accepted: bool }
        0x10 => CreativeInventoryAction { slot: i16, clicked_item: Option<Slot> }
        0x11 => EnchantItem { window_id: u8, enchantment: i8 }
        // 0x12 => UpdateSign { location: BlockPos, line0: Chat, line1: Chat, line2: Chat, line3: Chat }
        0x13 => PlayerAbilities { flags: i8, flying_speed: f32, walking_speed: f32 }
        0x14 => TabComplete { text: String, looking_at: Option<i64> }
        0x15 => ClientSettings { locale: String, view_distance: i8, chat_mode: i8, chat_colors: bool, displayed_skin_parts: u8 }
        0x16 => ClientStatus { action_id: Var<i32> }
        0x17 => PluginMessage { channel: String, data: Vec<u8>;
            impl Protocol for PluginMessage {
                type Clean = Self;
                fn proto_len(this: &Self) -> usize {
                    <String as Protocol>::proto_len(&this.channel) + this.data.len()
                }
                fn proto_encode(this: &Self, mut dst: &mut Write) -> io::Result<()> {
                    try!(<String as Protocol>::proto_encode(&this.channel, dst));
                    try!(dst.write_all(&this.data));
                    Ok(())
                }
                fn proto_decode(mut src: &mut Read) -> io::Result<PluginMessage> {
                    Ok(PluginMessage{
                        channel: try!(<String as Protocol>::proto_decode(src)),
                        data: { let mut data = vec![]; try!(src.read_to_end(&mut data)); data },
                    })
                }
            }
        }
        0x18 => Spectate { target_player: Uuid }
        0x19 => ResourcePackStatus { hash: String, result: Var<i32> }
    } }
}
pub mod status {
    pub mod clientbound { packets! {
        0x00 => StatusResponse { response: slp::Response }
        0x01 => Pong { time: i64 }
    } }
    pub mod serverbound { packets! {
        0x00 => StatusRequest {}
        0x01 => Ping { time: i64 }
    } }
}
pub mod login {
    pub mod clientbound { packets! {
        // 0x00 => Disconnect { reason: Chat }
        0x01 => EncryptionRequest { server_id: String, pubkey: Arr<Var<i32>, u8>, verify_token: Arr<Var<i32>, u8> }
        0x02 => LoginSuccess { uuid: UuidString, username: String }
        0x03 => SetCompression { threshold: Var<i32> }
    } }
    pub mod serverbound { packets! {
        0x00 => LoginStart { name: String }
        0x01 => EncryptionResponse { shared_secret: Arr<Var<i32>, u8>, verify_token: Arr<Var<i32>, u8> }
    } }
}
