//! MC Protocol Chunk data types.

use std::io::{self, Cursor};
use std::io::prelude::*;

use packet::play::clientbound::ChunkData;
use packet::Protocol;
use types::consts::Dimension;
use util::ReadExactExt;

/// ChunkColumn is a set of 0-16 chunks, up to 16x256x16 blocks.
pub struct ChunkColumn {
    pub chunks: Vec<Chunk>,
    pub biomes: Option<[u8; 256]>
}

impl ChunkColumn {
    pub fn len(&self) -> usize {
        use std::iter::AdditiveIterator;

        let chunks = self.chunks.iter().map(|x| x.len()).sum();
        let biomes = match self.biomes {
            Some(_) => 256,
            None => 0
        };
        chunks + biomes
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut dst: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        for chunk in self.chunks.iter() {
            for x in chunk.blocks.iter() {
                try!(<u16 as Protocol>::proto_encode(x, &mut dst));
            }
        }
        for chunk in self.chunks.iter() {
            try!(dst.write_all(&chunk.block_light));
        }
        for chunk in self.chunks.iter() {
            match chunk.sky_light {
                Some(xs) => try!(dst.write_all(&xs)),
                None => {}
            }
        }
        match self.biomes {
            Some(xs) => try!(dst.write_all(&xs)),
            None => {}
        }
        Ok(dst.into_inner())
    }
    pub fn decode(packet: ChunkData, dimension: Dimension) -> io::Result<ChunkColumn> {
        use std::num::Int;

        let mut src = Cursor::new(packet.chunk_data);
        let num_chunks = packet.mask.count_ones();
        let mut chunks = Vec::new();
        // NOTE: vec![Chunk::empty(); num_chunks as usize] won't work
        for _ in 0..num_chunks {
            chunks.push(Chunk::empty());
        }
        let mut column = ChunkColumn{
            chunks: chunks,
            biomes: None
        };
        for chunk in column.chunks.iter_mut() {
            for x in chunk.blocks.iter_mut() {
                *x = try!(<u16 as Protocol>::proto_decode(&mut src));
            }
        }
        for chunk in column.chunks.iter_mut() {
            // We use this instead of read_exact because it's an array, Vec is useless here.
            for x in chunk.block_light.iter_mut() {
                *x = try!(<u8 as Protocol>::proto_decode(&mut src));
            }
        }
        for chunk in column.chunks.iter_mut() {
            if dimension == Dimension::Overworld {
                // We use this instead of read_exact because it's an array, Vec is useless here.
                let mut sl = [0u8; 2048];
                for x in sl.iter_mut() {
                    *x = try!(<u8 as Protocol>::proto_decode(&mut src));
                }
                chunk.sky_light = Some(sl);
            }
        }
        if packet.continuous {
            let biomes = try!(src.read_exact(256));
            // Vec<u8> -> [u8; 256]
            let mut bs = [0u8; 256];
            for (idx, elt) in biomes.into_iter().enumerate() {
                bs[idx] = elt;
            }
            column.biomes = Some(bs)
        }
        Ok(column)
    }
}

/// Chunk is a group of 16x16x16 blocks.
///
/// `block_light`, `sky_light` are nibble arrays (4bit values)
#[derive(Copy)]
pub struct Chunk {
    pub blocks: [u16; 4096],
    pub block_light: [u8; 2048],
    pub sky_light: Option<[u8; 2048]>,
}

impl Chunk {
    pub fn len(&self) -> usize {
        let sky = match self.sky_light {
            Some(_) => 2048,
            None => 0
        };
        8192 + 2048 + sky
    }
    pub fn empty() -> Chunk {
        Chunk {
            blocks: [0u16; 4096],
            block_light: [0u8; 2048],
            sky_light: None
        }
    }
    pub fn new(block: u16, light: u8) -> Chunk {
        Chunk {
            blocks: [block; 4096],
            block_light: [light; 2048],
            sky_light: Some([light; 2048])
        }
    }
}
