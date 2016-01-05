//! Worlds (a group of dimensions).
//!
//! This module is a WORK IN PROGRESS.

use std::io::{self, Read, Write};
use std::thread::sleep;
use std::time::Duration;

use packet::{ChunkMeta, PacketRead, PacketWrite, Protocol};
use types::consts::*;
use types::{Chunk, ChunkColumn, Var};
use util::ReadExactly;

use rand;
use time;

// Temporal, only used within the BLOCK OF SHAME
const PACKET_NAMES: [&'static str; 26] = [
    "(c2s) KeepAlive",
    "(c2s) ChatMessage",
    "(c2s) UseEntity",
    "(c2s) Player",
    "(c2s) PlayerPosition",
    "(c2s) PlayerLook",
    "(c2s) PlayerPositionAndLook",
    "(c2s) PlayerDigging",
    "(c2s) PlayerBlockPlacement",
    "(c2s) HeldItemChange",
    "(c2s) Animation",
    "(c2s) EntityAction",
    "(c2s) SteerVehicle",
    "(c2s) CloseWindow",
    "(c2s) ClickWindow",
    "(c2s) ConfirmTransaction",
    "(c2s) CreativeInventoryAction",
    "(c2s) EnchantItem",
    "(c2s) UpdateSign",
    "(c2s) PlayerAbilities",
    "(c2s) TabComplete",
    "(c2s) ClientSettings",
    "(c2s) ClientStatus",
    "(c2s) PluginMessage",
    "(c2s) Spectate",
    "(c2s) ResourcePackStatus"
];

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
    pub fn handle_player<S: Read + Write>(&self, mut stream: S) -> io::Result<()> {
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
            gamemode: 0b0010,
            dimension: Dimension::Overworld,
            difficulty: 2,
            max_players: 20,
            level_type: "default".to_string(),
            reduced_debug_info: false
        }.write(&mut stream));
        debug!("<< JoinGame");
        // try!(stream.flush());

        // FIXME(toqueteos): Verify `flying_speed` and `walking_speed` values
        // are good, now they are just taken from Glowstone impl.
        // `flags` value is read from server's player list.
        try!(PlayerAbilities {
            flags: 0b1101, // flying and creative
            flying_speed: 0.05,
            walking_speed: 0.1
        }.write(&mut stream));
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

        // Send Abilities
        try!(PlayerAbilities {
            flags: 0b1101, // flying and creative
            flying_speed: 0.05,
            walking_speed: 0.1
        }.write(&mut stream));
        debug!("<< PlayerAbilities");
        try!(stream.flush());

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

        // BLOCK OF SHAME
        let mut t1 = time::get_time();
        loop {
            let t2 = time::get_time();
            let t = (t2 - t1).num_seconds();

            // Manually skip over incoming packets
            let len = try!(<Var<i32> as Protocol>::proto_decode(&mut stream));
            let id = try!(<Var<i32> as Protocol>::proto_decode(&mut stream));
            let n_read = len - 1;
            let buf = try!(stream.read_exactly(n_read as usize));
            // We could add a filter here, chat messages might be info!, position packets are debug!, etc...
            debug!("id={} length={} buf={:?} t2-t={}", PACKET_NAMES[id as usize], len, buf, t);

            // Send KeepAlive every 20 seconds, otherwise client times out
            if t > 20 {
                try!(KeepAlive { keep_alive_id: rand::random() }.write(&mut stream));
                debug!("<< KeepAlive");
                try!(stream.flush());

                t1 = time::get_time();
            }

            sleep(Duration::from_millis(15));
        }
        // /BLOCK OF SHAME

        Ok(())
    }
}
