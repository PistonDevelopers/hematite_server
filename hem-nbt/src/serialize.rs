use std::io;

use byteorder::{ByteOrder, BigEndian, WriteBytesExt};

use error::NbtError;

pub trait NbtFmt {
    fn write_nbt_fmt<W>(&self, dst: &mut W) -> Result<(), NbtError>
       where W: io::Write;

    #[inline]
    fn write_nbt_fmt_with_name<W, S>(&self, dst: &mut W, name: S) -> Result<(), NbtError>
       where W: io::Write,
             S: AsRef<str> {

        try!(dst.write_u8(Self::tag()));
        try!(write_bare_string(dst, name.as_ref()));
        self.write_nbt_fmt(dst)
    }
    
    #[inline] fn tag() -> u8 { 0x0a }
    #[inline] fn is_bare() -> bool { false }
}

macro_rules! nbtfmt_value {
  ($T:ty, $method:ident, $tag:expr, $bare:expr) => (
    impl NbtFmt for $T {
      fn write_nbt_fmt<W>(&self, dst: &mut W) -> Result<(), NbtError>
           where W: io::Write {
            $method(dst, *self)
      }
        #[inline] fn tag() -> u8 { $tag }
        #[inline] fn is_bare() -> bool { $bare }
    }
  )
}

macro_rules! nbtfmt_ptr {
  ($T:ty, $method:ident, $tag:expr, $bare:expr) => (
    impl NbtFmt for $T {
      fn write_nbt_fmt<W>(&self, dst: &mut W) -> Result<(), NbtError>
           where W: io::Write {
            $method(dst, self)
      }
        #[inline] fn tag() -> u8 { $tag }
        #[inline] fn is_bare() -> bool { $bare }
    }
  )
}

macro_rules! nbtfmt_slice {
  ($T:ty, $method:ident, $tag:expr, $bare:expr) => (
    impl NbtFmt for $T {
      fn write_nbt_fmt<W>(&self, dst: &mut W) -> Result<(), NbtError>
           where W: io::Write {
            $method(dst, &self[..])
      }
        #[inline] fn tag() -> u8 { $tag }
        #[inline] fn is_bare() -> bool { $bare }
    }
  )
}

nbtfmt_value!(i8, write_bare_byte, 0x01, true);
nbtfmt_value!(i16, write_bare_short, 0x02, true);
nbtfmt_value!(i32, write_bare_int, 0x03, true);
nbtfmt_value!(i64, write_bare_long, 0x04, true);
nbtfmt_value!(f32, write_bare_float, 0x05, true);
nbtfmt_value!(f64, write_bare_double, 0x06, true);
nbtfmt_ptr!([i8], write_bare_byte_array, 0x07, true);
nbtfmt_slice!(Vec<i8>, write_bare_byte_array, 0x07, true);
nbtfmt_ptr!(str, write_bare_string, 0x08, true);
nbtfmt_slice!(String, write_bare_string, 0x08, true);
nbtfmt_ptr!([i32], write_bare_int_array, 0x0b, true);
nbtfmt_slice!(Vec<i32>, write_bare_int_array, 0x0b, true);

// impl<T> NbtFmt for [T] where T: NbtFmt {
//  fn write_nbt_fmt<W>(&self, dst: &mut W) -> Result<(), NbtError>
//        where W: io::Write {
        
//          write_bare_list(dst, self.iter())
//  }
//     #[inline] fn tag() -> u8 { 0x09 }
//     #[inline] fn is_bare() -> bool { true }
// }

// impl<T> NbtFmt for Vec<T> where T: NbtFmt {
//  fn write_nbt_fmt<W>(&self, dst: &mut W) -> Result<(), NbtError>
//        where W: io::Write {
        
//          write_bare_list(dst, self.iter())
//  }
//     #[inline] fn tag() -> u8 { 0x09 }
//     #[inline] fn is_bare() -> bool { true }
// }

#[inline]
fn write_bare_byte<W>(dst: &mut W, value: i8) -> Result<(), NbtError>
   where W: io::Write {

    dst.write_i8(value).map_err(From::from)
}

#[inline]
fn write_bare_short<W>(dst: &mut W, value: i16) -> Result<(), NbtError>
   where W: io::Write {

    dst.write_i16::<BigEndian>(value).map_err(From::from)
}

#[inline]
fn write_bare_int<W>(dst: &mut W, value: i32) -> Result<(), NbtError>
   where W: io::Write {

    dst.write_i32::<BigEndian>(value).map_err(From::from)
}

#[inline]
fn write_bare_long<W>(dst: &mut W, value: i64) -> Result<(), NbtError>
   where W: io::Write {

    dst.write_i64::<BigEndian>(value).map_err(From::from)
}

#[inline]
fn write_bare_float<W>(dst: &mut W, value: f32) -> Result<(), NbtError>
   where W: io::Write {

    dst.write_f32::<BigEndian>(value).map_err(From::from)
}

#[inline]
fn write_bare_double<W>(dst: &mut W, value: f64) -> Result<(), NbtError>
   where W: io::Write {

    dst.write_f64::<BigEndian>(value).map_err(From::from)
}

#[inline]
fn write_bare_byte_array<W>(dst: &mut W, value: &[i8]) -> Result<(), NbtError>
   where W: io::Write {

    try!(dst.write_i32::<BigEndian>(value.len() as i32));
    for &v in value {
        try!(dst.write_i8(v));
    }
    Ok(())
}

#[inline]
fn write_bare_int_array<W>(dst: &mut W, value: &[i32]) -> Result<(), NbtError>
   where W: io::Write {

    try!(dst.write_i32::<BigEndian>(value.len() as i32));
    for &v in value {
        try!(dst.write_i32::<BigEndian>(v));
    }
    Ok(())
}

#[inline]
fn write_bare_string<W>(dst: &mut W, value: &str) -> Result<(), NbtError>
   where W: io::Write {
    
    try!(dst.write_u16::<BigEndian>(value.len() as u16));
    dst.write_all(value.as_bytes()).map_err(From::from)
}

#[inline]
fn write_bare_list<'a, W, I, T>(dst: &mut W, values: I) -> Result<(), NbtError>
   where W: io::Write,
         I: Iterator<Item=&'a T> + ExactSizeIterator,
         T: 'a + NbtFmt {

    // The list contents are prefixed by a byte tag for the type and the
    // length of the list (a big-endian i32).
    try!(dst.write_u8(T::tag()));
    try!(dst.write_i32::<BigEndian>(values.len() as i32));

    for ref value in values {
       try!(value.write_nbt_fmt(dst));
    }

    Ok(())
}

#[inline]
fn write_bare_compound<'a, W, I, T, S>(dst: &mut W, values: I) -> Result<(), NbtError>
   where W: io::Write,
         I: Iterator<Item=(&'a S, &'a T)>,
         S: 'a + AsRef<str>,
         T: 'a + NbtFmt {

    for (key, ref value) in values {
        try!(value.write_nbt_fmt_with_name(dst, key.as_ref()));
    }
    
    // Write the marker for the end of the Compound.
    dst.write_u8(0x00).map_err(From::from)
}

#[test]
fn nbt_test_struct_serialize() {
  struct TestStruct {
    name: String,
    health: i8,
    food: f32,
    emeralds: i16,
    timestamp: i32
  }

  impl NbtFmt for TestStruct {
    fn write_nbt_fmt<W>(&self, dst: &mut W) -> Result<(), NbtError>
           where W: io::Write {

            self.name.write_nbt_fmt_with_name(dst, "name");
            self.health.write_nbt_fmt_with_name(dst, "health");
            self.food.write_nbt_fmt_with_name(dst, "food");
            self.emeralds.write_nbt_fmt_with_name(dst, "emeralds");
            self.timestamp.write_nbt_fmt_with_name(dst, "timestamp");

            dst.write_u8(0x00).map_err(From::from)
        }
  }

  let test = TestStruct {
    name: "Herobrine".to_string(),
    health: 100, food: 20.0, emeralds: 12345, timestamp: 1424778774
  };

  let mut test_encoded = Vec::new();
  test.write_nbt_fmt_with_name(&mut test_encoded, "");

  let bytes = [
        0x0a,
            0x00, 0x00,
            0x08,
                0x00, 0x04,
                0x6e, 0x61, 0x6d, 0x65,
                0x00, 0x09,
                0x48, 0x65, 0x72, 0x6f, 0x62, 0x72, 0x69, 0x6e, 0x65,
            0x01,
                0x00, 0x06,
                0x68, 0x65, 0x61, 0x6c, 0x74, 0x68,
                0x64,
            0x05,
                0x00, 0x04,
                0x66, 0x6f, 0x6f, 0x64,
                0x41, 0xa0, 0x00, 0x00,
            0x02,
                0x00, 0x08,
                0x65, 0x6d, 0x65, 0x72, 0x61, 0x6c, 0x64, 0x73,
                0x30, 0x39,
            0x03,
                0x00, 0x09,
                0x74, 0x69, 0x6d, 0x65, 0x73, 0x74, 0x61, 0x6d, 0x70,
                0x54, 0xec, 0x66, 0x16,
        0x00
    ];

    assert_eq!(&bytes[..], &test_encoded[..]);
}
