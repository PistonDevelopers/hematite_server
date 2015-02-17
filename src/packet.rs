//! MC Protocol packets

use std::old_io::{ IoError, IoErrorKind, IoResult };

pub trait Protocol {
    type Clean = Self;
    fn proto_len(value: Self::Clean) -> usize;
    fn proto_encode(value: Self::Clean, dst: &mut Writer) -> IoResult<()>;
    fn proto_decode(src: &mut Reader, len: usize) -> IoResult<Self::Clean>;
}

macro_rules! packet {
    // Regular packets
    ($name:ident ($id:expr) { $($fname:ident: $lty:ty => $rty:ty),+ }) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $fname: $rty),*
        }

        impl Protocol for $name {
            type Clean = $name;
            fn proto_len(value: $name) -> usize {
                0 $(+ Protocol::proto_len(value.$fname))*
            }
            fn proto_encode(value: $name, dst: &mut Writer) -> IoResult<()> {
                let len = 1 + value.proto_len();
                try!(<VarInt as Protocol>::proto_encode(len as i32, dst));
                try!(<VarInt as Protocol>::proto_encode($id, dst));
                $(try!(<$lty as Protocol>::proto_encode(value.$fname, dst));)*
                // println!("proto_encode name={} id={:x} length={}", stringify!($name), $id, len);
                Ok(())
            }
            #[allow(unused_variables)]
            fn proto_decode(src: &mut Reader, len: usize) -> IoResult<$name> {
                let len: i32 = try!(<VarInt as Protocol>::proto_decode(src, len));
                let id: i32 = try!(<VarInt as Protocol>::proto_decode(src, len));
                // println!("proto_decode name={} id={:x} length={}", stringify!($name), id, len);
                if id != $id {
                    return Err(IoError {
                        kind: IoErrorKind::InvalidInput,
                        desc: "unexpected packet",
                        detail: Some(format!("Expected packet id #{:x}, got #{:x} instead.", $id, id))
                    });
                }
                Ok($name {
                    $($fname: try!(<$lty as Protocol>::proto_decode(src, len))),*
                })
            }
        }
    };
    // No field packets
    ($name:ident ($id:expr) {}) => {
        #[derive(Debug)]
        pub struct $name;

        impl Protocol for $name {
            type Clean = $name;
            #[allow(unused_variables)]
            fn proto_len(value: $name) -> usize { 0 }
            fn proto_encode(value: $name, dst: &mut Writer) -> IoResult<()> {
                let len = 1 + value.proto_len();
                try!(Protocol::proto_encode(VarInt::encode(len as i32), dst));
                try!(Protocol::proto_encode(VarInt::encode($id), dst));
                // println!("proto_encode name={} id={:x} length={}", stringify!($name), $id, len);
                Ok(())
            }
            #[allow(unused_variables)]
            fn proto_decode(src: &mut Reader, len: usize) -> IoResult<$name> {
                let len: i32 = try!(<VarInt as Protocol>::proto_decode(src, len));
                let id: i32 = try!(<VarInt as Protocol>::proto_decode(src, len));
                // println!("proto_decode name={} id={:x} length={}", stringify!($name), id, len);
                if id != $id {
                    return Err(IoError {
                        kind: IoErrorKind::InvalidInput,
                        desc: "unexpected packet",
                        detail: Some(format!("Expected packet id #{:x}, got #{:x} instead.", $id, id))
                    });
                }
                Ok($name)
            }
        }
    }
}

macro_rules! packets {
    ($($id:expr => $name:ident {$($packet:tt)*})*) => {
        $(packet!{ $name ($id) { $($packet)* } })*
    }
}

macro_rules! impl_protocol {
    ($name:ty, $len:expr, $enc_name:ident, $dec_name:ident) => {
        impl Protocol for $name {
            type Clean = $name;
            #[allow(unused_variables)]
            fn proto_len(value: $name) -> usize { $len }
            fn proto_encode(value: $name, dst: &mut Writer) -> IoResult<()> {
                try!(dst.$enc_name(value));
                Ok(())
            }
            #[allow(unused_variables)]
            fn proto_decode(src: &mut Reader, len: usize) -> IoResult<$name> {
                Ok(try!(src.$dec_name()))
            }
        }
    }
}

impl_protocol!(i8,  1, write_i8,     read_i8);
impl_protocol!(u8,  1, write_u8,     read_u8);
impl_protocol!(i16, 2, write_be_i16, read_be_i16);
impl_protocol!(u16, 2, write_be_u16, read_be_u16);
impl_protocol!(i32, 4, write_be_i32, read_be_i32);
impl_protocol!(u32, 4, write_be_u32, read_be_u32);
impl_protocol!(i64, 8, write_be_i64, read_be_i64);
impl_protocol!(u64, 8, write_be_u64, read_be_u64);
impl_protocol!(f32, 4, write_be_f32, read_be_f32);
impl_protocol!(f64, 8, write_be_f64, read_be_f64);

impl Protocol for bool {
    type Clean = bool;
    #[allow(unused_variables)]
    fn proto_len(value: bool) -> usize { 1 }
    fn proto_encode(value: bool, dst: &mut Writer) -> IoResult<()> {
        try!(dst.write_u8(if value { 1 } else { 0 }));
        Ok(())
    }
    #[allow(unused_variables)]
    fn proto_decode(src: &mut Reader, len: usize) -> IoResult<bool> {
        let value = try!(src.read_u8());
        if value > 1 {
            Err(IoError {
                kind: IoErrorKind::InvalidInput,
                desc: "invalid bool value",
                detail: Some(format!("Invalid bool value, expecting 0 or 1, got {}", value))
            })
        } else {
            Ok(value == 1)
        }
    }
}
