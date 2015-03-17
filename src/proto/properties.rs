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
    ($({ $field:ident, $fty:ident, $default:expr})+) => {
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
                $(try!(write!(&mut file, "{}={}\n", stringify!($field), self.$field));)*
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
    { allow_flight, bool, false }
    { allow_nether, bool, true }
    { announce_player_achievements, bool, true }
    { difficulty, i32, 1 }
    { enable_query, bool, false }
    { enable_rcon, bool, false }
    { enable_command_block, bool, false }
    { force_gamemode, bool, false }
    { gamemode, i32, 0 }
    { generate_structures, bool, true }
    { generator_settings, String, "".to_string() }
    { hardcore, bool, false }
    { level_name, String, "world".to_string() }
    { level_seed, String, "".to_string() }
    { level_type, String, "DEFAULT".to_string() }
    { max_build_height, i32, 256 }
    { max_players, i32, 20 }
    { max_tick_time, i32, 60000 }
    { max_world_size, i32, 29999984 }
    { motd, String, "A Minecraft Server".to_string() }
    { network_compression_threshold, i32, 256 }
    { online_mode, bool, true }
    { op_permission_level, i32, 4 }
    { player_idle_timeout, i32, 0 }
    { pvp, bool, true }
    { query_port, i32, 25565 }
    { rcon_password, String, "".to_string() }
    { rcon_port, i32, 25575 }
    { resource_pack, String, "".to_string() }
    { resource_pack_hash, String, "".to_string() }
    { server_ip, String, "".to_string() }
    { server_port, i32, 25565 }
    { snooper_enabled, bool, true }
    { spawn_animals, bool, true }
    { spawn_monsters, bool, true }
    { spawn_npcs, bool, true }
    { spawn_protection, i32, 16 }
    { use_native_transport, bool, true }
    { view_distance, i32, 10 }
    { white_list, bool, false }
}
