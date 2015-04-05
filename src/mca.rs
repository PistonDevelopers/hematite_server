//! MC Region file (.mca) handling.

use std::fs::File;
use std::io::ErrorKind::InvalidInput;
use std::io::prelude::*;
use std::io::{self, SeekFrom};
use std::path::Path;

use byteorder::{BigEndian, ReadBytesExt};
use nbt::{NbtBlob, NbtValue};

pub struct McaFile {
    // locations: [i32; 1024],
    // timestamps: [i32; 1024],
    columns: Vec<McaChunkColumn>
}

impl McaFile {
    pub fn read(path: &Path) -> io::Result<McaFile> {
        let mut file = try!(File::open(path));
        let mut locations = [0i32; 1024];
        let mut timestamps = [0i32; 1024];
        let mut columns = Vec::new();
        // Read first 8KB of file
        for loc in locations.iter_mut() {
            *loc = try!(file.read_i32::<BigEndian>());
        }
        for ts in timestamps.iter_mut() {
            *ts = try!(file.read_i32::<BigEndian>());
        }
        // File can contain up to 1024 chunk columns
        for idx in 0..1024 {
            let loc = locations[idx];
            let ts = timestamps[idx];
            if loc == 0 { // Empty chunk
                // println!("Empty chunk at {},{}", x, z);
                continue;
            }
            let idx = idx as isize;
            let loc = loc as usize;
            let (x, z) = (idx % 32, idx >> 5);
            let offset = loc >> 8;
            let sector_count = loc & 0xff;
            // Apply `offset` before reading `data`
            try!(file.seek(SeekFrom::Start((offset as u64) << 12)));
            // Chunk data: length (i32), compresson (u8), data(&[u8])
            let length = try!(file.read_i32::<BigEndian>());
            let compression = try!(file.read_u8());
            let mut take = (&mut file).take(length as u64 - 1);
            // We could use a channel to read MORE THAN ONE compressed NBT blob at a time.
            let data = match compression {
                0x01 => try!(NbtBlob::from_gzip(&mut take)),
                0x02 => try!(NbtBlob::from_zlib(&mut take)),
                cid => return Err(io::Error::new(InvalidInput, format!("unknown compression scheme 0x{:02x}", cid).as_ref()))
            };
            let chunk_blob = McaChunkBlob {
                x: x,
                z: z,
                offset: offset,
                sector_count: sector_count,
                timestamp: ts,
                length: length,
                compression: compression,
                data: data
            };
            columns.push(try!(chunk_blob.get_mca_chunk_column()));
        }
        println!("McaFile::read {:?} {:4}/1024 ({:02.2})", path, columns.len(), columns.len() as f64 / 1024.0 * 100.0);
        Ok(McaFile {
            // locations: locations,
            // timestamps: timestamps,
            columns: columns
        })
    }

    pub fn write(&self) -> io::Result<()> {
        self.columns
        Ok(())
    }
}

#[derive(Debug)]
pub struct McaChunkBlob {
    x: isize,
    z: isize,
    offset: usize,
    sector_count: usize,
    timestamp: i32,
    compression: u8,
    length: i32,
    data: NbtBlob
}

impl McaChunkBlob {
    // FIXME(toqueteos): Maybe we should call this on `McaFile::read`?
    /// Transforms `self.data` NbtBlob into a `McaChunkColumn` and returns it.
    pub fn get_mca_chunk_column(&self) -> io::Result<McaChunkColumn> {
        let ref level = self.data["Level"];
        if let &NbtValue::Compound(ref c) = level {
            let x = match c.get("xPos").unwrap() {
                &NbtValue::Int(value) => value,
                _ => return Err(io::Error::new(InvalidInput, "xPos not an Int"))
            };
            let z = match c.get("zPos").unwrap() {
                &NbtValue::Int(value) => value,
                _ => return Err(io::Error::new(InvalidInput, "zPos not an Int"))
            };
            let light_populated = match c.get("LightPopulated").unwrap() {
                &NbtValue::Byte(value) => value,
                _ => return Err(io::Error::new(InvalidInput, "LightPopulated not a Byte"))
            };
            let terrain_populated = match c.get("TerrainPopulated").unwrap() {
                &NbtValue::Byte(value) => value,
                _ => return Err(io::Error::new(InvalidInput, "TerrainPopulated not a Byte"))
            };
            let inhabited_time = match c.get("InhabitedTime").unwrap() {
                &NbtValue::Long(value) => value,
                _ => return Err(io::Error::new(InvalidInput, "InhabitedTime not a Long"))
            };
            let last_update = match c.get("LastUpdate").unwrap() {
                &NbtValue::Long(value) => value,
                _ => return Err(io::Error::new(InvalidInput, "LastUpdate not a Long"))
            };
            Ok(McaChunkColumn {
                x: x,
                z: z,
                light_populated: light_populated,
                terrain_populated: terrain_populated,
                inhabited_time: inhabited_time,
                last_update: last_update,
                biomes: [0u8; 256],
                height_map: [0i32; 256],
                sections: vec![],
            })
        } else {
            Err(io::Error::new(InvalidInput, "Level not a Compound"))
        }
    }
}

/// Disk file version of ColumnChunk
pub struct McaChunkColumn {
    x: i32,
    z: i32,
    light_populated: i8,
    terrain_populated: i8,
    inhabited_time: i64,
    last_update: i64,
    biomes: [u8; 256],
    height_map: [i32; 256],
    sections: Vec<McaChunk>
}

/// Disk file version of Chunk
pub struct McaChunk {
    pub y: i8,
    pub blocks: [u16; 4096],
    pub block_light: [u8; 2048],
    pub sky_light: [u8; 2048],
    pub data: [u8; 2048],
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use vanilla;

    use time;

    // FIXME(toqueteos): This could be a #[bench] function or some kind of test
    // once we have a world generator. Also, too many .unwrap calls!
    #[test]
    fn test_mcafile_read() {
        let mut path = vanilla::root_path();
        path.push("saves");

        let mut mca_files = vec![];
        for entry in fs::walk_dir(&path).unwrap() {
            let entry = entry.unwrap();
            match entry.path().extension() {
                Some(ext) => { if ext != "mca" { continue } }
                None => continue
            }
            println!("file: {:?}", entry.path());
            // Naive way of measuring time
            let start = time::get_time();
            let mca = McaFile::read(&entry.path()).unwrap();
            let end = time::get_time();
            mca_files.push(mca);
            let elapsed = (end - start).num_milliseconds();
            println!("elapsed {}ms", elapsed);
        }
    }
}
