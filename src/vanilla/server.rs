//! Vanilla server implementation.

use std::fs;
use std::io::{self, Write};
use std::net::TcpStream;
use std::path::Path;

use crypto::SymmStream;
use openssl::crypto::pkey::PKey;
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
    public_key: PKey,
    private_key: PKey,
}

impl Server {
    pub fn new() -> io::Result<Server> {
        use openssl::x509::X509Generator;

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

        let (cert, key) = X509Generator::new().generate().unwrap(); // TODO(brt): Map to appropriate ErrorKind if any

        Ok(Server {
            addr: addr,
            props: props,
            // players: vec![],
            worlds: vec![World::new()],
            public_key: cert.public_key(),
            private_key: key,
        })
    }

    pub fn addr(&self) -> &str { return &self.addr }
    pub fn port(&self) -> u16 { self.props.server_port }

    #[allow(unreachable_code)]
    pub fn handle(&self, mut stream: TcpStream) -> io::Result<()> {
        use packet::handshake::Packet::{self, Handshake};
        let state = match try!(Packet::read(&mut stream)) {
            Handshake(hs) => {
                debug!("Handshake proto_version={} server_address={} server_port={} next_state={:?}",
                         hs.proto_version, hs.server_address, hs.server_port, hs.next_state);
                hs.next_state
            }
        };
        match state {
            NextState::Status => {
                try!(slp::response(&mut stream));
                try!(slp::pong(&mut stream));
            }
            NextState::Login => {
                use openssl::crypto::pkey::EncryptionPadding;

                use packet::login::serverbound::Packet;
                use packet::login::serverbound::Packet::{LoginStart, EncryptionResponse};
                use packet::login::clientbound::{EncryptionRequest, LoginSuccess, SetCompression};

                let name = match try!(Packet::read(&mut stream)) {
                    LoginStart(login) => login.name,
                    EncryptionResponse(_) => {
                        return Err(io::Error::new(io::ErrorKind::InvalidInput,
                                   "Expecting login::serverbound::LoginStart packet, got EncryptionResponse"));
                    }
                };
                debug!(">> LoginStart name={}", name);

                try!(EncryptionRequest {
                    server_id: "".to_string(),
                    pubkey: self.public_key.save_pub(),
                    verify_token: "whatever".as_bytes().into(), // FIXME(brt): Should be randomly generated
                }.write(&mut stream));
                debug!("<< EncryptionRequest");

                let res = match try!(Packet::read(&mut stream)) {
                    EncryptionResponse(res) => res,
                    _ => {
                        return Err(io::Error::new(io::ErrorKind::InvalidInput,
                                                  "Expecting login::serverbound::EncryptionResponse"));
                    },
                };
                debug!(">> EncryptionResponse");

                // TODO(brt): Either verify or terminate the connection using login::clientbound::Disconnect

                let shared_secret = self.private_key.decrypt_with_padding(&res.shared_secret[..],
                                                                          EncryptionPadding::PKCS1v15);

                let mut stream = SymmStream::new(stream, &shared_secret[..]);

                // NOTE: threshold of `-1` disables compression
                let threshold = -1;
                try!(SetCompression { threshold: threshold }.write(&mut stream));
                debug!("<< LoginSetCompression");
                // try!(stream.flush());

                // NOTE: UUID *MUST* be sent with hyphens
                try!(LoginSuccess { uuid: Uuid::new_v4(), username: name }.write(&mut stream));
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
