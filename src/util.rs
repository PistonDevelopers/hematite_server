use std::io;
use std::io::prelude::*;

pub trait ReadExactExt: Read {
    /// Returns a `Vec<u8>` containing the next `len` bytes in the reader.
    ///
    /// Adapted from `byteorder::read_full`.
    fn read_exact(&mut self, len: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; len];
        let mut n_read = 0usize;
        while n_read < buf.len() {
            match try!(self.read(&mut buf[n_read..])) {
                0 => { return Err(io::Error::new(io::ErrorKind::InvalidInput, "unexpected EOF", None)); }
                n => n_read += n
            }
        }
        Ok(buf)
    }
}

impl<R: Read> ReadExactExt for R {}
