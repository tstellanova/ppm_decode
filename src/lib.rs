#![no_std]


use crate::PpmPhase::PULSE_ACTIVE;
use core::borrow::Borrow;

/// Base type for PPM timing
pub type Microseconds = u32;

/// PPM timing values
pub type PpmTime = Microseconds;

/// Default minimum channel value
pub const MIN_CHAN_VAL: PpmTime = 1000; //TODO s/b 800?
/// Default maximum channel value
pub const MAX_CHAN_VAL: PpmTime = 2000; //TODO s/b 2200 ?
/// Default midpoint channel value
pub const MID_CHAN_VAL: PpmTime = (MAX_CHAN_VAL + MIN_CHAN_VAL) / 2;

/// Default minimum width of a pulse
pub const MIN_PULSE_WIDTH: PpmTime =   200;

/// Default minimum reset/sync gap
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

enum PpmPhase {
    /// we have not yet received a long frame synchronization gap
    UNSYNC,
    /// we've received the start of a pulse
    PULSE_ACTIVE,
}

pub struct PpmParser {

    /// Configurable minimum channel value
    min_chan_value: PpmTime,

    /// Configurable maximum channel value
    max_chan_value: PpmTime,

    /// Configurable start/reset signal width
    min_sync_width: PpmTime,

    /// Configurable minimum number of channels per valid frame
    min_channels: u8,

    /// Current parsing phase
    phase: PpmPhase,

    /// the last time an (active) pulse started
    last_pulse_start: PpmTime,

    /// working memory for current frame capture
    working_frame: PpmFrame,

    /// frame ready for consumption
    parsed_frame: Option<PpmFrame>,

}

impl PpmParser {
    pub fn new() -> Self {
        Self {
            min_chan_value: MIN_CHAN_VAL,
            max_chan_value: MAX_CHAN_VAL,
            working_frame: PpmFrame {
                chan_values: [0; MAX_PPM_CHANNELS],
                chan_count: 0,
            },
            parsed_frame: None,
            min_sync_width: MIN_SYNC_WIDTH,
            min_channels: MIN_PPM_CHANNELS,
            phase: PpmPhase::UNSYNC,
            last_pulse_start: 0,
        }
    }

    /// Configure channel value range
    pub fn set_channel_limits(&mut self, min: PpmTime, max: PpmTime) -> &mut Self {
        self.min_chan_value = min;
        self.max_chan_value = max;
        self
    }

    /// Configure duration of frame sync
    pub fn set_sync_width(&mut self, width: PpmTime) -> &mut Self {
        self.min_sync_width = width;
        self
    }

    /// Handle a pulse start
    pub fn handle_pulse_start(&mut self, count: PpmTime) {
        let width = count - self.last_pulse_start;
        self.last_pulse_start = count;

        match self.phase {
            PpmPhase::UNSYNC => {
                // assume we've never received any pulses before
                if width >= MIN_SYNC_WIDTH {
                    //received sync
                    self.reset_channel_counter();
                    self.phase = PpmPhase::PULSE_ACTIVE;
                }
            }
            PpmPhase::PULSE_ACTIVE => {
                // previous pulse has ended
                if width >= MIN_SYNC_WIDTH {
                    self.working_frame.chan_count += 1;
                    //received sync -- we should be finished with prior frame
                    //TODO verify we receive a consistent number of channels
                    if self.working_frame.chan_count >= self.min_channels {
                        self.parsed_frame.replace(self.working_frame);
                    }
                    else {
                        //we didn't get expected minimum number of channels
                        self.parsed_frame = None;
                    }
                    self.reset_channel_counter();
                }
                else {
                    self.working_frame.chan_values[self.working_frame.chan_count as usize] = width;
                    self.working_frame.chan_count += 1;
                }
            }
        }

    }


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

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn process_pulses() {
        const TEST_CHAN_COUNT: u8 = 16;
        const TEST_RESYNC_WIDTH: PpmTime = 2500;
        let mut parser = PpmParser::new();
        parser.set_channel_limits(800, 2200)
            .set_sync_width(TEST_RESYNC_WIDTH);

        let mut cur_time: PpmTime = 100;
        //start with a garbage pulse from prior packet
        parser.handle_pulse_start(cur_time);
        //this effectively starts a new packet
        cur_time += TEST_RESYNC_WIDTH;
        let frame = parser.next_frame();
        assert!(frame.is_none(), "there should be no frame yet");

        //send a full frame
        for _ in 0..TEST_CHAN_COUNT {
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
            assert_eq!(valid_chans, TEST_CHAN_COUNT as usize, "wrong number of channels");
            for i in 0..valid_chans {
                let val = frame.chan_values[i];
                assert_eq!(val, MID_CHAN_VAL)
            }
        }
    }
}
