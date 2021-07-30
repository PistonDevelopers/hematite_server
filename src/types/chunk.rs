//! MC Protocol Chunk data types.

use std::fmt;
use std::io::prelude::*;
use std::io::{self, Cursor};

use crate::packet::Protocol;

/// `ChunkColumn` is a set of 0-16 chunks, up to 16x256x16 blocks.
pub struct ChunkColumn {
    pub chunks: Vec<Chunk>,
    pub biomes: Option<[u8; 256]>,
}

impl ChunkColumn {
    #[must_use]
    pub fn len(&self) -> usize {
        let chunks = self
            .chunks
            .iter()
            .map(Chunk::len)
            .fold(0, |acc, item| acc + item);
        let biomes = match self.biomes {
            Some(_) => 256,
            None => 0,
        };
        chunks + biomes
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        use byteorder::{LittleEndian, WriteBytesExt};

        let mut dst: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        for chunk in &self.chunks {
            for x in chunk.blocks.iter() {
                dst.write_u16::<LittleEndian>(*x)?;
            }
        }
        for chunk in &self.chunks {
            dst.write_all(&chunk.block_light)?;
        }
        for chunk in &self.chunks {
            match chunk.sky_light {
                Some(xs) => dst.write_all(&xs)?,
                None => {}
            }
        }
        match self.biomes {
            Some(xs) => dst.write_all(&xs)?,
            None => {}
        }
        Ok(dst.into_inner())
    }
    pub fn decode(
        src: &mut dyn Read,
        mask: u16,
        continuous: bool,
        sky_light: bool,
    ) -> io::Result<ChunkColumn> {
        let num_chunks = mask.count_ones();
        let mut chunks = Vec::new();
        // NOTE: vec![Chunk::empty(); num_chunks as usize] won't work
        for _ in 0..num_chunks {
            chunks.push(Chunk::default());
        }
        let mut column = ChunkColumn {
            chunks,
            biomes: None,
        };
        for chunk in &mut column.chunks {
            for x in chunk.blocks.iter_mut() {
                *x = <u16 as Protocol>::proto_decode(src)?;
            }
        }
        for chunk in &mut column.chunks {
            // We use this instead of read_exactly because it's an array, Vec is useless here.
            for x in chunk.block_light.iter_mut() {
                *x = <u8 as Protocol>::proto_decode(src)?;
            }
        }
        for chunk in &mut column.chunks {
            // sky_light value varies by packet
            // - 0x21 ChunkData uses `sky_light = dimension == Dimension::Overworld`
            // - 0x26 ChunkDataBulk uses `sky_light = true`
            if sky_light {
                // We use this instead of read_exactly because it's an array, Vec is useless here.
                let mut sl = [0_u8; 2048];
                for x in sl.iter_mut() {
                    *x = <u8 as Protocol>::proto_decode(src)?;
                }
                chunk.sky_light = Some(sl);
            }
        }
        if continuous {
            let mut biomes = [0_u8; 256];
            src.read_exact(&mut biomes)?;
            column.biomes = Some(biomes);
        }
        Ok(column)
    }
}

impl fmt::Debug for ChunkColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ChunkColumn chunks={} biomes={}",
            self.chunks.len(),
            self.biomes.is_some()
        )
    }
}

/// Chunk is a group of 16x16x16 blocks.
///
/// `block_light`, `sky_light` are nibble arrays (4bit values)
#[derive(Copy, Clone)]
pub struct Chunk {
    pub blocks: [u16; 4096],
    pub block_light: [u8; 2048],
    pub sky_light: Option<[u8; 2048]>,
}

impl Chunk {
    #[must_use]
    pub fn len(&self) -> usize {
        let sky = match self.sky_light {
            Some(_) => 2048,
            None => 0,
        };
        8192 + 2048 + sky
    }
    #[must_use]
    pub fn new(block: u16, light: u8) -> Chunk {
        Chunk {
            blocks: [block; 4096],
            block_light: [light; 2048],
            sky_light: Some([light; 2048]),
        }
    }
}

impl Default for Chunk {
    fn default() -> Chunk {
        Chunk {
            blocks: [0_u16; 4096],
            block_light: [0_u8; 2048],
            sky_light: None,
        }
    }
}

impl fmt::Debug for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Chunk blocks=[{}, {}, {}, ..] block_light=[{}, {}, {}, ..] sky_light={}",
            self.blocks[0],
            self.blocks[1],
            self.blocks[2],
            self.block_light[0],
            self.block_light[1],
            self.block_light[2],
            self.sky_light.is_some()
        )
    }
}
