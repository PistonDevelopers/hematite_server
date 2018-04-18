//! MC Server List Ping protocol.
//!
//! Reference: http://wiki.vg/Server_List_Ping

use std::fs::File;
use std::io::ErrorKind::InvalidInput;
use std::io::prelude::*;
use std::io;
use std::net::TcpStream;
use std::ops::Sub; // Sub for Timespec
use std::path::Path;

use consts;
use packet::{PacketRead, PacketWrite, Protocol};

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
    fn proto_encode(value: &Response, dst: &mut Write) -> io::Result<()> {
        try!(<String as Protocol>::proto_encode(
            &json::encode(&value).unwrap(),
            dst
        ));
        Ok(())
    }
    fn proto_decode(src: &mut Read) -> io::Result<Response> {
        let s = try!(<String as Protocol>::proto_decode(src));
        println!("Response proto_decode {}", s);
        json::decode(&s).map_err(|_| io::Error::new(InvalidInput, "found invalid JSON"))
    }
}

// FIXME(toqueteos): This is yelling to be a method of a Server struct or
// something more useful. We need the Handshake's `next_state` field in order
// to perform login for a player.
/// Server-side Server List response
pub fn response(stream: &mut TcpStream) -> io::Result<()> {
    use packet::status::serverbound::Packet::{self, StatusRequest};
    use packet::status::clientbound::StatusResponse;

    // C->S: Status Request packet
    match try!(Packet::read(stream)) {
        StatusRequest(_) => {
            // S->C: Status Response packet
            let mut file = try!(File::open(&Path::new("assets/favicon.png")));
            let mut contents = Vec::new();
            try!(file.read_to_end(&mut contents));
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
                    // FIXME(toqueteos): This is value should be a internal counter of server
                    online: 0,
                    // FIXME(toqueteos): This is value read from server.properties file
                    max: 20,
                    sample: None,
                },
                description: "With custom favicons! Woot :D".to_string(),
                favicon: Some(format!("data:image/png;base64,{:}", favicon)),
            };
            try!(StatusResponse { response: resp }.write(stream));
            Ok(())
        }
        wrong_packet => Err(io::Error::new(
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
    use packet::status::clientbound::Pong;
    use packet::status::serverbound::Packet::{self, Ping};

    // C->S: Ping packet
    match try!(Packet::read(stream)) {
        Ping(ping) => {
            // S->C: Pong packet
            try!(Pong { time: ping.time }.write(stream));
            Ok(())
        }
        wrong_packet => Err(io::Error::new(
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
    use packet::status::serverbound::StatusRequest;
    use packet::status::clientbound::Packet::{self, StatusResponse};

    // C->S: Status Request packet
    try!(StatusRequest.write(stream));

    // S->C: Status Response packet
    match try!(Packet::read(stream)) {
        StatusResponse(resp) => Ok(resp.response),
        wrong_packet => Err(io::Error::new(
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
    use packet::status::clientbound::Packet::{self, Pong};
    use packet::status::serverbound::Ping;

    // C->S: Ping packet
    let start = time::get_time();
    try!(Ping { time: start.sec }.write(stream));

    // S->C: Pong packet
    match try!(Packet::read(stream)) {
        Pong(_) => {
            let end = time::get_time();
            let elapsed = end.sub(start).num_milliseconds();
            Ok(elapsed)
        }
        wrong_packet => Err(io::Error::new(
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

    use packet::handshake::Handshake;
    use packet::{NextState, PacketWrite};

    #[test]
    #[cfg(vanilla_server_required)]
    fn client_server_list_ping() {
        let mut stream = TcpStream::connect("127.0.0.1:25565").unwrap();
        Handshake {
            proto_version: consts::PROTO_VERSION,
            server_address: "127.0.0.1".to_string(),
            server_port: 25565,
            next_state: NextState::Status,
        }.write(&mut stream)
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
        }.write(&mut stream)
            .unwrap();
        let response = request(&mut stream).unwrap();
        println!("request {:?}", response);
    }
}
