//! A protocol implementation for `nbt::Blob`s.

use std::io;

use nbt;

use packet::Protocol;

impl Protocol for nbt::Blob {
    type Clean = nbt::Blob;

    fn proto_len(value: &nbt::Blob) -> usize {
        value.len()
    }

    fn proto_encode(value: &nbt::Blob, dst: &mut io::Write) -> io::Result<()> {
        Ok(try!(value.write(dst)))
    }

    fn proto_decode(src: &mut io::Read) -> io::Result<nbt::Blob> {
        Ok(try!(nbt::Blob::from_reader(src)))
    }
}
