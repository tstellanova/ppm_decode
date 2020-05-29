/*
Copyright (c) 2020 Todd Stellanova
LICENSE: BSD3 (see LICENSE file)
*/

#![no_std]

//!
//! This library provides decoding of PPM pulse edges into PPM frames.
//! It does not require a particular interrupt handling or input pin
//! measurement strategy.  All the user needs to provide is the
//! relative timing of pulses, and the library will extract PPM
//! frames from that.
//!
//! PPM channel values are encoded as the gap between multiple pulses.
//! Typically PPM pulses are high values and the gaps between them
//! are low values; however, some PPM receivers invert the signal,
//! where the PPM signal is pulled high and pulses are low values.
//! PPM frames consist of multiple channels encoded in this way,
//! with a longer duration gap between the last channel and the
//! first channel, after which comes the next frame.
//! This frame-separating gap is referred to as a frame sync or reset.
//! So a PPM frame with five channels might look like:
//!
//! ______|___|___|___|___|___|______
//! Where high values are the pulses and low values are the
//! gaps between pulses. The pulse duration itself is typically
//! tuned to be as short as possible and still reliably transmitted.
//!
//! The library provides defaults for common configuration values
//! such as:
//! - Minimum PPM channel value (the minimum gap between pulses)
//! - Maximum PPM channel value (the maximum gap between pulses)
//! - Minimum frame sync duration (the minimum time for a gap between
//! pulses to be considered a frame sync / reset)
//! - Minimum number of PPM channels to be considered a valid frame.
//!
//!

/// Base type for PPM timing
/// Your clock for measuring pulse edges will need at least microsecond resolution.
pub type Microseconds = u32;

/// PPM timing values
pub type PpmTime = Microseconds;

/// Default minimum channel value
pub const MIN_CHAN_VAL: PpmTime = 800;
/// Default maximum channel value
pub const MAX_CHAN_VAL: PpmTime = 2200;
/// Default midpoint channel value
pub const MID_CHAN_VAL: PpmTime = (MAX_CHAN_VAL + MIN_CHAN_VAL) / 2;

/// Default minimum gap between frames (no pulses / inactive/ sync)
pub const MIN_SYNC_WIDTH: PpmTime = 4000;

/// Default minimum number of channels per frame
pub const MIN_PPM_CHANNELS: u8 = 5;

/// Maximum PPM channels this library supports
pub const MAX_PPM_CHANNELS: usize = 20;

/// A single group of PPM channel values
#[derive(Copy, Clone, Debug)]
pub struct PpmFrame {
    /// Decoded PPM channel values
    pub chan_values: [PpmTime; MAX_PPM_CHANNELS],
    /// Number of channels decoded (≤ MAX_PPM_CHANNELS)
    pub chan_count: u8,
}

/// Configuration values for PpmParser
#[derive(Copy, Clone, Debug)]
pub struct ParserConfig {
    /// Configurable minimum channel value
    min_chan_value: PpmTime,

    /// Configurable maximum channel value
    max_chan_value: PpmTime,

    /// Configurable middle channel value
    mid_chan_value: PpmTime,

    /// Configurable start/reset signal width
    min_sync_width: PpmTime,

    /// Configurable minimum number of channels per valid frame
    min_channels: u8,

    /// The maximum timer value, after which the clock/timer wraps,
    /// eg 0xFFFF for a 16-bit timer, 0xFFFF_FFFF for a 32-bit timer
    max_ppm_time: u32,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            min_chan_value: MIN_CHAN_VAL,
            max_chan_value: MAX_CHAN_VAL,
            mid_chan_value: MID_CHAN_VAL,
            min_sync_width: MIN_SYNC_WIDTH,
            min_channels: MIN_PPM_CHANNELS,
            max_ppm_time: 0xFFFF_FFFF,
        }
    }
}

/// The main PPM decoder.
///
/// # Example:
/// ```
///     use ppm_decode::*;
///         let mut parser = PpmParser::new();
///         //arbitrary start time
///         let mut cur_time: PpmTime = 100;
///
///         //start with a trailing pulse from prior frame, to force resync
///         parser.handle_pulse_start(cur_time);
///         let frame = parser.next_frame();
///         assert!(frame.is_none(), "there should be no complete frame yet");
///
///         //this effectively starts a new frame:
///         cur_time += MIN_SYNC_WIDTH;
///         // send n+1 pulses where n is the channel counts
///         for _ in 0..MIN_PPM_CHANNELS + 1 {
///             parser.handle_pulse_start(cur_time);
///             let frame = parser.next_frame();
///             assert!(frame.is_none(), "frame should be incomplete");
///             // each pulse is separated by the same gap in this test,
///             // which means all channels have the same value in this frame
///             cur_time += MID_CHAN_VAL;
///         }
///
///         //send the next sync
///         cur_time += MIN_SYNC_WIDTH;
///         parser.handle_pulse_start(cur_time);
///         //should now have a complete frame available
///         let frame_opt = parser.next_frame();
///         assert!(frame_opt.is_some(), "frame should be complete");
///
///         if let Some(frame) = frame_opt {
///             let valid_chans = frame.chan_count;
///             assert_eq!( valid_chans, MIN_PPM_CHANNELS, "wrong number of channels");
///             for i in 0..valid_chans as usize {
///                 let val = frame.chan_values[i];
///                 assert_eq!(val, MID_CHAN_VAL)
///             }
///         }
/// ```
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
        self.config.mid_chan_value = (max + min) / 2;
        self
    }

    /// Configure duration of frame sync
    pub fn set_sync_width(&mut self, width: PpmTime) -> &mut Self {
        self.config.min_sync_width = width;
        self
    }

    /// Set the minimum number of channels in a valid frame
    pub fn set_minimum_channels(&mut self, channels: u8) -> &mut Self {
        self.config.min_channels = channels;
        self
    }

    /// Set the maximum timer value -- allows us to use timers with
    /// different resolution than the default 32 bits
    pub fn set_max_ppm_time(&mut self, value: PpmTime) -> &mut Self {
        self.config.max_ppm_time = value;
        self
    }

    /// Get the next available PPM frame, if any.
    /// This function may return `None` if a complete
    /// frame has not been received yet, or if no
    /// frame sync has been received.
    pub fn next_frame(&mut self) -> Option<PpmFrame> {
        self.parsed_frame.take()
    }

    /// Handle a pulse start.  This could be the time
    /// in microseconds of a pulse rising edge or falling edge
    /// (depending on the PPM input and your measurement strategy)
    /// -- it does not really matter as long as you measure the
    /// the pulses consistently.
    ///
    pub fn handle_pulse_start(&mut self, count: PpmTime) {
        //calculate pulse width using wrapping subtraction based on max_ppm_time
        let width = if count > self.last_pulse_start {
            count - self.last_pulse_start
        } else {
            (self.config.max_ppm_time - self.last_pulse_start) + count
        };
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
                if width >= MIN_SYNC_WIDTH {
                    // Received sync -- check whether finished decoding a whole frame
                    // TODO add a feature to only allow slow drift of the channel count
                    if self.working_frame.chan_count >= self.config.min_channels
                    {
                        // We've received the configured minimum number of channels:
                        // frame is complete.
                        self.parsed_frame.replace(self.working_frame);
                    } else {
                        // We didn't receive the expected minimum number of channels.
                        self.parsed_frame = None;
                    }
                    self.reset_channel_counter();
                } else {
                    // Verify the pulse received is within limits, otherwise resync.
                    if width >= self.config.min_chan_value
                        && width <= self.config.max_chan_value
                    {
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
        //start with a garbage pulse from prior frame
        parser.handle_pulse_start(cur_time);
        let frame = parser.next_frame();
        assert!(frame.is_none(), "there should be no frame yet");

        //send a full frame
        //this effectively starts a new frame:
        cur_time += TEST_RESYNC_WIDTH;
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
            let valid_chans = frame.chan_count;
            assert_eq!(
                valid_chans, TEST_CHAN_COUNT,
                "wrong number of channels"
            );
            for i in 0..valid_chans as usize {
                let val = frame.chan_values[i];
                assert_eq!(val, MID_CHAN_VAL)
            }
        }
    }

    #[test]
    fn overflow_timer() {
        const TEST_CHAN_COUNT: u8 = 3;
        let mut parser = PpmParser::new();
        parser.set_minimum_channels(TEST_CHAN_COUNT);

        // for this test all the channel pulses are separated by the same gap (same channel value)
        const PULSE_GAP_TIME: PpmTime = MID_CHAN_VAL;

        // Send a pulse train that looks like this:
        // |______|___|___|___|______
        // where the third pulse arrives after PpmTime overflow
        // This calculated start time is for the first pulse:
        let mut cur_time: PpmTime =
            PpmTime::max_value() - PULSE_GAP_TIME - MIN_SYNC_WIDTH + 10;
        //start with a garbage pulse from prior frame
        parser.handle_pulse_start(cur_time);
        let frame = parser.next_frame();
        assert!(frame.is_none(), "there should be no complete frame yet");

        //this effectively starts a new frame:
        cur_time += MIN_SYNC_WIDTH;
        for _ in 0..TEST_CHAN_COUNT + 1 {
            parser.handle_pulse_start(cur_time);
            let frame = parser.next_frame();
            assert!(frame.is_none(), "frame should be incomplete");
            // this should overflow at the third pulse:
            cur_time = cur_time.wrapping_add(MID_CHAN_VAL);
        }

        //send the next sync
        cur_time += MIN_SYNC_WIDTH;
        parser.handle_pulse_start(cur_time);
        //should now have a complete frame available
        let frame_opt = parser.next_frame();
        assert!(frame_opt.is_some(), "frame should be complete");

        if let Some(frame) = frame_opt {
            let valid_chans = frame.chan_count;
            assert_eq!(
                valid_chans, TEST_CHAN_COUNT,
                "wrong number of channels"
            );
            for i in 0..valid_chans as usize {
                let val = frame.chan_values[i];
                assert_eq!(val, MID_CHAN_VAL)
            }
        }
    }
}
