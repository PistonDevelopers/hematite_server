//! MC Protocol constants.

use std::io::prelude::*;
use std::io;
use std::num::FromPrimitive;

use packet::Protocol;

macro_rules! enum_protocol_impl {
    ($name:ty, $repr:ty, $dec_repr:ident) => {
        impl Protocol for $name {
            type Clean = $name;

            #[allow(unused_variables)]
            fn proto_len(value: &$name) -> usize { <$repr as Protocol>::proto_len(&(*value as $repr)) }

            fn proto_encode(value: &$name, mut dst: &mut Write) -> io::Result<()> {
                let repr = *value as $repr;
                try!(<$repr as Protocol>::proto_encode(&repr, dst));
                Ok(())
            }

            fn proto_decode(mut src: &mut Read) -> io::Result<$name> {
                let value = try!(<$repr as Protocol>::proto_decode(src));
                match FromPrimitive::$dec_repr(value) {
                    Some(x) => Ok(x),
                    None => Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid enum", None))
                }
            }
        }
    }
}

enum_protocol_impl!(Dimension, i8, from_i8);

#[repr(i8)]
#[derive(Copy, Debug, FromPrimitive, PartialEq)]
pub enum Dimension {
    Nether = -1,
    Overworld = 0,
    End = 1
}

#[derive(Copy, Debug, Clone, Eq, PartialEq)]
pub enum Color {
    Black       = 0x0,
    DarkBlue    = 0x1,
    DarkGreen   = 0x2,
    DarkCyan    = 0x3,
    DarkRed     = 0x4,
    Purple      = 0x5,
    Gold        = 0x6,
    Gray        = 0x7,
    DarkGray    = 0x8,
    Blue        = 0x9,
    BrightGreen = 0xa,
    Cyan        = 0xb,
    Red         = 0xc,
    Pink        = 0xd,
    Yellow      = 0xe,
    White       = 0xf
}

impl Color {
    pub fn to_string(&self) -> String {
        match self {
            &Color::Black => "black".to_string(),
            &Color::DarkBlue => "dark_blue".to_string(),
            &Color::DarkGreen => "dark_green".to_string(),
            &Color::DarkCyan => "dark_aqua".to_string(),
            &Color::DarkRed => "dark_red".to_string(),
            &Color::Purple => "dark_purple".to_string(),
            &Color::Gold => "gold".to_string(),
            &Color::Gray => "gray".to_string(),
            &Color::DarkGray => "dark_gray".to_string(),
            &Color::Blue => "blue".to_string(),
            &Color::BrightGreen => "green".to_string(),
            &Color::Cyan => "aqua".to_string(),
            &Color::Red => "red".to_string(),
            &Color::Pink => "light_purple".to_string(),
            &Color::Yellow => "yellow".to_string(),
            &Color::White => "white".to_string()
        }
    }

    pub fn from_string(string: &String) -> Option<Color> {
        match string.as_slice() {
            "black"        => Some(Color::Black),
            "dark_blue"    => Some(Color::DarkBlue),
            "dark_green"   => Some(Color::DarkGreen),
            "dark_aqua"    => Some(Color::DarkCyan),
            "dark_red"     => Some(Color::DarkRed),
            "dark_purple"  => Some(Color::Purple),
            "gold"         => Some(Color::Gold),
            "gray"         => Some(Color::Gray),
            "dark_gray"    => Some(Color::DarkGray),
            "blue"         => Some(Color::Blue),
            "green"        => Some(Color::BrightGreen),
            "aqua"         => Some(Color::Cyan),
            "red"          => Some(Color::Red),
            "light_purple" => Some(Color::Pink),
            "yellow"       => Some(Color::Yellow),
            "white"        => Some(Color::White),
            _              => None
        }
    }
}
