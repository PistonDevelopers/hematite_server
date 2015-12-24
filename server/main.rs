extern crate hematite_server as hem;
#[macro_use]
extern crate log;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::sync::mpsc;

use hem::vanilla::Server;

use log::{LogLevel, LogLevelFilter, LogMetadata, LogRecord, SetLoggerError};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Info
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
}

fn init_logger() -> Result<(), SetLoggerError> {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Info);
        Box::new(SimpleLogger)
    })
}

fn main() {
    init_logger().expect("failed to initialize logger");

    info!("hematite server");

    let server = Server::new().expect("failed new server");

    let listener = TcpListener::bind(&(server.addr(), server.port())).expect("failed tcp bind");
    // NOTE(toqueteos): As soon as we need &mut server reference this won't work
    let server_ref = Arc::new(server);

    let (keep_alive_tx, keep_alive_rx) = mpsc::channel();
    thread::spawn(move || Server::keep_alive_loop(keep_alive_rx));

    // Accept connections and process them, spawning a new thread for each one
    for conn in listener.incoming() {
        match conn {
            Ok(conn) => {
                let srv = server_ref.clone();
                let keep_alive_tx = keep_alive_tx.clone();
                thread::spawn(move || {
                    match srv.handle(conn, keep_alive_tx) {
                        Ok(_) => {}
                        Err(err) => info!("{}", err),
                    }
                });
            }
            Err(err) => info!("Connection error {:?}", err),
        }
    }
}
