//! Worlds (a group of dimensions).
//!
//! This module is a WORK IN PROGRESS.

use std::io::{self, Write};
use std::net::TcpStream;

use packet::{ChunkMeta, PacketRead, PacketWrite};
use types::consts::*;
use types::{Chunk, ChunkColumn};

use rand;
use time;

/// World is a set of dimensions which tick in sync.
pub struct World {
    start: time::Timespec
}

impl World {
    pub fn new() -> World {
        World { start: time::get_time() }
    }

    // FIXME(toqueteos): Read from world's level.dat file
    pub fn world_age(&self) -> i64 {
        let end = time::get_time();
        let elapsed = (end - self.start).num_seconds();
        elapsed * 20
    }

    // FIXME(toqueteos): Read from world's level.dat file
    pub fn time_of_day(&self) -> i64 {
        self.world_age() % 24000
    }

    #[allow(unreachable_code)]
    pub fn handle_player(&self, mut stream: TcpStream) -> io::Result<()> {
        use packet::play::serverbound::Packet;
        use packet::play::serverbound::Packet::ClientSettings;
        use packet::play::clientbound::{ChangeGameState, ChunkDataBulk, JoinGame, KeepAlive};
        use packet::play::clientbound::{PlayerAbilities, PlayerPositionAndLook};
        use packet::play::clientbound::{PluginMessage, TimeUpdate, WorldSpawn};

        // FIXME(toqueteos): We need:
        // - An id generator, can't use UUID here
        // - Read world info from disk
        // - Read some keypairs from server.properties
        try!(JoinGame {
                 entity_id: 0,
                 gamemode: 0b0000, // 0: Survival, 1: Creative, 2: Adventure, 3: Spectator
                 dimension: Dimension::Overworld,
                 difficulty: 2,
                 max_players: 20,
                 level_type: "default".to_string(),
                 reduced_debug_info: false,
             }
             .write(&mut stream));
        debug!("<< JoinGame");
        // try!(stream.flush());

        // FIXME(toqueteos): Verify `flying_speed` and `walking_speed` values
        // are good, now they are just taken from Glowstone impl.
        // `flags` value is read from server's player list.
        try!(PlayerAbilities {
                 flags: 0b0000, // god | can fly | flying | creative
                 flying_speed: 0.05,
                 walking_speed: 0.1,
             }
             .write(&mut stream));
        debug!("<< PlayerAbilities");
        // try!(stream.flush());

        // WRITE `MC|Brand` plugin
        try!(PluginMessage {
            channel: "MC|Brand".to_string(),
            data: b"hematite".to_vec()
        }.write(&mut stream));
        debug!("<< PluginMessage");
        // try!(stream.flush());

        // WRITE supported channels
        try!(PluginMessage {
            channel: "REGISTER".to_string(),
            data: b"MC|Brand\0".to_vec()
        }.write(&mut stream));
        debug!("<< PluginMessage");
        // try!(stream.flush());

        // FIXME(toqueteos): We need a chunk loader handling disk reads and
        // using real chunks not made up ones.
        let mut meta = vec![];
        let mut data = vec![];
        for z in -1..2 {
            for x in -1..2 {
                meta.push(ChunkMeta { x: x, z: z, mask: 0b000_0000_0000_1111 });
                data.push(ChunkColumn {
                    chunks: vec![
                        Chunk::new(1 << 4, 0xff),
                        Chunk::new(2 << 4, 0xff),
                        Chunk::new(3 << 4, 0xff),
                        Chunk::new(4 << 4, 0xff),
                    ],
                    biomes: Some([1u8; 256])
                });
            }
        }
        try!(ChunkDataBulk {
            sky_light_sent: true,
            chunk_meta: meta,
            chunk_data: data,
        }.write(&mut stream));
        debug!("<< ChunkDataBulk");
        // try!(stream.flush());

        // Send Compass
        try!(WorldSpawn { location: [10, 65, 10] }.write(&mut stream));
        debug!("<< WorldSpawn");
        // try!(stream.flush());

        // Send Time
        try!(TimeUpdate {
            world_age: self.world_age(),
            time_of_day: self.time_of_day()
        }.write(&mut stream));
        debug!("<< TimeUpdate");
        // try!(stream.flush());

        // Send Weather
        try!(ChangeGameState { reason: 1, value: 0.0 }.write(&mut stream));
        debug!("<< ChangeGameState Weather");
        // try!(stream.flush());

        // Send RainDensity
        try!(ChangeGameState { reason: 8, value: 0.0 }.write(&mut stream));
        debug!("<< ChangeGameState RainDensity");
        // try!(stream.flush());

        // Send SkyDarkness
        try!(ChangeGameState { reason: 9, value: 0.0 }.write(&mut stream));
        debug!("<< ChangeGameState SkyDarkness");
        // try!(stream.flush());

        // // Send Inventory items
        // let wi = ClientWindowItems {
        //     window_id: 0,
        //     slots: repeat(EMPTY_SLOT).take(45).collect()
        // };
        // try!(wi.write(&mut stream));
        debug!("<< WindowItems (not sent)");
        // try!(stream.flush());

        try!(PlayerPositionAndLook {
            position: [0.0, 64.0, 0.0],
            yaw: 0.0,
            pitch: 0.0,
            flags: 0x1f
        }.write(&mut stream));
        debug!("<< PlayerPositionAndLook");
        // try!(stream.flush());

        // Read Client Settings
        match try!(Packet::read(&mut stream)) {
            ClientSettings(cs) => debug!(">> ClientSettings {:?}", cs),
            wrong_packet => panic!("Expecting play::serverbound::ClientSettings packet, got {:?}", wrong_packet)
        }

        // let cm = ChatMessage { data: Chat::new("Server: Welcome to hematite server!"), position: 1 };
        // try!(cm.write(&mut stream));
        // debug!("<< ChatMessage data={:?} position={}", cm.data, cm.position);
        // try!(stream.flush());

        // Send first Keep Alive
        try!(KeepAlive { keep_alive_id: rand::random() }.write(&mut stream));
        debug!("<< KeepAlive");
        try!(stream.flush());

        loop {
            match try!(Packet::read(&mut stream)) {
                // 0x00 => KeepAlive { keep_alive_id: i32 }
                Packet::KeepAlive(pkt) => {
                    info!("0x00 KeepAlive - keep_alive_id={}", pkt.keep_alive_id);
                }
                // 0x01 => ChatMessage { message: String }
                Packet::ChatMessage(pkt) => {
                    use packet::play::clientbound::{ChatMessage, PlayerAbilities};
                    use types::ChatJson;

                    info!("0x01 ChatMessage - message={}", pkt.message);

                    match pkt.message.trim() {
                        "/gamemode s" => {
                            try!(PlayerAbilities {
                                     flags: 0b0000, // god | can fly | flying | creative
                                     flying_speed: 0.05,
                                     walking_speed: 0.1,
                                 }
                                 .write(&mut stream));
                        }
                        "/gamemode c" => {
                            try!(PlayerAbilities {
                                     flags: 0b1101, // god | can fly | flying | creative
                                     flying_speed: 0.05,
                                     walking_speed: 0.1,
                                 }
                                 .write(&mut stream));
                        }
                        _ => {}
                    }

                    let response = ChatMessage {
                        data: ChatJson::from(pkt.message),
                        position: 0,
                    };
                    try!(response.write(&mut stream));
                }
                // 0x02 => UseEntity { target_eid: i32, use_type: EntityUseAction }
                // Packet::UseEntity(pkt) => {
                //     info!("0x02 UseEntity");
                // }
                // 0x03 => PlayerIdle { on_ground: bool }
                Packet::PlayerIdle(pkt) => {
                    info!("0x03 PlayerIdle - on_ground={}", pkt.on_ground);
                }
                // 0x04 => PlayerPosition { position: [f64; 3], on_ground: bool }
                Packet::PlayerPosition(pkt) => {
                    info!("0x04 PlayerPosition - position={:?} on_ground={}",
                          pkt.position,
                          pkt.on_ground);
                }
                // 0x05 => PlayerLook { yaw: f32, pitch: f32, on_ground: bool }
                Packet::PlayerLook(pkt) => {
                    info!("0x05 PlayerLook - yaw={} pitch={} on_ground={}",
                          pkt.yaw,
                          pkt.pitch,
                          pkt.on_ground);
                }
                // 0x06 => PlayerPositionAndLook { position: [f64; 3], yaw: f32, pitch: f32, on_ground: bool }
                Packet::PlayerPositionAndLook(pkt) => {
                    info!("0x06 PlayerPositionAndLook - position={:?} yaw={} pitch={} \
                           on_ground={}",
                          pkt.position,
                          pkt.yaw,
                          pkt.pitch,
                          pkt.on_ground);
                }
                // 0x07 => PlayerDigging { status: i8, location: BlockPos, face: i8 }
                Packet::PlayerDigging(pkt) => {
                    info!("0x07 PlayerDigging - status={} location={:?} face={}",
                          pkt.status,
                          pkt.location,
                          pkt.face);
                }
                // 0x08 => PlayerBlockPlacement { location: BlockPos, direction: i8, held_item: Option<Slot>, cursor: [i8; 3] }
                Packet::PlayerBlockPlacement(pkt) => {
                    info!("0x08 PlayerBlockPlacement - location={:?} direction={} held_item={:?} \
                           cursor={:?}",
                          pkt.location,
                          pkt.direction,
                          pkt.held_item,
                          pkt.cursor);
                }
                // 0x09 => HeldItemChange { slot: i16 }
                Packet::HeldItemChange(pkt) => {
                    info!("0x09 HeldItemChange - slot={}", pkt.slot);
                }
                // 0x0a => Animation {}
                Packet::Animation(_) => {
                    info!("0x0a Animation");
                }
                // 0x0b => EntityAction { entity_id: Var<i32>, action_id: Var<i32>, jump_boost: Var<i32> }
                Packet::EntityAction(pkt) => {
                    info!("0x0b EntityAction - entity_id={} action_id={} jump_boost={}",
                          pkt.entity_id,
                          pkt.action_id,
                          pkt.jump_boost);
                }
                // 0x0c => SteerVehicle { sideways: f32, forward: f32, flags: u8 }
                Packet::SteerVehicle(pkt) => {
                    info!("0x0c SteerVehicle - sideways={} forward={} flags={}",
                          pkt.sideways,
                          pkt.forward,
                          pkt.flags);
                }
                // 0x0d => CloseWindow { window_id: u8 }
                Packet::CloseWindow(pkt) => {
                    info!("0x0d CloseWindow - window_id={}", pkt.window_id);
                }
                // 0x0e => ClickWindow { window_id: u8, slot: i16, button: i8, action_number: i16, mode: i8, clicked_item: Option<Slot> }
                Packet::ClickWindow(pkt) => {
                    info!("0x0e ClickWindow - window_id={} button={} action_number={} mode={} \
                           clicked_item={:?}",
                          pkt.window_id,
                          pkt.button,
                          pkt.action_number,
                          pkt.mode,
                          pkt.clicked_item);
                }
                // 0x0f => ConfirmTransaction { window_id: u8, action_number: i16, accepted: bool }
                Packet::ConfirmTransaction(pkt) => {
                    info!("0x0f ConfirmTransaction - window_id={} action_number={} accepted={}",
                          pkt.window_id,
                          pkt.action_number,
                          pkt.accepted);
                }
                // 0x10 => CreativeInventoryAction { slot: i16, clicked_item: Option<Slot> }
                Packet::CreativeInventoryAction(pkt) => {
                    info!("0x10 CreativeInventoryAction - slot={} clicked_item={:?}",
                          pkt.slot,
                          pkt.clicked_item);
                }
                // 0x11 => EnchantItem { window_id: u8, enchantment: i8 }
                Packet::EnchantItem(pkt) => {
                    info!("0x11 EnchantItem - window_id={} enchantment={}",
                          pkt.window_id,
                          pkt.enchantment);
                }
                // 0x12 => UpdateSign { location: BlockPos, line0: ChatJson, line1: ChatJson, line2: ChatJson, line3: ChatJson }
                Packet::UpdateSign(pkt) => {
                    info!("0x12 UpdateSign - location={:?} line0={:?} line1={:?} line2={:?} \
                           line3={:?}",
                          pkt.location,
                          pkt.line0,
                          pkt.line1,
                          pkt.line2,
                          pkt.line3);
                }
                // 0x13 => PlayerAbilities { flags: i8, flying_speed: f32, walking_speed: f32 }
                Packet::PlayerAbilities(pkt) => {
                    info!("0x13 PlayerAbilities - flags={} flying_speed={} walking_speed={}",
                          pkt.flags,
                          pkt.flying_speed,
                          pkt.walking_speed);
                }
                // 0x14 => TabComplete { text: String, looking_at: Option<i64> }
                Packet::TabComplete(pkt) => {
                    info!("0x14 TabComplete - text={} looking_at={:?}",
                          pkt.text,
                          pkt.looking_at);
                }
                // 0x15 => ClientSettings { locale: String, view_distance: i8, chat_mode: i8, chat_colors: bool, displayed_skin_parts: u8 }
                Packet::ClientSettings(pkt) => {
                    info!("0x15 ClientSettings - locale={} view_distance={} chat_mode={} \
                           chat_colors={} displayed_skin_parts={}",
                          pkt.locale,
                          pkt.view_distance,
                          pkt.chat_mode,
                          pkt.chat_colors,
                          pkt.displayed_skin_parts);
                }
                // 0x16 => ClientStatus { action_id: Var<i32> }
                Packet::ClientStatus(pkt) => {
                    info!("0x16 ClientStatus - action_id={}", pkt.action_id);

                    match pkt.action_id {
                        1 | 2 => {
                            use packet::Stat;
                            use packet::play::clientbound::Statistics;

                            let stats = vec![
                            Stat { name: "achievement.openInventory".to_string(), value: 1 },
                            Stat { name: "achievement.mineWood".to_string(), value: 0 },
                            Stat { name: "achievement.buildWorkBench".to_string(), value: 0 },
                            Stat { name: "achievement.buildPickaxe".to_string(), value: 0 },
                            Stat { name: "achievement.buildFurnace".to_string(), value: 0 },
                            Stat { name: "achievement.acquireIron".to_string(), value: 0 },
                            Stat { name: "achievement.buildHoe".to_string(), value: 0 },
                            Stat { name: "achievement.makeBread".to_string(), value: 0 },
                            Stat { name: "achievement.bakeCake".to_string(), value: 0 },
                            Stat { name: "achievement.buildBetterPickaxe".to_string(), value: 0 },
                            Stat { name: "achievement.cookFish".to_string(), value: 0 },
                            Stat { name: "achievement.onARail".to_string(), value: 0 },
                            Stat { name: "achievement.buildSword".to_string(), value: 0 },
                            Stat { name: "achievement.killEnemy".to_string(), value: 0 },
                            Stat { name: "achievement.killCow".to_string(), value: 0 },
                            Stat { name: "achievement.flyPig".to_string(), value: 0 },
                            Stat { name: "achievement.snipeSkeleton".to_string(), value: 0 },
                            Stat { name: "achievement.diamonds".to_string(), value: 0 },
                            Stat { name: "achievement.diamondsToYou".to_string(), value: 0 },
                            Stat { name: "achievement.portal".to_string(), value: 0 },
                            Stat { name: "achievement.ghast".to_string(), value: 0 },
                            Stat { name: "achievement.blazeRod".to_string(), value: 0 },
                            Stat { name: "achievement.potion".to_string(), value: 0 },
                            Stat { name: "achievement.theEnd".to_string(), value: 0 },
                            Stat { name: "achievement.theEnd2".to_string(), value: 0 },
                            Stat { name: "achievement.enchantments".to_string(), value: 0 },
                            Stat { name: "achievement.overkill".to_string(), value: 0 },
                            Stat { name: "achievement.bookcase".to_string(), value: 0 },
                            Stat { name: "achievement.breedCow".to_string(), value: 0 },
                            Stat { name: "achievement.spawnWither".to_string(), value: 0 },
                            Stat { name: "achievement.killWither".to_string(), value: 0 },
                            Stat { name: "achievement.fullBeacon".to_string(), value: 0 },
                            Stat { name: "achievement.exploreAllBiomes".to_string(), value: 0 },
                            Stat { name: "stat.leaveGame".to_string(), value: 0 },
                            Stat { name: "stat.playOneMinute".to_string(), value: 0 },
                            Stat { name: "stat.timeSinceDeath".to_string(), value: 0 },
                            Stat { name: "stat.sneakTime".to_string(), value: 0 },
                            Stat { name: "stat.walkOneCm".to_string(), value: 0 },
                            Stat { name: "stat.crouchOneCm".to_string(), value: 0 },
                            Stat { name: "stat.sprintOneCm".to_string(), value: 0 },
                            Stat { name: "stat.swimOneCm".to_string(), value: 0 },
                            Stat { name: "stat.fallOneCm".to_string(), value: 0 },
                            Stat { name: "stat.climbOneCm".to_string(), value: 0 },
                            Stat { name: "stat.flyOneCm".to_string(), value: 0 },
                            Stat { name: "stat.diveOneCm".to_string(), value: 0 },
                            Stat { name: "stat.minecartOneCm".to_string(), value: 0 },
                            Stat { name: "stat.boatOneCm".to_string(), value: 0 },
                            Stat { name: "stat.pigOneCm".to_string(), value: 0 },
                            Stat { name: "stat.horseOneCm".to_string(), value: 0 },
                            Stat { name: "stat.aviateOneCm".to_string(), value: 0 },
                            Stat { name: "stat.jump".to_string(), value: 0 },
                            Stat { name: "stat.damageDealt".to_string(), value: 0 },
                            Stat { name: "stat.damageTaken".to_string(), value: 0 },
                            Stat { name: "stat.deaths".to_string(), value: 0 },
                            Stat { name: "stat.mobKills".to_string(), value: 0 },
                            Stat { name: "stat.playerKills".to_string(), value: 0 },
                            Stat { name: "stat.drop".to_string(), value: 0 },
                            Stat { name: "stat.itemEnchanted".to_string(), value: 0 },
                            Stat { name: "stat.animalsBred".to_string(), value: 0 },
                            Stat { name: "stat.fishCaught".to_string(), value: 0 },
                            Stat { name: "stat.junkFished".to_string(), value: 0 },
                            Stat { name: "stat.treasureFished".to_string(), value: 0 },
                            Stat { name: "stat.talkedToVillager".to_string(), value: 0 },
                            Stat { name: "stat.tradedWithVillager".to_string(), value: 0 },
                            Stat { name: "stat.cakeSlicesEaten".to_string(), value: 0 },
                            Stat { name: "stat.cauldronFilled".to_string(), value: 0 },
                            Stat { name: "stat.cauldronUsed".to_string(), value: 0 },
                            Stat { name: "stat.armorCleaned".to_string(), value: 0 },
                            Stat { name: "stat.bannerCleaned".to_string(), value: 0 },
                            Stat { name: "stat.brewingstandInteraction".to_string(), value: 0 },
                            Stat { name: "stat.beaconInteraction".to_string(), value: 0 },
                            Stat { name: "stat.craftingTableInteraction".to_string(), value: 0 },
                            Stat { name: "stat.furnaceInteraction".to_string(), value: 0 },
                            Stat { name: "stat.dispenserInspected".to_string(), value: 0 },
                            Stat { name: "stat.dropperInspected".to_string(), value: 0 },
                            Stat { name: "stat.hopperInspected".to_string(), value: 0 },
                            Stat { name: "stat.chestOpened".to_string(), value: 0 },
                            Stat { name: "stat.trappedChestTriggered".to_string(), value: 0 },
                            Stat { name: "stat.enderchestOpened".to_string(), value: 0 },
                            Stat { name: "stat.noteblockPlayed".to_string(), value: 0 },
                            Stat { name: "stat.noteblockTuned".to_string(), value: 0 },
                            Stat { name: "stat.flowerPotted".to_string(), value: 0 },
                            Stat { name: "stat.recordPlayed".to_string(), value: 0 },
                            Stat { name: "stat.sleepInBed".to_string(), value: 0 },
                        ];
                            let response = Statistics { stats: stats };
                            try!(response.write(&mut stream));
                        }
                        _ => {}
                    }
                }
                // 0x17 => PluginMessage { channel: String, data: Vec<u8> }
                Packet::PluginMessage(pkt) => {
                    info!("0x17 PluginMessage - channel={} data={:?}",
                          pkt.channel,
                          pkt.data);
                }
                // 0x18 => Spectate { target_player: Uuid }
                Packet::Spectate(pkt) => {
                    info!("0x18 Spectate - target_player={}", pkt.target_player);
                }
                // 0x19 => ResourcePackStatus { hash: String, result: Var<i32> }
                Packet::ResourcePackStatus(pkt) => {
                    info!("0x19 ResourcePackStatus - hash={} result={}",
                          pkt.hash,
                          pkt.result);
                }
            }
        }

        Ok(())
    }
}
