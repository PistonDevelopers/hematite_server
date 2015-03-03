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

/// Holds packet methods implemented by the `packets!` macro for all packets.
trait PacketBase {
    /// The packet ID.
    fn id(&self) -> i32;
}

/// A trait for encoding/decoding the body of a single packet type.
trait Packet: PacketBase {
    /// Encodes the packet body and writes it to a writer.
    fn encode(&self, dst: &mut Write) -> io::Result<()>;
    /// Decodes the packet body from a reader.
    fn decode(src: &mut Read, len: usize) -> io::Result<Self>;

    /// The length of the packet's fields, in bytes.
    ///
    /// The default implementation encodes the entire packet and can panic when encoding fails.
    fn len(&self) -> usize {
        let mut buf = vec![];
        self.encode(&mut buf).ok().expect("tried to get the length of a malformed packet");
        buf.len()
    }

    /// Writes a full packet to a writer, including length and packet ID.
    ///
    /// **TODO:** add support for compression.
    fn write(&self, dst: &mut Write) -> io::Result<()> {
        let len = <Var<i32> as Protocol>::proto_len(&self.id()) + self.len();
        try!(<Var<i32> as Protocol>::proto_encode(&(len as i32), dst));
        try!(<Var<i32> as Protocol>::proto_encode(&self.id(), dst));
        try!(self.encode(dst));
        Ok(())
    }
}

pub enum Direction {
    Clientbound,
    Serverbound
}

macro_rules! packet {
    // Regular packets
    ($name:ident ($id:expr) { $($fname:ident: $fty:ty),+ }) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $fname: <$fty as Protocol>::Clean),*
        }

        impl PacketBase for $name {
            #[allow(unused_variables)]
            fn id(&self) -> i32 { $id }
        }

        impl Packet for $name {
            fn len(&self) -> usize {
                0 $(+ <$fty as Protocol>::proto_len(&self.$fname) as usize)*
            }

            fn encode(&self, mut dst: &mut Write) -> io::Result<()> {
                $(try!(<$fty as Protocol>::proto_encode(&self.$fname, dst));)*
                Ok(())
            }

            #[allow(unused_variables)]
            fn decode(mut src: &mut Read, len: usize) -> io::Result<$name> {
                Ok($name {
                    $($fname: try!(<$fty as Protocol>::proto_decode(src))),*
                })
            }
        }
    };
    // No field packets
    ($name:ident ($id:expr) {}) => {
        #[derive(Debug)]
        pub struct $name;

        impl PacketBase for $name {
            #[allow(unused_variables)]
            fn id(&self) -> i32 { $id }
        }

        impl Packet for $name {
            #[allow(unused_variables)]
            fn len(&self) -> usize { 0 }

            #[allow(unused_variables)]
            fn encode(&self, dst: &mut Write) -> io::Result<()> {
                Ok(())
            }

            #[allow(unused_variables)]
            fn decode(src: &mut Read, len: usize) -> io::Result<$name> {
                Ok($name)
            }
        }
    };
    // Custom encode/decode packets
    ($name:ident ($id:expr) { $($fname:ident: $fty:ty),+; $impl_packet:item }) => {
        pub struct $name {
            $(pub $fname: $fty),*
        }

        impl PacketBase for $name {
            #[allow(unused_variables)]
            fn id(&self) -> i32 { $id }
        }

        $impl_packet
    }
}

macro_rules! packets {
    ($($state:ident => $state_mod:ident { clientbound { $($c_id:expr => $c_name:ident { $($c_packet:tt)* })* } serverbound { $($s_id:expr => $s_name:ident { $($s_packet:tt)* })* } })*) => {
        $(
            pub mod $state_mod {
                pub mod clientbound {
                    #![allow(unused_imports)]
                    use packet::{BlockChangeRecord, ExplosionOffset, Packet, PacketBase, Protocol, Stat, State};
                    use types::{Arr, Nbt, Slot, Var};

                    use std::io;
                    use std::io::prelude::*;

                    use uuid::Uuid;

                    $(packet!{ $c_name ($c_id) { $($c_packet)* } })*

                    pub enum PacketEnum {
                        $($c_name($c_name)),*
                    }
                }

                pub mod serverbound {
                    #![allow(unused_imports)]
                    use packet::{BlockChangeRecord, ExplosionOffset, Packet, PacketBase, Protocol, Stat, State};
                    use types::{Arr, Nbt, Slot, Var};

                    use std::io;
                    use std::io::prelude::*;

                    use uuid::Uuid;

                    $(packet!{ $s_name ($s_id) { $($s_packet)* } })*

                    pub enum PacketEnum {
                        $($s_name($s_name)),*
                    }
                }

                pub enum PacketEnum {
                    Clientbound(clientbound::PacketEnum),
                    Serverbound(serverbound::PacketEnum)
                }
            }
        )*

        pub enum PacketEnum {
            $($state($state_mod::PacketEnum)),*
        }
        
        #[derive(Debug)]
        pub enum State {
            $($state),*
        }

        /// Reads a new packet from a reader, wrapping in an enum for exhaustive matching.
        ///
        /// **TODO:** add support for compression.
        fn read_packet(direction: Direction, state: State, mut src: &mut Read) -> io::Result<PacketEnum> {
            let proto_len = try!(<Var<i32> as Protocol>::proto_decode(src));
            let id = try!(<Var<i32> as Protocol>::proto_decode(src));
            let len = proto_len as usize - <Var<i32> as Protocol>::proto_len(&id);
            match state {
                $(State::$state => match direction {
                    Direction::Clientbound => match id {
                        $($c_id => Ok(PacketEnum::$state($state_mod::PacketEnum::Clientbound($state_mod::clientbound::PacketEnum::$c_name(try!(<$state_mod::clientbound::$c_name as Packet>::decode(src, len)))))),)*
                        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown packet id for clientbound packet", None))
                    },
                    Direction::Serverbound => match id {
                        $($s_id => Ok(PacketEnum::$state($state_mod::PacketEnum::Serverbound($state_mod::serverbound::PacketEnum::$s_name(try!(<$state_mod::serverbound::$s_name as Packet>::decode(src, len)))))),)*
                        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown packet id for serverbound packet", None))
                    }
                }),*
            }
        }
    }
}

macro_rules! impl_protocol {
    ($name:ty, 1, $enc_name:ident, $dec_name:ident) => {
        impl Protocol for $name {
            type Clean = $name;

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
            type Clean = $name;
    
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
    ($name:ident { $($fname:ident: $fty:ty),+ }) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $fname: <$fty as Protocol>::Clean),*
        }

        impl Protocol for $name {
            type Clean = $name;

            fn proto_len(value: &$name) -> usize {
                0 $(+ <$fty as Protocol>::proto_len(&value.$fname) as usize)*
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

/// Encodes the `State` based on the values in the `Handshake` packet.
///
/// Can only encode `Status` and `Login` states, others will result in an error.
impl Protocol for State {
    type Clean = State;

    #[allow(unused_variables)]
    fn proto_len(value: &State) -> usize { 1 }

    fn proto_encode(value: &State, dst: &mut Write) -> io::Result<()> {
        let i = match *value {
            State::Status => 1,
            State::Login => 2,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid state", None))
        };
        try!(<Var<i32> as Protocol>::proto_encode(&i, dst));
        Ok(())
    }

    fn proto_decode(src: &mut Read) -> io::Result<State> {
        match try!(<Var<i32> as Protocol>::proto_decode(src)) {
            1 => Ok(State::Status),
            2 => Ok(State::Login),
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

    ExplosionOffset {
        x: i8,
        y: i8,
        z: i8
    }

    Stat {
        name: String,
        value: Var<i32>
    }
}

packets! {
    Handshaking => handshake {
        clientbound {
            0x00 => Handshake { proto_version: Var<i32>, server_address: String, server_port: u16, next_state: State }
        }
        serverbound {}
    }
    Play => play {
        clientbound {
            0x00 => KeepAlive { keep_alive_id: Var<i32> }
            0x01 => JoinGame { entity_id: i32, gamemode: u8, dimension: i8, difficulty: u8, max_players: u8, level_type: String, reduced_debug_info: bool }
            // 0x02 => ChatMessage { data: Chat, position: i8 }
            0x03 => TimeUpdate { world_age: i64, time_of_day: i64 }
            0x04 => EntityEquipment { entity_id: Var<i32>, slot: i16, item: Option<Slot> }
            0x05 => WorldSpawn { location: i64 }
            0x06 => UpdateHealth { health: f32, food: Var<i32>, saturation: f32 }
            0x07 => Respawn { dimension: i32, difficulty: u8, gamemode: u8, level_type: String }
            0x08 => PlayerPositionAndLook { x: f64, y: f64, z: f64, yaw: f32, pitch: f32, flags: i8 }
            0x09 => HeldItemChange { slot: i8 }
            0x0a => UseBed { entity_id: Var<i32>, location: i64 }
            0x0b => Animation { entity_id: Var<i32>, animation: u8 }
            // 0x0c => SpawnPlayer { entity_id: Var<i32>, player_uuid: Uuid, x: i32, y: i32, z: i32, yaw: u8, pitch: u8, current_item: i16, metadata: Metadata }
            0x0d => CollectItem { collected_eid: Var<i32>, collector_eid: Var<i32> }
            // 0x0e => SpawnObject { entity_id: Var<i32>, type_: i8, x: i32, y: i32, z: i32, pitch: u8, yaw: u8, data: ObjectData }
            // 0x0f => SpawnMob { entity_id: Var<i32>, type_: u8, x: i32, y: i32, z: i32, yaw: u8, pitch: u8, head_pitch: u8, velocity_x: i16, velocity_y: i16, velocity_z: i16, metadata: Metadata }
            0x10 => SpawnPainting { entity_id: Var<i32>, title: String, location: i64, direction: u8 }
            0x11 => SpawnExperienceOrb { entity_id: Var<i32>, x: i32, y: i32, z: i32, count: i16 }
            0x12 => EntityVelocity { entity_id: Var<i32>, velocity_x: i16, velocity_y: i16, velocity_z: i16 }
            0x13 => DestroyEntities { entity_ids: Arr<Var<i32>, Var<i32>> }
            0x14 => EntityIdle { entity_id: Var<i32> }
            0x15 => EntityRelativeMove { entity_id: Var<i32>, delta_x: i8, delta_y: i8, delta_z: i8, on_ground: bool }
            0x16 => EntityLook { entity_id: Var<i32>, yaw: u8, pitch: u8, on_ground: bool }
            0x17 => EntityLookAndRelativeMove { entity_id: Var<i32>, delta_x: i8, delta_y: i8, delta_z: i8, yaw: u8, pitch: u8, on_ground: bool }
            0x18 => EntityTeleport { entity_id: Var<i32>, x: i32, y: i32, z: i32, yaw: u8, pitch: u8, on_ground: bool }
            0x19 => EntityHeadLook { entity_id: Var<i32>, head_yaw: u8 }
            0x1A => EntityStatus { entity_id: i32, entity_status: i8 }
            0x1B => AttachEntity { riding_eid: i32, vehicle_eid: i32, leash: bool }
            // 0x1C => EntityMetadata { entity_id: Var<i32>, metadata: Metadata }
            0x1D => EntityEffect { entity_id: Var<i32>, effect_id: i8, amplifier: i8, duration: Var<i32>, hide_particles: bool }
            0x1E => RemoveEntityEffect { entity_id: Var<i32>, effect_id: i8 }
            0x1F => SetExperience { xp_bar: f32, level: Var<i32>, xp_total: Var<i32> }
            // 0x20 => EntityProperties { entity_id: Var<i32>, properties: Arr<i32, Property> }
            // 0x21 => ChunkData { chunk_x: i32, chunk_z: i32, ground_up_continuous: bool, mask: u16, chunk_data: Chunk; impl Packet for ChunkData { ... } } // chunk_data is length-prefixed and may or may not represent an entire chunk column
            0x22 => MultiBlockChange { chunk_x: i32, chunk_z: i32, records: Arr<Var<i32>, BlockChangeRecord> }
            0x23 => BlockChange { location: i64, block_id: Var<i32> }
            0x24 => BlockAction { location: i64, byte1: u8, byte2: u8, block_type: Var<i32> }
            0x25 => BlockBreakAnimation { entity_id: Var<i32>, location: i64, destroy_stage: i8 }
            // 0x26 => MapChunkBulk { sky_light_sent: bool, chunks: Vec<Chunk>; impl Packet for MapChunkBulk { ... } } // PROBLEM: chunks is encoded as two arrays, the first one specifying which sections of each chunk column are empty
            0x27 => Explosion { x: f32, y: f32, z: f32, radius: f32, records: Arr<i32, ExplosionOffset>, player_motion_x: f32, player_motion_y: f32, player_motion_z: f32 }
            0x28 => Effect { effect_id: i32, location: i64, data: i32, disable_relative_volume: bool }
            0x29 => SoundEffect { name: String, x: i32, y: i32, z: i32, volume: f32, pitch: u8 }
            // 0x2a => Particle { particle_id: i32, long_distance: bool, x: f32, y: f32, z: f32, offset_x: f32, offset_y: f32, offset_z: f32, particle_data: f32, particle_count: i32, data: Vec<i32>; impl Packet for Particle { ... } } // PROBLEM: length of data depends on particle_id
            0x2b => ChangeGameState { reason: u8, value: f32 }
            0x2c => SpawnGlobalEntity { entity_id: Var<i32>, type_: i8, x: i32, y: i32, z: i32 }
            // 0x2d => OpenWindow { window_id: u8, window_type: String, window_title: Chat, slots: u8, entity_id: Option<i32>; impl Packet for OpenWindow { ... } } // PROBLEM: entity_id depends on window_type
            0x2e => CloseWindow { window_id: u8 }
            0x2f => SetSlot { window_id: u8, slot: i16, data: Option<Slot> }
            0x30 => WindowItems { window_id: u8, slots: Arr<i16, Option<Slot>> }
            0x31 => WindowProperty { window_id: u8, property: i16, value: i16 }
            0x32 => ConfirmTransaction { window_id: u8, action_number: i16, accepted: bool }
            // 0x33 => UpdateSign { location: i64, line0: Chat, line1: Chat, line2: Chat, line3: Chat }
            // 0x34 => UpdateMap { map_id: Var<i32>, scale: i8, icons: Arr<Var<i32>, MapIcon>, data: MapData } // MapData is a quirky format holding optional pixel data for an arbitrary rectangle on the map
            // 0x35 => UpdateBlockEntity { location: i64, action: u8, nbt_data: Nbt; impl Packet for UpdateBlockEntity { ... } } // PROBLEM: nbt_data is omitted entirely if it encodes an empty NBT tag
            0x36 => SignEditorOpen { location: i64 }
            0x37 => Statistics { stats: Arr<Var<i32>, Stat> }
            // 0x38 => UpdatePlayerList { action: Var<i32>, players: Arr<Var<i32>, PlayerListItem>; impl Packet for UpdatePlayerList { ... } } // PROBLEM: suructure of `players` elements depends on `action`
            0x39 => PlayerAbilities { flags: i8, flying_speed: f32, walking_speed: f32 }
            0x3a => TabComplete { matches: Arr<Var<i32>, String> }
            // 0x3b => ScoreboardObjective { objective_name: String, mode: ObjectiveAction }
            // 0x3c => UpdateScore { score_name: String, action: ScoreAction }
            0x3d => DisplayScoreboard { position: i8, score_name: String }
            // 0x3e => UpdateTeam { team_name: String, action: TeamAction }
            // 0x3f => PluginMessage { channel: String, data: Vec<u8>; impl Packet for PluginMessage { ... } } // PROBLEM: length of `data` comes from packet length
            // 0x40 => Disconnect { reason: Chat }
            0x41 => ServerDifficulty { difficulty: u8 }
            // 0x42 => PlayCombatEvent { event: CombatEvent }
            0x43 => Camera { camera_id: Var<i32> }
            // 0x44 => WorldBorder { action: WorldBorderAction }
            // 0x45 => Title { action: TitleAction }
            0x46 => SetCompression { threshold: Var<i32> }
            // 0x47 => PlayerListHeaderFooter { header: Chat, footer: Chat }
            0x48 => ResourcePackSend { url: String, hash: String }
            0x49 => UpdateEntityNbt { entity_id: Var<i32>, tag: Nbt }
        }
        serverbound {
            0x00 => KeepAlive { keep_alive_id: i32 }
            0x01 => ChatMessage { message: String }
            // 0x02 => UseEntity { target_eid: i32, use_type: EntityUseAction }
            0x03 => PlayerIdle { on_ground: bool }
            0x04 => PlayerPosition { x: f64, y: f64, z: f64, on_ground: bool }
            0x05 => PlayerLook { yaw: f32, pitch: f32, on_ground: bool }
            0x06 => PlayerPositionAndLook { x: f64, y: f64, z: f64, yaw: f32, pitch: f32, on_ground: bool }
            0x07 => PlayerDigging { status: i8, location: i64, face: i8 }
            0x08 => PlayerBlockPlacement { location: i64, direction: i8, held_item: Option<Slot>, cursor_x: i8, cursor_y: i8, cursor_z: i8 }
            0x09 => HeldItemChange { slot: i16 }
            0x0a => Animation {}
            0x0b => EntityAction { entity_id: Var<i32>, action_id: Var<i32>, jump_boost: Var<i32> }
            0x0c => SteerVehicle { sideways: f32, forward: f32, flags: u8 }
            0x0d => CloseWindow { window_id: u8 }
            0x0e => ClickWindow { window_id: u8, slot: i16, button: i8, action_number: i16, mode: i8, clicked_item: Option<Slot> }
            0x0f => ConfirmTransaction { window_id: u8, action_number: i16, accepted: bool }
            0x10 => CreativeInventoryAction { slot: i16, clicked_item: Option<Slot> }
            0x11 => EnchantItem { window_id: u8, enchantment: i8 }
            // 0x12 => UpdateSign { location: i64, line0: Chat, line1: Chat, line2: Chat, line3: Chat }
            0x13 => PlayerAbilities { flags: i8, flying_speed: f32, walking_speed: f32 }
            0x14 => TabComplete { text: String, looking_at: Option<i64> }
            0x15 => ClientSettings { locale: String, view_distance: i8, chat_mode: i8, chat_colors: bool, displayed_skin_parts: u8 }
            0x16 => ClientStatus { action_id: Var<i32> }
            // 0x17 => PluginMessage { channel: String, data: Vec<u8>; impl Packet for PluginMessage { ... } } // PROBLEM: length of `data` comes from packet length
            0x18 => Spectate { target_player: Uuid }
            0x19 => ResourcePackStatus { hash: String, result: Var<i32> }
        }
    }
    Status => status {
        clientbound {
            0x00 => StatusResponse { response: String }
            0x01 => Pong { time: i64 }
        }
        serverbound {
            0x00 => StatusRequest {}
            0x01 => Ping { time: i64 }
        }
    }
    Login => login {
        clientbound {
            // 0x00 => Disconnect { reason: Chat }
            0x01 => EncryptionRequest { server_id: String, pubkey: Arr<Var<i32>, u8>, verify_token: Arr<Var<i32>, u8> }
            // 0x02 => LoginSuccess { uuid: Uuid, username: String; impl Packet for LoginSuccess { ... } } // NOTE: uuid field is encoded as a string!
            0x03 => SetCompression { threshold: Var<i32> }
        }
        serverbound {
            0x00 => LoginStart { name: String }
            0x01 => EncryptionResponse { shared_secret: Arr<Var<i32>, u8>, verify_token: Arr<Var<i32>, u8> }
        }
    }
}
