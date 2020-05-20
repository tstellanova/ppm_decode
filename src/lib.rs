use arraydeque::{ArrayDeque, Wrapping};

#![no_std]

type PpmValue = u16;
const PPM_MIN_VAL: PpmValue = 1000;
const PPM_MAX_VAL: PpmValue = 2000;
const PPM_MID_VAL: PpmValue = (PPM_MAX_VAL + PPM_MIN_VAL) / 2;

/// theoretical maximum PPM channels in normal usage
const MAX_PPM_CHANNELS: usize = 16;

pub struct PpmFrame {
    /// Decoded PPM channel values
    chan_values: [PpmValue; MAX_PPM_CHANNELS],
    /// Length of decoded PPM frame
    frame_length: u16,
    /// Number of channels decoded (may not be the same as MAX_PPM_CHANNELS)
    decoded_chan_count: u8,
    /// Timestamp of this frame decode
    timestamp: u64,
}

pub struct PpmParser {

}

impl PpmParser {
    pub fn new() -> Self {
        Self {

        }
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) {
        unimplemented!()
    }

    pub fn next_frame() -> Result<Option<PpmFrame>, Error> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
