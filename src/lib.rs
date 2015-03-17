#![feature(core)]
#![feature(io)]
#![feature(net)]
#![feature(std_misc)]
#![feature(test)]

extern crate byteorder;
extern crate flate2;
extern crate nbt;
extern crate "rustc-serialize" as rustc_serialize;
extern crate test;
extern crate time;
extern crate uuid;

pub mod packet;
pub mod proto;
pub mod types;
mod util;
