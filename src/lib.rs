#![feature(core)]
#![feature(fs)]
#![feature(io)]
#![feature(test)]

extern crate byteorder;
extern crate flate2;
extern crate uuid;
extern crate "hem-nbt" as nbt;
extern crate test;

pub mod packet;
pub mod types;
mod util;
