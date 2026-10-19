use std::fmt;

/// Reasons a Smacker stream fails to open or decode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    BadSignature,
    Truncated,
    Dimensions,
    BinkAudio,
    OversizedTree,
    ShortBitstream,
    BadPalette,
    BadChunk,
    BadAudio,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BadSignature => "signature is neither SMK2 nor SMK4",
            Self::Truncated => "data ends before the declared tables or frames",
            Self::Dimensions => "width or height exceeds 32768",
            Self::BinkAudio => "a track is flagged as Bink audio rather than Smacker audio",
            Self::OversizedTree => "a Huffman tree is larger than its declared size",
            Self::ShortBitstream => "a frame chunk ends before its codes are complete",
            Self::BadPalette => "a palette record is malformed",
            Self::BadChunk => "an audio chunk does not fit inside its frame",
            Self::BadAudio => "audio data disagrees with the track description",
        })
    }
}

impl std::error::Error for Error {}
