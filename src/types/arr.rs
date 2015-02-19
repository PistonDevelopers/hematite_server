use std::iter::{ AdditiveIterator, FromIterator };
use std::num::{ NumCast, ToPrimitive };
use std::old_io::{ IoError, IoErrorKind, IoResult };

use packet::Protocol;

struct Arr<L, T>;

impl<L: Protocol, T: Protocol> Protocol for Arr<L, T> where L::Clean: NumCast {
    type Clean = Vec<T::Clean>;

    fn proto_len(value: &Vec<T::Clean>) -> usize {
        let len_len = <L as Protocol>::proto_len(&(<<L as Protocol>::Clean as NumCast>::from(value.len()).unwrap()));
        let len_values = value.iter().map(|elt| <T as Protocol>::proto_len(elt)).sum();
        len_len + len_values
    }

    fn proto_encode(value: Vec<T::Clean>, dst: &mut Writer) -> IoResult<()> {
        let len = try!(<L::Clean as NumCast>::from(value.len()).ok_or(IoError {
            kind: IoErrorKind::InvalidInput,
            desc: "could not convert length of vector to Array length type",
            detail: None
        }));
        try!(<L as Protocol>::proto_encode(len, dst));
        for elt in value {
            try!(<T as Protocol>::proto_encode(elt, dst));
        }
        Ok(())
    }

    fn proto_decode(src: &mut Reader) -> IoResult<Vec<T::Clean>> {
        let len = try!(try!(<L as Protocol>::proto_decode(src)).to_uint().ok_or(IoError {
            kind: IoErrorKind::InvalidInput,
            desc: "could not read length of vector from Array length type",
            detail: None
        }));
        <IoResult<Vec<T::Clean>> as FromIterator<_>>::from_iter((0..len).map(|_| <T as Protocol>::proto_decode(src)))
    }
}
