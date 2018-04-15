extern crate hematite_server as hem;
#[macro_use]
extern crate log;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use hem::vanilla::Server;

use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

static SIMPLE_LOGGER: SimpleLogger = SimpleLogger;

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

fn init_logger() -> Result<(), SetLoggerError> {
    log::set_logger(&SIMPLE_LOGGER)?;
    log::set_max_level(LevelFilter::Info);
    Ok(())
}

fn main() {
    init_logger().expect("failed to initialize logger");

    info!("hematite server");

    let server = Server::new().expect("failed new server");

    let listener = TcpListener::bind(&(server.addr(), server.port())).expect("failed tcp bind");
    // NOTE(toqueteos): As soon as we need &mut server reference this won't work
    let server_ref = Arc::new(server);
    // Accept connections and process them, spawning a new tasks for each one
    for conn in listener.incoming() {
        match conn {
            Ok(conn) => {
                let srv = server_ref.clone();
                thread::spawn(move || match srv.handle(conn) {
                    Ok(_) => {}
                    Err(err) => info!("{}", err),
                });
            }
            Err(e) => info!("Connection error {:?}", e),
        }
    }
}
