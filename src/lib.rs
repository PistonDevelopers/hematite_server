#![deny(
    unused,
    rust_2018_compatibility,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style,
    missing_debug_implementations
)]
#![warn(clippy::all, clippy::pedantic, missing_copy_implementations)]
#![allow(
    clippy::module_name_repetitions,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::identity_op
)]

#[macro_use]
extern crate log;
pub use nbt;

pub mod consts;
pub mod packet;
pub mod proto;
pub mod types;
mod util;
pub mod vanilla;
pub mod world;
