//! Vanilla server implementation.

use std::fs;
use std::io::{self, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::mpsc;

use packet::{NextState, PacketRead, PacketWrite};
use proto::properties::Properties;
use proto::slp;
use world::World;

use uuid::Uuid;

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
            addr: addr,
            props: props,
            // players: vec![],
            worlds: vec![World::new()],
        })
    }

    pub fn addr(&self) -> &str {
        return &self.addr;
    }
    pub fn port(&self) -> u16 {
        self.props.server_port
    }

    pub fn keep_alive_loop(rx: mpsc::Receiver<TcpStream>) {
        use std::ops::Add;
        use std::thread;
        use std::time::Duration;

        use rand;

        use packet::play::clientbound::KeepAlive;

        let mut connections: Vec<TcpStream> = Vec::new();
        loop {
            // Try recv from channel for 5 seconds adding `TcpStream`s to connections
            let mut timeout = Duration::new(0, 0);
            loop {
                match rx.try_recv() {
                    Ok(stream) => {
                        connections.push(stream);
                    }
                    Err(_) => {
                        let delta = Duration::from_millis(30);
                        thread::sleep(delta);
                        timeout = timeout.add(delta);
                    }
                }
                if timeout.as_secs() > 5 {
                    break;
                }
            }

            // Send KeepAlive packet "request"s
            let mut length = connections.len();
            let mut i: usize = 0;
            while i < length {
                let pkt = KeepAlive { keep_alive_id: rand::random() };
                let peer_addr = connections[i].peer_addr().expect("<unknown peer addr>");
                match pkt.write(&mut connections[i]) {
                    Ok(_) => {
                        info!("{} KeepAlive - keep_alive_id={}",
                              peer_addr,
                              pkt.keep_alive_id)
                    }
                    Err(_) => {
                        connections.swap_remove(i);
                        length -= 1;
                        info!("Disconnected {}", peer_addr);
                        continue;
                    }
                }
                i += 1;
            }

            thread::sleep(Duration::from_secs(5));
        }
    }

    #[allow(unreachable_code)]
    pub fn handle(&self,
                  mut stream: TcpStream,
                  keep_alive_tx: mpsc::Sender<TcpStream>)
                  -> io::Result<()> {
        use packet::handshake::Packet::{self, Handshake};
        let state = match try!(Packet::read(&mut stream)) {
            Handshake(hs) => {
                debug!("Handshake proto_version={} server_address={} server_port={} \
                        next_state={:?}",
                       hs.proto_version,
                       hs.server_address,
                       hs.server_port,
                       hs.next_state);
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
                use packet::login::serverbound::Packet::{LoginStart, EncryptionResponse};
                use packet::login::clientbound::{LoginSuccess, SetCompression};

                let name = match try!(Packet::read(&mut stream)) {
                    LoginStart(login) => login.name,
                    EncryptionResponse(_) => {
                        return Err(io::Error::new(io::ErrorKind::InvalidInput,
                                                  "Expecting login::serverbound::LoginStart \
                                                   packet, got EncryptionResponse"));
                    }
                };
                debug!(">> LoginStart name={}", name);

                // {
                //     use types::ChatJson;
                //     try!(Disconnect { reason: ChatJson::from("Succesful login but still kicking you from Rust!") }.write(&mut stream));
                // }

                // NOTE: threshold of `-1` disables compression
                let threshold = -1;
                try!(SetCompression { threshold: threshold }.write(&mut stream));
                debug!("<< LoginSetCompression");
                // try!(stream.flush());

                // NOTE: UUID *MUST* be sent with hyphens
                try!(LoginSuccess {
                         uuid: Uuid::new_v4(),
                         username: name,
                     }
                     .write(&mut stream));
                debug!("<< LoginSuccess");
                // try!(stream.flush());

                // FIXME(toqueteos): Won't work because `name` is moved at `LoginSuccess`.
                // info!("Player {} joined.", name);

                // TODO(toqueteos): Add `name` to server's player list and do whatever else stuff is
                // required.

                try!(stream.flush());

                // TODO(toqueteos): Determine player world and send `stream` to it.
                try!(self.worlds[0].handle_player(stream, keep_alive_tx));
            }
        }
        Ok(())
    }
}
