/*
Copyright (c) 2020 Todd Stellanova
LICENSE: BSD3 (see LICENSE file)
*/

#![no_std]

/// Base type for PPM timing
pub type Microseconds = u32;

/// PPM timing values
pub type PpmTime = Microseconds;

/// Default minimum channel value:
/// in practice as low as 800 microseconds
pub const MIN_CHAN_VAL: PpmTime = 1000;
/// Default maximum channel value:
/// in practice typically up to 2200 microseconds
pub const MAX_CHAN_VAL: PpmTime = 2000;
/// Default midpoint channel value
pub const MID_CHAN_VAL: PpmTime = (MAX_CHAN_VAL + MIN_CHAN_VAL) / 2;

/// Default minimum gap between frames (no pulses / inactive)
pub const MIN_SYNC_WIDTH: PpmTime = 2300;

/// Default minimum number of channels per frame
pub const MIN_PPM_CHANNELS: u8 = 5;

/// theoretical maximum PPM channels in normal usage
pub const MAX_PPM_CHANNELS: usize = 20;

/// Errors in this crate
#[derive(Debug)]
pub enum Error {
    /// Generic error
    General,
}

#[derive(Copy, Clone, Debug)]
pub struct PpmFrame {
    /// Decoded PPM channel values
    chan_values: [PpmTime; MAX_PPM_CHANNELS],
    /// Number of channels decoded (≤ MAX_PPM_CHANNELS)
    chan_count: u8,
}

#[derive(Copy, Clone, Debug)]
pub struct ParserConfig {
    /// Configurable minimum channel value
    min_chan_value: PpmTime,

    /// Configurable maximum channel value
    max_chan_value: PpmTime,

    /// Configurable start/reset signal width
    min_sync_width: PpmTime,

    /// Configurable minimum number of channels per valid frame
    min_channels: u8,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            min_chan_value: MIN_CHAN_VAL,
            max_chan_value: MAX_CHAN_VAL,
            min_sync_width: MIN_SYNC_WIDTH,
            min_channels: MIN_PPM_CHANNELS
        }
    }
}


impl PpmParser {
    pub fn new() -> Self {
        Self {
            config: Default::default(),
            working_frame: PpmFrame {
                chan_values: [0; MAX_PPM_CHANNELS],
                chan_count: 0,
            },
            parsed_frame: None,
            state: ParserState::Scanning,
            last_pulse_start: 0,
        }
    }

    /// Configure channel value range
    pub fn set_channel_limits(
        &mut self,
        min: PpmTime,
        max: PpmTime,
    ) -> &mut Self {
        self.config.min_chan_value = min;
        self.config.max_chan_value = max;
        self
    }

    /// Configure duration of frame sync
    pub fn set_sync_width(&mut self, width: PpmTime) -> &mut Self {
        self.config.min_sync_width = width;
        self
    }

    /// Handle a pulse start
    pub fn handle_pulse_start(&mut self, count: PpmTime) {
        let width = count.wrapping_sub(self.last_pulse_start);
        self.last_pulse_start = count;

        match self.state {
            ParserState::Scanning => {
                // assume we've never received any pulses before:
                // detect a long sync/reset gap
                if width >= self.config.min_sync_width {
                    //received sync
                    self.reset_channel_counter();
                    self.state = ParserState::Synced;
                }
            }
            ParserState::Synced => {
                // previous pulse has ended
                if width >= MIN_SYNC_WIDTH {
                    //received sync -- we should be finished with prior frame
                    //TODO verify we receive a consistent number of channels
                    if self.working_frame.chan_count >= self.config.min_channels {
                        self.parsed_frame.replace(self.working_frame);
                    } else {
                        //we didn't get expected minimum number of channels
                        self.parsed_frame = None;
                    }
                    self.reset_channel_counter();
                }
                else {
                    // verify the pulse received is within limits, otherwise resync
                    if width >= self.config.min_chan_value  &&
                        width <= self.config.max_chan_value {
                        self.working_frame.chan_values
                            [self.working_frame.chan_count as usize] = width;
                        self.working_frame.chan_count += 1;
                        //TODO verify we haven't received TOO MANY channels (<MAX_PPM_CHANNELS)
                    } else {
                        // bogus pulse -- resynchronize
                        self.reset_channel_counter();
                        self.state = ParserState::Scanning;
                    }
                }
            }
        }
    }

    /// Get the next available PPM frame, if any
    pub fn next_frame(&mut self) -> Option<PpmFrame> {
        self.parsed_frame.take()
    }

    /// We've either finished receiving all channels
    /// (and have received a sync/reset)
    /// or we received garbage and need to clear our buffers.
    fn reset_channel_counter(&mut self) {
        self.working_frame.chan_count = 0;
    }
}

pub struct PpmParser {
    /// Parser configuration
    config: ParserConfig,

    /// Current parsing state
    state: ParserState,

    /// the last time an (active) pulse started
    last_pulse_start: PpmTime,

    /// working memory for current frame capture
    working_frame: PpmFrame,

    /// frame ready for consumption
    parsed_frame: Option<PpmFrame>,
}

enum ParserState {
    /// we have not yet received a long reset/synchronization
    Scanning,
    /// we've received a sync and are trying to receive pulses
    Synced,
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn process_pulses() {
        const TEST_CHAN_COUNT: u8 = 16;
        const TEST_RESYNC_WIDTH: PpmTime = 2500;
        let mut parser = PpmParser::new();
        parser
            .set_channel_limits(800, 2200)
            .set_sync_width(TEST_RESYNC_WIDTH - 10);

        let mut cur_time: PpmTime = 100;
        //start with a garbage pulse from prior packet
        parser.handle_pulse_start(cur_time);
        //this effectively starts a new packet
        cur_time += TEST_RESYNC_WIDTH;
        let frame = parser.next_frame();
        assert!(frame.is_none(), "there should be no frame yet");

        //send a full frame
        for _ in 0..TEST_CHAN_COUNT + 1 {
            parser.handle_pulse_start(cur_time);
            let frame = parser.next_frame();
            assert!(frame.is_none(), "frame should be incomplete");
            cur_time += MID_CHAN_VAL;
        }

        //send the next sync
        cur_time += TEST_RESYNC_WIDTH;
        parser.handle_pulse_start(cur_time);
        //should now have a complete frame available
        let frame_opt = parser.next_frame();
        assert!(frame_opt.is_some(), "frame should be complete");

        if let Some(frame) = frame_opt {
            let valid_chans = frame.chan_count as usize;
            assert_eq!(
                valid_chans, TEST_CHAN_COUNT as usize,
                "wrong number of channels"
            );
            for i in 0..valid_chans {
                let val = frame.chan_values[i];
                assert_eq!(val, MID_CHAN_VAL)
            }
        }
    }
}
