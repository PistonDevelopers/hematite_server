//! A protocol implementation for `NbtBlob`s.

use std::io;
use nbt::NbtBlob;
use packet::Protocol;

impl Protocol for NbtBlob {
    type Clean = NbtBlob;

    fn proto_len(value: &NbtBlob) -> usize {
        value.len()
    }

    fn proto_encode(value: &NbtBlob, mut dst: &mut io::Write) -> io::Result<()> {
        Ok(try!(value.write(dst)))
    }

    fn proto_decode(mut src: &mut io::Read) -> io::Result<NbtBlob> {
        Ok(try!(NbtBlob::from_reader(src)))
    }
}
