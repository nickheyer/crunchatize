use crate::Error;
use crate::bits::BitReader;
use crate::header::AudioInfo;
use crate::huffman::Tree8;

/// Appends the interleaved samples of one track chunk, with 8-bit samples widened to 16 bits
pub(crate) fn decode(chunk: &[u8], info: AudioInfo, out: &mut Vec<i16>) -> Result<(), Error> {
    if info.compressed {
        return decode_dpcm(chunk, info, out);
    }
    if info.bits16 {
        if !chunk.len().is_multiple_of(2) {
            return Err(Error::BadAudio);
        }
        out.extend(chunk.chunks_exact(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]])));
    } else {
        out.extend(chunk.iter().map(|&byte| widen(byte)));
    }
    Ok(())
}

fn widen(sample: u8) -> i16 {
    (i16::from(sample) - 128) << 8
}

/// Reads the unpacked length, three flag bits, one tree per sample byte, the bases, then the deltas
fn decode_dpcm(chunk: &[u8], info: AudioInfo, out: &mut Vec<i16>) -> Result<(), Error> {
    let unpacked = crate::header::read_u32(chunk, 0).map_err(|_| Error::BadChunk)? as usize;
    let bits = &mut BitReader::new(&chunk[4..]);
    if !bits.bit()? {
        return Ok(());
    }
    let stereo = bits.bit()?;
    let bits16 = bits.bit()?;
    if stereo != info.stereo || bits16 != info.bits16 {
        return Err(Error::BadAudio);
    }
    let channels = 1 + usize::from(stereo);
    let bytes_per_sample = 1 + usize::from(bits16);
    if unpacked == 0 || !unpacked.is_multiple_of(channels * bytes_per_sample) {
        return Err(Error::BadAudio);
    }
    let trees = (0..channels * bytes_per_sample)
        .map(|_| Tree8::read(bits))
        .collect::<Result<Vec<_>, _>>()?;
    let total = unpacked / bytes_per_sample;
    let channel_mask = usize::from(stereo);
    if bits16 {
        let mut predictor = [0u16; 2];
        for channel in (0..channels).rev() {
            let high = bits.bits(8)? as u16;
            let low = bits.bits(8)? as u16;
            predictor[channel] = high << 8 | low;
        }
        out.extend(predictor[..channels].iter().map(|&base| base as i16));
        for index in channels..total {
            let channel = index & channel_mask;
            let low = u16::from(trees[2 * channel].decode(bits)?);
            let high = u16::from(trees[2 * channel + 1].decode(bits)?);
            predictor[channel] = predictor[channel].wrapping_add(high << 8 | low);
            out.push(predictor[channel] as i16);
        }
    } else {
        let mut predictor = [0u8; 2];
        for channel in (0..channels).rev() {
            predictor[channel] = bits.bits(8)? as u8;
        }
        out.extend(predictor[..channels].iter().map(|&base| widen(base)));
        for index in channels..total {
            let channel = index & channel_mask;
            predictor[channel] = predictor[channel].wrapping_add(trees[channel].decode(bits)?);
            out.push(widen(predictor[channel]));
        }
    }
    Ok(())
}
