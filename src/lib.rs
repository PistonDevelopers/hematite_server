#![feature(core, step_by, test)]
#![cfg_attr(test, deny(missing_docs, warnings))]

extern crate byteorder;
extern crate flate2;
extern crate nbt;
extern crate rustc_serialize;
extern crate test;
extern crate time;
extern crate uuid;

pub mod packet;
pub mod proto;
pub mod types;
mod util;
