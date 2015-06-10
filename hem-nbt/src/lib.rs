//! MC Named Binary Tag type.

#![feature(core)]
#![cfg_attr(test, feature(test))]

extern crate byteorder;
extern crate flate2;
#[cfg(test)] extern crate test;

/* Re-export the core API from submodules. */
pub use blob::NbtBlob;
pub use error::NbtError;
pub use value::NbtValue;

mod blob;
mod error;
mod value;
#[cfg(test)] mod tests;
