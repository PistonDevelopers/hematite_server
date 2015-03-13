#![feature(core)]
#![feature(fs)]
#![feature(io)]
#![feature(path)]
#![feature(test)]

extern crate byteorder;
extern crate flate2;
extern crate nbt;
extern crate uuid;
extern crate test;

pub mod packet;
pub mod proto;
pub mod types;
mod util;
