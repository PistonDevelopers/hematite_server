use std::io::{Read, Write, Result};
use std::net::TcpStream;
use openssl::crypto::symm::{Crypter, Mode, Type};

pub struct SymmStream {
    stream: TcpStream,
    encrypter: Crypter,
    decrypter: Crypter,
}

impl SymmStream {
    pub fn new(stream: TcpStream, shared_secret: &[u8]) -> SymmStream {
        let encrypter = Crypter::new(Type::AES_128_CFB8);
        let decrypter = Crypter::new(Type::AES_128_CFB8);

        encrypter.init(Mode::Encrypt, shared_secret, shared_secret);
        decrypter.init(Mode::Decrypt, shared_secret, shared_secret);

        SymmStream {
            stream: stream,
            encrypter: encrypter,
            decrypter: decrypter,
        }
    }
}

impl Read for SymmStream {
    fn read(&mut self, mut out: &mut [u8]) -> Result<usize> {
        use std::io;

        let stream = <TcpStream as Read>::by_ref(&mut self.stream);

        let mut cipher = Vec::new();
        try!(stream.take(out.len() as u64).read_to_end(&mut cipher));

        let plain = self.decrypter.update(&cipher[..]);

        io::copy(&mut &plain[..], &mut out).map(|r| r as usize)
    }
}

impl Write for SymmStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        try!(self.stream.write(&self.encrypter.update(buf)[..]));

        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        try!(self.stream.write(&self.encrypter.finalize()[..]));

        self.stream.flush()
    }
}
