use crate::Error;
use crate::bits::BitReader;

const NODE: u32 = 0x8000_0000;

/// Prefix tree with byte leaves, flattened so a node stores the offset of its one branch
pub(crate) struct Tree8 {
    nodes: Vec<u32>,
}

/// Prefix tree with 16-bit leaves plus three leaves that echo recently decoded values
pub(crate) struct BigTree {
    nodes: Vec<u32>,
    recent: [usize; 3],
}

/// Builds the flat node array, an absent tag bit yields a lone zero leaf
fn read_nodes(
    bits: &mut BitReader,
    limit: usize,
    mut leaf: impl FnMut(&mut BitReader, usize) -> Result<u32, Error>,
) -> Result<Vec<u32>, Error> {
    let mut nodes = Vec::new();
    let mut open = Vec::new();
    loop {
        if nodes.len() >= limit {
            return Err(Error::OversizedTree);
        }
        if bits.bit()? {
            open.push(nodes.len());
            nodes.push(NODE);
            continue;
        }
        let index = nodes.len();
        nodes.push(leaf(bits, index)?);
        loop {
            let Some(&parent) = open.last() else {
                return Ok(nodes);
            };
            if nodes[parent] == NODE {
                nodes[parent] = NODE | (nodes.len() - parent) as u32;
                break;
            }
            open.pop();
        }
    }
}

fn walk(nodes: &[u32], bits: &mut BitReader) -> Result<usize, Error> {
    let mut index = 0;
    while nodes[index] & NODE != 0 {
        index += if bits.bit()? {
            (nodes[index] & !NODE) as usize
        } else {
            1
        };
    }
    Ok(index)
}

impl Tree8 {
    pub(crate) fn read(bits: &mut BitReader) -> Result<Self, Error> {
        if !bits.bit()? {
            return Ok(Self { nodes: vec![0] });
        }
        let nodes = read_nodes(bits, 511, |bits, _| bits.bits(8))?;
        bits.bit()?;
        Ok(Self { nodes })
    }

    pub(crate) fn decode(&self, bits: &mut BitReader) -> Result<u8, Error> {
        Ok(self.nodes[walk(&self.nodes, bits)?] as u8)
    }
}

impl BigTree {
    /// Reads both byte trees, three escape codes, and the 16-bit tree bounded by the header size
    pub(crate) fn read(bits: &mut BitReader, size: u32) -> Result<Self, Error> {
        if !bits.bit()? {
            return Ok(Self { nodes: vec![0, 0], recent: [1, 1, 1] });
        }
        let low = Tree8::read(bits)?;
        let high = Tree8::read(bits)?;
        let escapes = [bits.bits(16)?, bits.bits(16)?, bits.bits(16)?];
        let mut recent = [None; 3];
        let mut nodes = read_nodes(bits, (size as usize).div_ceil(4), |bits, index| {
            let value = u32::from(low.decode(bits)?) | u32::from(high.decode(bits)?) << 8;
            match escapes.iter().position(|&escape| escape == value) {
                Some(slot) => {
                    recent[slot] = Some(index);
                    Ok(0)
                }
                None => Ok(value),
            }
        })?;
        bits.bit()?;
        let recent = recent.map(|slot| {
            slot.unwrap_or_else(|| {
                nodes.push(0);
                nodes.len() - 1
            })
        });
        Ok(Self { nodes, recent })
    }

    pub(crate) fn reset(&mut self) {
        for &slot in &self.recent {
            self.nodes[slot] = 0;
        }
    }

    /// Decodes one value and rotates it into the recent leaves when it differs from the newest
    pub(crate) fn decode(&mut self, bits: &mut BitReader) -> Result<u16, Error> {
        let value = self.nodes[walk(&self.nodes, bits)?];
        let [first, second, third] = self.recent;
        if value != self.nodes[first] {
            self.nodes[third] = self.nodes[second];
            self.nodes[second] = self.nodes[first];
            self.nodes[first] = value;
        }
        Ok(value as u16)
    }
}
