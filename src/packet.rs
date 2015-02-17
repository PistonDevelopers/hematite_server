//! MC Protocol packets

use std::old_io::{ IoError, IoErrorKind, IoResult };

use types::VarInt;

use uuid::Uuid;

pub trait Protocol {
    type Clean = Self;
    fn proto_len(value: &Self::Clean) -> usize;
    fn proto_encode(value: Self::Clean, dst: &mut Writer) -> IoResult<()>;
    fn proto_decode(src: &mut Reader, plen: usize) -> IoResult<Self::Clean>;
}

macro_rules! packet {
    // Regular packets
    ($name:ident ($id:expr) { $($fname:ident: $fty:ty),+ }) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $fname: <$fty as Protocol>::Clean),*
        }

        impl Protocol for $name {
            type Clean = $name;
            fn proto_len(value: &$name) -> usize {
                0 $(+ <$fty as Protocol>::proto_len(&value.$fname) as usize)*
            }
            fn proto_encode(value: $name, dst: &mut Writer) -> IoResult<()> {
                let len = <VarInt as Protocol>::proto_len(&$id) + <$name as Protocol>::proto_len(&value);
                try!(<VarInt as Protocol>::proto_encode(len as i32, dst));
                try!(<VarInt as Protocol>::proto_encode($id, dst));
                $(try!(<$fty as Protocol>::proto_encode(value.$fname, dst));)*
                // println!("proto_encode name={} id={:x} length={}", stringify!($name), $id, len);
                Ok(())
            }
            #[allow(unused_variables)]
            fn proto_decode(src: &mut Reader, plen: usize) -> IoResult<$name> {
                let len: i32 = try!(<VarInt as Protocol>::proto_decode(src, plen));
                let id: i32 = try!(<VarInt as Protocol>::proto_decode(src, plen));
                // println!("proto_decode name={} id={:x} length={}", stringify!($name), id, plen);
                if id != $id {
                    return Err(IoError {
                        kind: IoErrorKind::InvalidInput,
                        desc: "unexpected packet",
                        detail: Some(format!("Expected packet id #{:x}, got #{:x} instead.", $id, id))
                    });
                }
                Ok($name {
                    $($fname: try!(<$fty as Protocol>::proto_decode(src, plen))),*
                })
            }
        }
    };
    // No field packets
    ($name:ident ($id:expr) {}) => {
        #[derive(Debug)]
        pub struct $name;

        impl Protocol for $name {
            type Clean = $name;
            #[allow(unused_variables)]
            fn proto_len(value: &$name) -> usize { 0 }
            fn proto_encode(value: $name, dst: &mut Writer) -> IoResult<()> {
                let len = 1 + <$name as Protocol>::proto_len(&value);
                try!(<VarInt as Protocol>::proto_encode(len as i32, dst));
                try!(<VarInt as Protocol>::proto_encode($id, dst));
                // println!("proto_encode name={} id={:x} length={}", stringify!($name), $id, len);
                Ok(())
            }
            #[allow(unused_variables)]
            fn proto_decode(src: &mut Reader, plen: usize) -> IoResult<$name> {
                let len: i32 = try!(<VarInt as Protocol>::proto_decode(src, plen));
                let id: i32 = try!(<VarInt as Protocol>::proto_decode(src, plen));
                // println!("proto_decode name={} id={:x} length={}", stringify!($name), id, plen);
                if id != $id {
                    return Err(IoError {
                        kind: IoErrorKind::InvalidInput,
                        desc: "unexpected packet",
                        detail: Some(format!("Expected packet id #{:x}, got #{:x} instead.", $id, id))
                    });
                }
                Ok($name)
            }
        }
    }
}

macro_rules! packets {
    ($($id:expr => $name:ident {$($packet:tt)*})*) => {
        $(packet!{ $name ($id) { $($packet)* } })*
    }
}

macro_rules! impl_protocol {
    ($name:ty, $len:expr, $enc_name:ident, $dec_name:ident) => {
        impl Protocol for $name {
            type Clean = $name;
            #[allow(unused_variables)]
            fn proto_len(value: &$name) -> usize { $len }
            fn proto_encode(value: $name, dst: &mut Writer) -> IoResult<()> {
                try!(dst.$enc_name(value));
                Ok(())
            }
            #[allow(unused_variables)]
            fn proto_decode(src: &mut Reader, plen: usize) -> IoResult<$name> {
                Ok(try!(src.$dec_name()))
            }
        }
    }
}

impl_protocol!(i8,  1, write_i8,     read_i8);
impl_protocol!(u8,  1, write_u8,     read_u8);
impl_protocol!(i16, 2, write_be_i16, read_be_i16);
impl_protocol!(u16, 2, write_be_u16, read_be_u16);
impl_protocol!(i32, 4, write_be_i32, read_be_i32);
impl_protocol!(u32, 4, write_be_u32, read_be_u32);
impl_protocol!(i64, 8, write_be_i64, read_be_i64);
impl_protocol!(u64, 8, write_be_u64, read_be_u64);
impl_protocol!(f32, 4, write_be_f32, read_be_f32);
impl_protocol!(f64, 8, write_be_f64, read_be_f64);

impl Protocol for bool {
    type Clean = bool;
    #[allow(unused_variables)]
    fn proto_len(value: &bool) -> usize { 1 }
    fn proto_encode(value: bool, dst: &mut Writer) -> IoResult<()> {
        try!(dst.write_u8(if value { 1 } else { 0 }));
        Ok(())
    }
    #[allow(unused_variables)]
    fn proto_decode(src: &mut Reader, plen: usize) -> IoResult<bool> {
        let value = try!(src.read_u8());
        if value > 1 {
            Err(IoError {
                kind: IoErrorKind::InvalidInput,
                desc: "invalid bool value",
                detail: Some(format!("Invalid bool value, expecting 0 or 1, got {}", value))
            })
        } else {
            Ok(value == 1)
        }
    }
}

packets! {
    // Clientbound packets
    0x00 => ClientKeepAlive                 { keep_alive_id: VarInt }
    0x01 => ClientJoinGame                  { entity_id: i32, gamemode: u8, dimension: i8, difficulty: u8, max_players: u8, level_type: String, reduced_debug_info: bool }
    // 0x02 => ClientChatMessage               { data: Chat, position: i8 }
    0x03 => ClientTimeUpdate                { world_age: i64, time_of_day: i64 }
    // 0x04 => ClientEntityEquipment           { entity_id: i32, slot: i16, item: Nbt }
    0x05 => ClientSpawnPosition             { location: i64 }
    0x06 => ClientUpdateHealth              { health: f32, food: i32, food_saturation: f32 }
    0x07 => ClientRespawn                   { dimension: i32, difficulty: u8, gamemode: u8, level_type: String }
    0x08 => ClientPlayerPositionAndLook     { x: f64, y: f64, z: f64, yaw: f32, pitch: f32, flags: i8 }
    0x09 => ClientHeldItemChange            { slot: i8 }
    0x0a => ClientUseBed                    { entity_id: i32, location: i64 }
    0x0b => ClientAnimation                 { entity_id: i32, animation: u8 }
    // 0x0c => ClientSpawnPlayer               { entity_id: i32, player_uuid: Uuid, x: i32, y: i32, z: i32, yaw: u8, pitch: u8, current_item: i16, metadata: Metadata }
    0x0d => ClientCollectItem               { collected_entity_id: i32, collector_entity_id: i32 }
    // 0x0e => ClientSpawnObject               { entity_id: i32, type_: i8, x: i32, y: i32, z: i32, pitch: u8, yaw: u8, data: ObjectData }
    // 0x0f => ClientSpawnMob                  { entity_id: i32, type_: u8, x: i32, y: i32, z: i32, yaw: u8, pitch: u8, velocity_x: i16, velocity_y: i16, velocity_z: i16, metadata: Metadata }
    0x10 => ClientSpawnPainting             { entity_id: i32, title: String, location: i64, direction: u8 }
    0x11 => ClientSpawnExperienceOrb        { entity_id: i32, x: i32, y: i32, z: i32, count: i16 }
    0x12 => ClientEntityVelocity            { entity_id: i32, velocity_x: i16, velocity_y: i16, velocity_z: i16 }
    // 0x13 => ClientDestroyEntities           { entity_ids: Vec<VarInt> }
    0x14 => ClientEntity                    { entity_id: i32 }
    0x15 => ClientEntityRelativeMove        { entity_id: i32, delta_x: i8, delta_y: i8, delta_z: i8, on_ground: bool }
    0x16 => ClientEntityLook                { entity_id: i32, yaw: u8, pitch: u8, on_ground: bool }
    0x17 => ClientEntityLookAndRelativeMove { entity_id: i32, delta_x: i8, delta_y: i8, delta_z: i8, yaw: u8, pitch: u8, on_ground: bool }
    0x18 => ClientEntityTeleport            { entity_id: i32, x: i32, y: i32, z: i32, yaw: u8, pitch: u8, on_ground: bool }
    0x19 => ClientEntityHeadLook            { entity_id: i32, head_yaw: u8 }
    0x1A => ClientEntityStatus              { entity_id: i32, entity_status: i8 }
    0x1B => ClientAttachEntity              { entity_id: i32, vehicle_id: i32, leash: bool }
    // 0x1C => ClientEntityMetadata            { entity_id: i32, metadata: Metadata }
    0x1D => ClientEntityEffect              { entity_id: i32, effect_id: i8, amplifier: i8, duration: i32, hide_particles: bool }
    0x1E => ClientRemoveEntityEffect        { entity_id: i32, effect_id: i8 }
    0x1F => ClientSetExperience             { xp_bar: f32, level: i32, xp_total: i32 }
    // 0x20 => ClientEntityProperties          { entity_id: i32, properties: Vec<Property> }
    // 0x21 => ClientChunkData                 { chunk_x: i32, chunk_z: i32, ground_up_continuous: bool, mask: i16, chunk_data: Vec<Chunk> }
    // 0x22 => ClientMultiBlockChange          { chunk_x: i32, chunk_z: i32, records: Vec<Record> }
    0x23 => ClientBlockChange               { location: i64, block_id: i32 }
    0x24 => ClientBlockAction               { location: i64, byte1: u8, byte2: u8, block_type: i32 }
    0x25 => ClientBlockBreakAnimation       { entity_id: i32, location: i64, destroy_stage: i8 }
    // 0x26 => ClientMapChunkBulk              { sky_light_sent: bool, chunks: Option<(Vec<ChunkMeta>, Vec<Chunk>)> }
    // 0x27 => ClientExplosion                 { x: f32, y: f32, z: f32, radius: f32, records: Vec<ExplosionOffset>, player_motion_x: f32, player_motion_y: f32, player_motion_z: f32 }
    0x28 => ClientEffect                    { effect_id: i32, location: i64, data: i32, disable_relative_volume: bool }
    0x29 => ClientSoundEffect               { name: String, effect_x: i32, effect_y: i32, effect_z: i32, volume: f32, pitch: u8 }
    // 0x2a => ClientParticle                  { particle_id: i32, long_distance: bool, x: f32, y: f32, z: f32, offset_x: f32, offset_y: f32, offset_z: f32, particle_data: f32, data: Vec<VarInt> }
    0x2b => ClientChangeGameState           { reason: u8, value: f32 }
    0x2c => ClientSpawnGlobalEntity         { entity_id: i32, type_: i8, x: i32, y: i32, z: i32 }
    // 0x2d => ClientOpenWindow                { window_id: u8, window_type: String, window_title: Chat, slots: u8, entity_id: Option<i32> } // PROBLEM: entity_id depends on window_type
    0x2e => ClientCloseWindow               { window_id: u8 }
    // 0x2f => ClientSetSlot                   { window_id: u8, slot: i16, data: Slot }
    // 0x30 => ClientWindowItems               { window_id: u8, slots: Vec<Slot> }
    0x31 => ClientWindowProperty            { window_id: u8, property: i16, value: i16 }
    0x32 => ClientConfirmTransaction        { window_id: u8, action_number: i16, accepted: bool }
    0x33 => ClientUpdateSign                { location: i64, line0: String, line1: String, line2: String, line3: String }
    // 0x34 => ClientMaps                      { item_damage: i32, scale: i8, icons: MapIcons, columns: MapColumns }
    0x35 => ClientUpdateBlockEntity         { location: i64, action: u8, nbt_data: u8 }
    0x36 => ClientSignEditorOpen            { location: i64 }
    // 0x37 => ClientStatistics                { stats: Stats }
    // 0x38 => ClientPlayerListItem            { action: i32, players: Players }
    0x39 => ClientPlayerAbilities           { flags: i8, flying_speed: f32, walking_speed: f32 }
    // 0x3a => ClientTabComplete               { matches: Matches }
    // 0x3b => ClientScoreboardObjective       { objective_name: String, mode: ScoreMode }
    // 0x3c => ClientUpdateScore               { score_name: String, action: ScoreAction }
    0x3d => ClientDisplayScoreboard         { position: i8, score_name: String }
    // 0x3e => ClientTeams                     { team_name: String, team: Team }
    // 0x3f => ClientPluginMessage             { channel: String, data: Vec<u8> } // PROBLEM: length of `data` comes from packet length
    // 0x40 => ClientDisconnect                { reason: Chat }
    0x41 => ClientServerDifficulty          { difficulty: u8 }
    // 0x42 => ClientCombatEvent               { event: Event }
    0x43 => ClientCamera                    { camera_id: i32 }
    // 0x44 => ClientWorldBorder               { action: WorldBorderAction }
    // 0x45 => ClientTitle                     { action: TitleAction }
    0x46 => ClientSetCompression            { threshold: VarInt }
    // 0x47 => ClientPlayerListHeaderFooter    { header: Chat, footer: Chat }
    0x48 => ClientResourcePackSend          { url: String, hash: String }
    // 0x49 => ClientUpdateEntityNbt           { entity_id: i32, tag: Nbt }
    // Serverbound packets
    0x00 => ServerKeepAlive                 { keep_alive_id: i32 }
    0x01 => ServerChatMessage               { message: String }
    // 0x02 => ServerUseEntity                 { target: i32, use_type: UseType }
    0x03 => ServerPlayer                    { on_ground: bool }
    0x04 => ServerPlayerPosition            { x: f64, y: f64, z: f64, on_ground: bool }
    0x05 => ServerPlayerLook                { yaw: f32, pitch: f32, on_ground: bool }
    0x06 => ServerPlayerPositionAndLook     { x: f64, y: f64, z: f64, yaw: f32, pitch: f32, on_ground: bool }
    0x07 => ServerPlayerDigging             { status: i8, location: i64, face: i8 }
    // 0x08 => ServerPlayerBlockPlacement      { location: i64, direction: i8, held_item: Nbt, cursor_x: i8, cursor_y: i8, cursor_z: i8 }
    0x09 => ServerHeldItemChange            { slot: i16 }
    0x0a => ServerAnimation {}
    0x0b => ServerEntityAction              { entity_id: i32, action_id: i32, jump_boost: i32 }
    0x0c => ServerSteerVehicle              { sideways: f32, forward: f32, flags: u8 }
    0x0d => ServerCloseWindow               { window_id: i8 }
    // 0x0e => ServerClickWindow               { window_id: i8, slot: i16, button: i8, action_number: i16, mode: i8, clicked_item: Nbt }
    0x0f => ServerConfirmTransaction        { window_id: i8, action_number: i16, accepted: bool }
    0x10 => ServerCreativeInventoryAction   { slot: i16, clicked_item: i16 }
    0x11 => ServerEnchantItem               { window_id: i8, enchantment: i8 }
    0x12 => ServerUpdateSign                { location: i64, line0: String, line1: String, line2: String, line3: String }
    0x13 => ServerPlayerAbilities           { flags: i8, flying_speed: f32, walking_speed: f32 }
    // 0x14 => ServerTabComplete               { text: String, block_position: TabPosition }
    0x15 => ServerClientSettings            { locale: String, view_distance: i8, chat_mode: i8, chat_colors: bool, displayed_skin_parts: u8 }
    0x16 => ServerClientStatus              { action_id: i32 }
    // 0x17 => ServerPluginMessage             { channel: String, data: Vec<u8> }
    0x18 => ServerSpectate                  { target_player: Uuid }
    0x19 => ServerResourcePackStatus        { hash: String, result: i32 }
    // Handshake
    0x00 => Handshake                       { proto_version: VarInt, server_address: String, server_port: u16, next_state: VarInt }
    // Status
    0x00 => StatusResponse                  { response: String }
    0x00 => StatusRequest {}
    0x01 => StatusPing                      { time: i64 }
    // Login
    // 0x00 => LoginDisconnect                 { reason: Chat } // same as 0x40 client?
    0x00 => LoginStart                      { name: String }
    0x02 => LoginSuccess                    { uuid: String, username: String } // NOTE(toqueteos): uuid field is not an Uuid!
    0x03 => LoginSetCompression             { threshold: VarInt } // same as 0x46 client?
}
