//! Vanilla server implementation.

use std::fs;
use std::io::{self, Write};
use std::net::TcpStream;
use std::path::Path;

use packet::{NextState, PacketRead, PacketWrite};
use proto::properties::Properties;
use proto::slp;
use world::World;

use uuid::Uuid;

/// TODO(toqueteos): Move this to its own module. Proposal: src/vanilla/mod.rs
pub struct Server {
    addr: String,
    props: Properties,
    // Dummy player storage, just their username.
    // players: Vec<String>,
    worlds: Vec<World>,
}

impl Server {
    pub fn new() -> io::Result<Server> {
        let properties_path = &Path::new("server.properties");
        let props = match fs::metadata(properties_path) {
            // let props = match properties_path.metadata() {
            Ok(_) => try!(Properties::load(properties_path)),
            Err(_) => Properties::default(),
        };
        info!("{:?}", props);

        // There's no *prettier way* of doing this, if it was an Option then
        // there's .unwrap_or but it's just a String.
        let addr = if props.server_ip.is_empty() {
            "0.0.0.0".to_string()
        } else {
            props.server_ip.clone()
        };
        Ok(Server {
            addr,
            props,
            // players: vec![],
            worlds: vec![World::new()],
        })
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }
    pub fn port(&self) -> u16 {
        self.props.server_port
    }

    #[allow(unreachable_code)]
    pub fn handle(&self, mut stream: TcpStream) -> io::Result<()> {
        use packet::handshake::Packet::{self, Handshake};
        let state = match try!(Packet::read(&mut stream)) {
            Handshake(hs) => {
                debug!(
                    "Handshake proto_version={} server_address={} server_port={} next_state={:?}",
                    hs.proto_version, hs.server_address, hs.server_port, hs.next_state
                );
                hs.next_state
            }
        };
        match state {
            NextState::Status => {
                try!(slp::response(&mut stream));
                try!(slp::pong(&mut stream));
            }
            NextState::Login => {
                use packet::login::serverbound::Packet;
                use packet::login::serverbound::Packet::{EncryptionResponse, LoginStart};
                use packet::login::clientbound::{LoginSuccess, SetCompression};

                let username = match try!(Packet::read(&mut stream)) {
                    LoginStart(login) => login.name,
                    EncryptionResponse(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "Expecting login::serverbound::LoginStart packet, got EncryptionResponse",
                        ));
                    }
                };
                debug!(">> LoginStart name={}", username);

                // NOTE: threshold of `-1` disables compression
                let threshold = -1;
                try!(SetCompression { threshold }.write(&mut stream));
                debug!("<< LoginSetCompression");
                // try!(stream.flush());

                // NOTE: UUID *MUST* be sent with hyphens
                try!(
                    LoginSuccess {
                        uuid: Uuid::new_v4(),
                        username,
                    }.write(&mut stream)
                );
                debug!("<< LoginSuccess");
                // try!(stream.flush());

                // FIXME(toqueteos): Won't work because `name` is moved at `LoginSuccess`.
                // info!("Player {} joined.", name);

                // TODO(toqueteos): Add `name` to server's player list and do whatever else stuff is
                // required.

                try!(stream.flush());

                // TODO(toqueteos): Determine player world and send `stream` to it.
                try!(self.worlds[0].handle_player(stream));
            }
        }
        Ok(())
    }
}
