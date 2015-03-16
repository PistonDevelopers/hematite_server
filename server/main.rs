extern crate hematite_server as hem;
#[macro_use]
extern crate log;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

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

fn init() -> Result<(), SetLoggerError> {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Info);
        Box::new(SimpleLogger)
    })
}

fn main () {
    init().unwrap();

    info!("hematite server");

    let server = Server::new().unwrap();

    let listener = TcpListener::bind(&(server.addr(), server.port())).unwrap();
    // NOTE(toqueteos): As soon as we need &mut server reference this won't work
    let server_ref = Arc::new(server);
    // Accept connections and process them, spawning a new tasks for each one
    for conn in listener.incoming() {
        match conn {
            Ok(conn) => {
                let srv = server_ref.clone();
                thread::spawn(move|| {
                    match srv.handle(conn) {
                        Ok(_) => {}
                        Err(err) => info!("{}", err)
                    }
                });
            }
            Err(e) => info!("Connection error {:?}", e)
        }
    }
}
