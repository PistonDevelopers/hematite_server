#![cfg_attr(test, deny(missing_docs, warnings))]
#![forbid(unused_variables)]
// #![feature(associated_type_defaults)]
// #![feature(read_exact)]

extern crate byteorder;
extern crate flate2;
#[macro_use]
extern crate log;
extern crate nbt;
extern crate num;
extern crate rand;
extern crate regex;
extern crate rustc_serialize;
extern crate time;
extern crate uuid;

pub mod packet;
pub mod proto;
pub mod types;
mod util;
pub mod vanilla;
pub mod world;
