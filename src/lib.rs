#![cfg_attr(test, deny(missing_docs, warnings))]
#![forbid(unused_variables)]

extern crate byteorder;
extern crate flate2;
extern crate nbt;
extern crate num;
extern crate regex;
extern crate rustc_serialize;
extern crate time;
extern crate uuid;

pub mod mca;
pub mod packet;
pub mod proto;
pub mod types;
pub mod util;
pub mod vanilla;
