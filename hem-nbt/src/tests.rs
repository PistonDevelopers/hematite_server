use std::collections::HashMap;
use std::io;
use std::fs::File;

use test::Bencher;

use blob::NbtBlob;
use error::NbtError;
use value::NbtValue;

#[test]
fn nbt_nonempty() {
    let mut nbt = NbtBlob::new("".to_string());
    nbt.insert("name".to_string(),      "Herobrine").unwrap();
    nbt.insert("health".to_string(),    100i8).unwrap();
    nbt.insert("food".to_string(),      20.0f32).unwrap();
    nbt.insert("emeralds".to_string(),  12345i16).unwrap();
    nbt.insert("timestamp".to_string(), 1424778774i32).unwrap();

    let bytes = vec![
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

    // Test correct length.
    assert_eq!(bytes.len(), nbt.len());

    // We can only test if the decoded bytes match, since the HashMap does
    // not guarantee order (and so encoding is likely to be different, but
    // still correct).
    let mut src = io::Cursor::new(bytes);
    let file = NbtBlob::from_reader(&mut src).unwrap();
    assert_eq!(&file, &nbt);
}

#[test]
fn nbt_empty_nbtfile() {
    let nbt = NbtBlob::new("".to_string());

    let bytes = vec![
        0x0a,
            0x00, 0x00,
        0x00
    ];

    // Test correct length.
    assert_eq!(bytes.len(), nbt.len());

    // Test encoding.
    let mut dst = Vec::new();
    nbt.write(&mut dst).unwrap();
    assert_eq!(&dst, &bytes);

    // Test decoding.
    let mut src = io::Cursor::new(bytes);
    let file = NbtBlob::from_reader(&mut src).unwrap();
    assert_eq!(&file, &nbt);
}

#[test]
fn nbt_nested_compound() {
    let mut inner = HashMap::new();
    inner.insert("test".to_string(), NbtValue::Byte(123));
    let mut nbt = NbtBlob::new("".to_string());
    nbt.insert("inner".to_string(), NbtValue::Compound(inner)).unwrap();

    let bytes = vec![
        0x0a,
            0x00, 0x00,
            0x0a,
                0x00, 0x05,
                0x69, 0x6e, 0x6e, 0x65, 0x72,
                0x01,
                0x00, 0x04,
                0x74, 0x65, 0x73, 0x74,
                0x7b,
            0x00,
        0x00
    ];

    // Test correct length.
    assert_eq!(bytes.len(), nbt.len());

    // Test encoding.
    let mut dst = Vec::new();
    nbt.write(&mut dst).unwrap();
    assert_eq!(&dst, &bytes);

    // Test decoding.
    let mut src = io::Cursor::new(bytes);
    let file = NbtBlob::from_reader(&mut src).unwrap();
    assert_eq!(&file, &nbt);
}

#[test]
fn nbt_empty_list() {
    let mut nbt = NbtBlob::new("".to_string());
    nbt.insert("list".to_string(), NbtValue::List(Vec::new())).unwrap();

    let bytes = vec![
        0x0a,
            0x00, 0x00,
            0x09,
                0x00, 0x04,
                0x6c, 0x69, 0x73, 0x74,
                0x01,
                0x00, 0x00, 0x00, 0x00,
        0x00
    ];

    // Test correct length.
    assert_eq!(bytes.len(), nbt.len());

    // Test encoding.
    let mut dst = Vec::new();
    nbt.write(&mut dst).unwrap();
    assert_eq!(&dst, &bytes);

    // Test decoding.
    let mut src = io::Cursor::new(bytes);
    let file = NbtBlob::from_reader(&mut src).unwrap();
    assert_eq!(&file, &nbt);
}

#[test]
fn nbt_no_root() {
    let bytes = vec![0x00];
    // Will fail, because the root is not a compound.
    assert_eq!(NbtBlob::from_reader(&mut io::Cursor::new(&bytes[..])),
            Err(NbtError::NoRootCompound));
}

#[test]
fn nbt_no_end_tag() {
    let bytes = vec![
        0x0a,
            0x00, 0x00,
            0x09,
                0x00, 0x04,
                0x6c, 0x69, 0x73, 0x74,
                0x01,
                0x00, 0x00, 0x00, 0x00
    ];

    // Will fail, because there is no end tag.
    assert_eq!(NbtBlob::from_reader(&mut io::Cursor::new(&bytes[..])),
            Err(NbtError::IncompleteNbtValue));
}

#[test]
fn nbt_invalid_id() {
    let bytes = vec![
        0x0a,
            0x00, 0x00,
            0x0f, // No tag associated with 0x0f.
                0x00, 0x04,
                0x6c, 0x69, 0x73, 0x74,
                0x01,
        0x00
    ];
    assert_eq!(NbtBlob::from_reader(&mut io::Cursor::new(&bytes[..])),
               Err(NbtError::InvalidTypeId(15)));
}

#[test]
fn nbt_invalid_list() {
    let mut nbt = NbtBlob::new("".to_string());
    let mut badlist = Vec::new();
    badlist.push(NbtValue::Byte(1));
    badlist.push(NbtValue::Short(1));
    // Will fail to insert, because the List is heterogeneous.
    assert_eq!(nbt.insert("list".to_string(), NbtValue::List(badlist)),
               Err(NbtError::HeterogeneousList));
}

#[test]
fn nbt_bad_compression() {
    // These aren't in the zlib or gzip format, so they'll fail.
    let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(NbtBlob::from_gzip(&mut io::Cursor::new(&bytes[..])).is_err());
    assert!(NbtBlob::from_zlib(&mut io::Cursor::new(&bytes[..])).is_err());
}

#[test]
fn nbt_compression() {
    // Create a non-trivial NbtBlob.
    let mut nbt = NbtBlob::new("".to_string());
    nbt.insert("name".to_string(), NbtValue::String("Herobrine".to_string())).unwrap();
    nbt.insert("health".to_string(), NbtValue::Byte(100)).unwrap();
    nbt.insert("food".to_string(), NbtValue::Float(20.0)).unwrap();
    nbt.insert("emeralds".to_string(), NbtValue::Short(12345)).unwrap();
    nbt.insert("timestamp".to_string(), NbtValue::Int(1424778774)).unwrap();

    // Test zlib encoding/decoding.
    let mut zlib_dst = Vec::new();
    nbt.write_zlib(&mut zlib_dst).unwrap();
    let zlib_file = NbtBlob::from_zlib(&mut io::Cursor::new(zlib_dst)).unwrap();
    assert_eq!(&nbt, &zlib_file);

    // Test gzip encoding/decoding.
    let mut gzip_dst = Vec::new();
    nbt.write_gzip(&mut gzip_dst).unwrap();
    let gz_file = NbtBlob::from_gzip(&mut io::Cursor::new(gzip_dst)).unwrap();
    assert_eq!(&nbt, &gz_file);
}

#[test]
fn nbt_bigtest() {
    let mut bigtest_file = File::open("../tests/big1.nbt").unwrap();
    let bigtest = NbtBlob::from_gzip(&mut bigtest_file).unwrap();
    // This is a pretty indirect way of testing correctness.
    assert_eq!(1544, bigtest.len());
}

#[bench]
fn nbt_bench_bigwrite(b: &mut Bencher) {
    let mut file = File::open("../tests/big1.nbt").unwrap();
    let nbt = NbtBlob::from_gzip(&mut file).unwrap();
    b.iter(|| {
        nbt.write(&mut io::sink())
    });
}

#[bench]
fn nbt_bench_smallwrite(b: &mut Bencher) {
    let mut file = File::open("../tests/small4.nbt").unwrap();
    let nbt = NbtBlob::from_reader(&mut file).unwrap();
    b.iter(|| {
        nbt.write(&mut io::sink())
    });
}