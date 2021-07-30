//! MC Server List Ping protocol.
//!
//! Reference: <http://wiki.vg/Server_List_Ping>

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::ErrorKind::InvalidInput;
use std::net::TcpStream;
use std::ops::Sub; // Sub for Timespec
use std::path::Path;

use crate::consts;
use crate::packet::{PacketRead, PacketWrite, Protocol};

use crate::packet::status::clientbound::Packet::{Pong, StatusResponse};
use crate::packet::status::serverbound::Packet::{Ping, StatusRequest};
use rustc_serialize::base64::{ToBase64, STANDARD};
use rustc_serialize::json;
use time;

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct Description {
    pub text: String,
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct Players {
    pub max: i32,
    pub online: i32,
    pub sample: Option<Vec<Sample>>,
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct Sample {
    pub name: String,
    pub id: String,
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct Version {
    pub name: String,
    pub protocol: i32,
}

/// Response sent to clients as JSON.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct Response {
    // FIXME(toqueteos): This is ChatJson
    pub description: String,
    pub favicon: Option<String>,
    pub players: Players,
    pub version: Version,
}

impl Protocol for Response {
    type Clean = Response;

    fn proto_len(value: &Response) -> usize {
        <String as Protocol>::proto_len(&json::encode(&value).unwrap())
    }
    fn proto_encode(value: &Response, dst: &mut dyn Write) -> io::Result<()> {
        <String as Protocol>::proto_encode(&json::encode(&value).unwrap(), dst)?;
        Ok(())
    }
    fn proto_decode(src: &mut dyn Read) -> io::Result<Response> {
        let s = <String as Protocol>::proto_decode(src)?;
        println!("Response proto_decode {}", s);
        json::decode(&s).map_err(|_| io::Error::new(InvalidInput, "found invalid JSON"))
    }
}

// FIXME(toqueteos): This is yelling to be a method of a Server struct or
// something more useful. We need the Handshake's `next_state` field in order
// to perform login for a player.
/// Server-side Server List response
pub fn response(stream: &mut TcpStream) -> io::Result<()> {
    use crate::packet::status::clientbound::StatusResponse;
    use crate::packet::status::serverbound::Packet;

    // C->S: Status Request packet
    match Packet::read(stream)? {
        StatusRequest(_) => {
            // S->C: Status Response packet
            let mut file = File::open(&Path::new("assets/favicon.png"))?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;
            let favicon = contents.to_base64(STANDARD);
            // FIXME(toqueteos): Micro-optimization? We could totally drop JSON
            // encoding and just replace player values (online & max) with format! all
            // other values are static.
            let resp = Response {
                version: Version {
                    name: consts::VERSION.to_string(),
                    protocol: consts::PROTO_VERSION,
                },
                players: Players {
                    // FIXME(toqueteos): This value should be an internal counter of server
                    online: 0,
                    // FIXME(toqueteos): This value read from server.properties file
                    max: 20,
                    sample: None,
                },
                description: "With custom favicons! Woot :D".to_string(),
                favicon: Some(format!("data:image/png;base64,{:}", favicon)),
            };
            StatusResponse { response: resp }.write(stream)?;
            Ok(())
        }
        wrong_packet @ Ping(_) => Err(io::Error::new(
            InvalidInput,
            &format!(
                "Invalid packet read, expecting C->S StatusRequest packet, got {:?}",
                wrong_packet
            )[..],
        )),
    }
}

/// Server-side pong response, optional
pub fn pong(stream: &mut TcpStream) -> io::Result<()> {
    use crate::packet::status::clientbound::Pong;
    use crate::packet::status::serverbound::Packet;

    // C->S: Ping packet
    match Packet::read(stream)? {
        Ping(ping) => {
            // S->C: Pong packet
            Pong { time: ping.time }.write(stream)?;
            Ok(())
        }
        wrong_packet @ StatusRequest(_) => Err(io::Error::new(
            InvalidInput,
            &format!(
                "Invalid packet read, expecting C->S Ping packet, got {:?}",
                wrong_packet
            )[..],
        )),
    }
}

/// Client-side Server List request
pub fn request(stream: &mut TcpStream) -> io::Result<Response> {
    use crate::packet::status::clientbound::Packet;
    use crate::packet::status::serverbound::StatusRequest;

    // C->S: Status Request packet
    StatusRequest.write(stream)?;

    // S->C: Status Response packet
    match Packet::read(stream)? {
        StatusResponse(resp) => Ok(resp.response),
        wrong_packet @ Pong(_) => Err(io::Error::new(
            InvalidInput,
            &format!(
                "Invalid packet read, expecting S->C StatusResponse packet, got {:?}",
                wrong_packet
            )[..],
        )),
    }
}

/// Client-side ping request, optional
pub fn ping(stream: &mut TcpStream) -> io::Result<i64> {
    use crate::packet::status::clientbound::Packet;
    use crate::packet::status::serverbound::Ping;

    // C->S: Ping packet
    let start = time::get_time();
    Ping { time: start.sec }.write(stream)?;

    // S->C: Pong packet
    match Packet::read(stream)? {
        Pong(_) => {
            let end = time::get_time();
            let elapsed = end.sub(start).num_milliseconds();
            Ok(elapsed)
        }
        wrong_packet @ StatusResponse(_) => Err(io::Error::new(
            InvalidInput,
            &format!(
                "Invalid packet read, expecting S->C Pong packet, got {:?}",
                wrong_packet
            )[..],
        )),
    }
}

#[allow(unused_imports)]
#[cfg(test)]
mod tests {
    // This module is special, compiler sees imports as they were unused
    // because there's no `vanilla_server_required` cfg set.
    //
    // Unless we tell Travis to run a vanilla server, these tests will
    // only get run if the cfg attr is removed manually.

    use super::*;

    use std::io::prelude::*;
    use std::net::TcpStream;

    use crate::packet::handshake::Handshake;
    use crate::packet::{NextState, PacketWrite};

    #[test]
    #[cfg(vanilla_server_required)]
    fn client_server_list_ping() {
        let mut stream = TcpStream::connect("127.0.0.1:25565").unwrap();
        Handshake {
            proto_version: consts::PROTO_VERSION,
            server_address: "127.0.0.1".to_string(),
            server_port: 25565,
            next_state: NextState::Status,
        }
        .write(&mut stream)
        .unwrap();
        let response = request(&mut stream).unwrap();
        println!("request {:?}", response);
        let elapsed = ping(&mut stream).unwrap();
        println!("ping {}", elapsed);
    }

    #[test]
    #[should_panic]
    #[cfg(vanilla_server_required)]
    fn client_slp_reversed() {
        let mut stream = TcpStream::connect("127.0.0.1:25565").unwrap();
        let elapsed = ping(&mut stream).unwrap();
        println!("ping {}", elapsed);
        Handshake {
            proto_version: consts::PROTO_VERSION,
            server_address: "127.0.0.1".to_string(),
            server_port: 25565,
            next_state: NextState::Status,
        }
        .write(&mut stream)
        .unwrap();
        let response = request(&mut stream).unwrap();
        println!("request {:?}", response);
    }
}
