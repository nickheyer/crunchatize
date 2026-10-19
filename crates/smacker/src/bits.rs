use crate::Error;

/// Reads bits least significant first from each successive byte
pub(crate) struct BitReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> BitReader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub(crate) fn bit(&mut self) -> Result<bool, Error> {
        let byte = *self.data.get(self.position >> 3).ok_or(Error::ShortBitstream)?;
        let bit = (byte >> (self.position & 7)) & 1;
        self.position += 1;
        Ok(bit == 1)
    }

    /// Returns up to 32 bits with the first bit read in the lowest position
    pub(crate) fn bits(&mut self, count: u32) -> Result<u32, Error> {
        let mut value = 0;
        for shift in 0..count {
            if self.bit()? {
                value |= 1 << shift;
            }
        }
        Ok(value)
    }
}
