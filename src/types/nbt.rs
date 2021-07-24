//! A protocol implementation for `nbt::Blob`s.

use std::io;

use nbt;

use crate::packet::Protocol;

impl Protocol for nbt::Blob {
    type Clean = nbt::Blob;

    fn proto_len(value: &nbt::Blob) -> usize {
        value.len()
    }

    fn proto_encode(value: &nbt::Blob, dst: &mut dyn io::Write) -> io::Result<()> {
        Ok(value.write(dst)?)
    }

    fn proto_decode(src: &mut dyn io::Read) -> io::Result<nbt::Blob> {
        Ok(nbt::Blob::from_reader(src)?)
    }
}
