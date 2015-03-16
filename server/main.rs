extern crate "hematite_server" as hem;

use std::io;

use hem::packet::Protocol;

fn main() {
    let value = false;
    let first_value = vec![0];

    let mut w = Vec::new();
    <bool as Protocol>::proto_encode(&value, &mut w).unwrap();
    assert_eq!(&w, &first_value);

    let mut r = io::Cursor::new(w);
    let value = <bool as Protocol>::proto_decode(&mut r).unwrap();
    assert_eq!(false, value);

    println!("It works!");
}
