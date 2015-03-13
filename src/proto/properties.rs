//! Parse server.properties files

use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, BufWriter};
use std::num::ParseIntError;
use std::path::Path;
use std::str::ParseBoolError;

macro_rules! parse {
    ($value:ident, String) => {
        $value.to_string()
    };
    ($value:ident, bool) => {
        try!($value.parse().map_err(|_: ParseBoolError| io::Error::new(io::ErrorKind::InvalidInput, "invalid bool value", None)))
    };
    ($value:ident, i32) => {
        try!($value.parse().map_err(|_: ParseIntError| io::Error::new(io::ErrorKind::InvalidInput, "invalid i32 value", None)))
    }
}

macro_rules! server_properties_impl {
    ($({ $field:ident, $hyphen:expr, $fty:ident, $default:expr})+) => {
        /// Vanilla server.properties
        ///
        /// Documentation of each filed here: http://minecraft.gamepedia.com/Server.properties
        #[derive(Debug)]
        pub struct Properties {
            $(pub $field: $fty),*
        }

        impl Properties {
            pub fn default() -> Properties {
                Properties{
                    $($field: $default),*
                }
            }

            /// Load and parse a server.properties file from `path`,
            pub fn load(path: &Path) -> io::Result<Properties> {
                let mut p = Properties::default();
                let file = try!(File::open(path));
                let file = BufReader::new(file);
                for line in file.lines().map(|l| l.unwrap()) {
                    // Ignore comment lines
                    if line.trim().starts_with("#") {
                        continue
                    }
                    let parts: Vec<&str> = line.trim().splitn(1, '=').collect();
                    let (prop, value) = (parts[0], parts[1]);
                    match prop {
                        $(stringify!($field) => p.$field = parse!(value, $fty),)*
                        prop => println!("Unknown property {}", prop)
                    }
                }
                Ok(p)
            }

            /// Saves a server.properties file into `path`. It creates the
            /// file if it does not exist, and will truncate it if it does.
            pub fn save(&self, path: &Path) -> io::Result<()> {
                let file = try!(File::create(path));
                let mut file = BufWriter::new(file);
                // Header
                try!(write!(&mut file, "#Minecraft server properties"));
                try!(write!(&mut file, "#(File modification datestamp)"));
                // Body. Vanilla MC does write 37 out of 40 properties by default, it
                // only writes the 3 left if they are not using default values. It
                // also writes them unsorted (possibly because they are stored in a
                // HashMap).
                $(try!(write!(&mut file, "{}={}\n", $hyphen, self.$field));)*
                Ok(())
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn decode_default() {
                let props = Properties::default();
                $(assert_eq!(props.$field, $default));*
            }
        }
    }
}

server_properties_impl! {
    { allow_flight, "allow-flight", bool, false }
    { allow_nether, "allow-nether", bool, true }
    { announce_player_achievements, "announce-player-achievements", bool, true }
    { difficulty, "difficulty", i32, 1 }
    { enable_query, "enable-query", bool, false }
    { enable_rcon, "enable-rcon", bool, false }
    { enable_command_block, "enable-command-block", bool, false }
    { force_gamemode, "force-gamemode", bool, false }
    { gamemode, "gamemode", i32, 0 }
    { generate_structures, "generate-structures", bool, true }
    { generator_settings, "generator-settings", String, "".to_string() }
    { hardcore, "hardcore", bool, false }
    { level_name, "level-name", String, "world".to_string() }
    { level_seed, "level-seed", String, "".to_string() }
    { level_type, "level-type", String, "DEFAULT".to_string() }
    { max_build_height, "max-build-height", i32, 256 }
    { max_players, "max-players", i32, 20 }
    { max_tick_time, "max-tick-time", i32, 60000 }
    { max_world_size, "max-world-size", i32, 29999984 }
    { motd, "motd", String, "A Minecraft Server".to_string() }
    { network_compression_threshold, "network-compression-threshold", i32, 256 }
    { online_mode, "online-mode", bool, true }
    { op_permission_level, "op-permission-level", i32, 4 }
    { player_idle_timeout, "player-idle-timeout", i32, 0 }
    { pvp, "pvp", bool, true }
    { query_port, "query.port", i32, 25565 }
    { rcon_password, "rcon.password", String, "".to_string() }
    { rcon_port, "rcon.port", i32, 25575 }
    { resource_pack, "resource-pack", String, "".to_string() }
    { resource_pack_hash, "resource-pack-hash", String, "".to_string() }
    { server_ip, "server-ip", String, "".to_string() }
    { server_port, "server-port", i32, 25565 }
    { snooper_enabled, "snooper-enabled", bool, true }
    { spawn_animals, "spawn-animals", bool, true }
    { spawn_monsters, "spawn-monsters", bool, true }
    { spawn_npcs, "spawn-npcs", bool, true }
    { spawn_protection, "spawn-protection", i32, 16 }
    { use_native_transport, "use-native-transport", bool, true }
    { view_distance, "view-distance", i32, 10 }
    { white_list, "white-list", bool, false }
}
