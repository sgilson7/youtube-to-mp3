//! The recording: what is kept, what is left out, when it stops, and the MP3
//! it becomes.
//!
//! Sound arrives in short blocks from the shared tab. Blocks before the first
//! sound are dropped, so the file starts when the video does. Once sound has
//! started, a silent block is held back rather than encoded: if sound
//! returns, the held silence is encoded in its place (a pause inside the
//! video stays); if the recording stops first, the held silence is dropped
//! (the quiet after the video ends is left out). With stopping on silence
//! turned on, enough held silence ends the recording by itself.

use rusty_mp3::{Mp3Encoder, Mp3EncoderConfig};

use crate::pcm;

/// The bitrates offered, in kilobits a second. The encoder's notes say its
/// 320 kbps output is garbled, so it is not offered (BRIEF.md).
pub const BITRATES: &[u32] = &[128, 192, 256];

/// The bitrate chosen until the reader picks another.
pub const DEFAULT_BITRATE: u32 = 192;

/// Sample rates the MP3 format holds at these bitrates (MPEG-1 Layer III).
pub const RATES: &[u32] = &[32_000, 44_100, 48_000];

/// A block whose loudest sample reads at or below this is silence: 32 of
/// 32768, about 60 decibels below full scale.
pub const SILENCE: u32 = 32;

/// Seconds of silence, after sound has started, that end the recording when
/// stopping on silence is on.
pub const SILENCE_SECONDS: u32 = 3;

/// The longest recording, in hours. It bounds the memory a forgotten
/// recording can take.
pub const LIMIT_HOURS: u32 = 3;

/// Why a recording stopped by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    Silence,
    Limit,
}

/// What a block of sound did to the recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fed {
    /// Still waiting for the first sound.
    Waiting,
    /// Recording continues.
    Recording,
    /// The recording should stop now, for this reason.
    Stop(Stop),
}

/// A recording in progress at one sample rate.
pub struct Recorder {
    rate: u32,
    bitrate: u32,
    stop_on_silence: bool,
    limit_frames: u64,
    encoder: Mp3Encoder,
    started: bool,
    kept_frames: u64,
    held: Vec<u8>,
    held_frames: u64,
    peak: u32,
}

impl Recorder {
    /// A recorder at `rate`, or `None` if MP3 cannot hold that rate.
    pub fn new(rate: u32, bitrate: u32, stop_on_silence: bool) -> Option<Recorder> {
        Recorder::with_limit(rate, bitrate, stop_on_silence, u64::from(LIMIT_HOURS) * 3600)
    }

    /// As `new`, with the limit given in seconds, so tests can reach it.
    pub fn with_limit(
        rate: u32,
        bitrate: u32,
        stop_on_silence: bool,
        limit_seconds: u64,
    ) -> Option<Recorder> {
        if !RATES.contains(&rate) || !BITRATES.contains(&bitrate) {
            return None;
        }
        Some(Recorder {
            rate,
            bitrate,
            stop_on_silence,
            limit_frames: limit_seconds * u64::from(rate),
            encoder: Mp3Encoder::new(Mp3EncoderConfig {
                bitrate_kbps: bitrate,
                vbr_quality: None,
            }),
            started: false,
            kept_frames: 0,
            held: Vec::new(),
            held_frames: 0,
            peak: 0,
        })
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn bitrate(&self) -> u32 {
        self.bitrate
    }

    /// Whether any sound has been heard.
    pub fn started(&self) -> bool {
        self.started
    }

    /// Sample frames since the first sound, the held silence included: the
    /// length the reader sees while recording.
    pub fn heard_frames(&self) -> u64 {
        self.kept_frames + self.held_frames
    }

    /// Sample frames that will be in the file.
    pub fn kept_frames(&self) -> u64 {
        self.kept_frames
    }

    /// The loudest sample since the last call, as a share of full scale out of
    /// 100, and starts the next reading.
    pub fn take_level(&mut self) -> u32 {
        let level = self.peak * 100 / pcm::FULL_SCALE;
        self.peak = 0;
        level
    }

    fn encode(&mut self, interleaved: &[u8]) {
        // The rate was checked in `with_limit`, the only way to make a
        // Recorder, so the encoder cannot refuse it.
        let _ = self.encoder.push_pcm_f32le(interleaved, 2, self.rate);
    }

    /// Takes one block: the left and right channels as little-endian samples.
    /// A mono source sends the same channel twice.
    pub fn feed(&mut self, left: &[u8], right: &[u8]) -> Fed {
        let loudest = pcm::peak(left).max(pcm::peak(right));
        self.peak = self.peak.max(loudest);
        let frames = pcm::frames_in(left).min(pcm::frames_in(right)) as u64;
        let silent = loudest <= SILENCE;
        if !self.started {
            if silent {
                return Fed::Waiting;
            }
            self.started = true;
        }
        let block = pcm::interleave(left, right);
        if silent {
            self.held.extend_from_slice(&block);
            self.held_frames += frames;
        } else {
            let held = std::mem::take(&mut self.held);
            self.encode(&held);
            self.kept_frames += self.held_frames;
            self.held_frames = 0;
            self.encode(&block);
            self.kept_frames += frames;
        }
        if self.heard_frames() >= self.limit_frames {
            return Fed::Stop(Stop::Limit);
        }
        if self.stop_on_silence && self.held_frames >= u64::from(SILENCE_SECONDS) * u64::from(self.rate) {
            return Fed::Stop(Stop::Silence);
        }
        Fed::Recording
    }

    /// Ends the recording, drops the held silence, and returns the MP3 frames,
    /// or `None` if no sound was heard.
    pub fn finish(mut self) -> Option<Vec<u8>> {
        if !self.started {
            return None;
        }
        self.encoder.finish();
        let mut mp3 = Vec::new();
        while let Ok(packet) = self.encoder.next_packet() {
            mp3.extend(packet);
        }
        Some(mp3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 32_000;
    const BLOCK: usize = 128;
    const LOUD: u32 = 0x3f00_0000; // 0.5
    const QUIET: u32 = 0x3800_0000; // 2^-15, reads as 1: silence

    fn block(sample: u32) -> Vec<u8> {
        sample.to_le_bytes().repeat(BLOCK)
    }

    fn recorder(stop_on_silence: bool) -> Recorder {
        Recorder::new(RATE, 128, stop_on_silence).unwrap()
    }

    fn feed(r: &mut Recorder, sample: u32, blocks: usize) -> Fed {
        let b = block(sample);
        let mut last = Fed::Waiting;
        for _ in 0..blocks {
            last = r.feed(&b, &b);
        }
        last
    }

    #[test]
    fn only_rates_and_bitrates_the_format_holds_are_accepted() {
        assert!(Recorder::new(48_000, 192, true).is_some());
        assert!(Recorder::new(44_100, 256, true).is_some());
        assert!(Recorder::new(96_000, 192, true).is_none());
        assert!(Recorder::new(22_050, 192, true).is_none());
        assert!(Recorder::new(48_000, 320, true).is_none());
    }

    #[test]
    fn silence_before_the_first_sound_is_left_out() {
        let mut r = recorder(true);
        assert_eq!(feed(&mut r, QUIET, 1000), Fed::Waiting);
        assert!(!r.started());
        assert_eq!(r.heard_frames(), 0);
        assert_eq!(feed(&mut r, LOUD, 2), Fed::Recording);
        assert_eq!(r.kept_frames(), 2 * BLOCK as u64);
    }

    #[test]
    fn a_pause_between_sounds_is_kept() {
        let mut r = recorder(true);
        feed(&mut r, LOUD, 1);
        feed(&mut r, QUIET, 10);
        assert_eq!(r.kept_frames(), BLOCK as u64, "the pause is held, not yet kept");
        assert_eq!(r.heard_frames(), 11 * BLOCK as u64);
        feed(&mut r, LOUD, 1);
        assert_eq!(
            r.kept_frames(),
            12 * BLOCK as u64,
            "sound returned, so the pause is kept"
        );
    }

    #[test]
    fn silence_at_the_end_is_left_out_of_the_file() {
        let mut ends_quiet = recorder(false);
        feed(&mut ends_quiet, LOUD, 250);
        feed(&mut ends_quiet, QUIET, 250);
        assert_eq!(ends_quiet.kept_frames(), 250 * BLOCK as u64);
        let mut sound_only = recorder(false);
        feed(&mut sound_only, LOUD, 250);
        assert_eq!(
            ends_quiet.finish().unwrap().len(),
            sound_only.finish().unwrap().len()
        );
    }

    #[test]
    fn enough_silence_after_sound_stops_the_recording() {
        // 3 s at 32000/s is 96000 frames, which is 750 blocks of 128.
        let mut r = recorder(true);
        feed(&mut r, LOUD, 1);
        assert_eq!(feed(&mut r, QUIET, 749), Fed::Recording);
        assert_eq!(feed(&mut r, QUIET, 1), Fed::Stop(Stop::Silence));
    }

    #[test]
    fn silence_does_not_stop_the_recording_when_that_is_off() {
        let mut r = recorder(false);
        feed(&mut r, LOUD, 1);
        assert_eq!(feed(&mut r, QUIET, 2000), Fed::Recording);
    }

    #[test]
    fn silence_before_any_sound_never_stops_the_recording() {
        let mut r = recorder(true);
        assert_eq!(feed(&mut r, QUIET, 2000), Fed::Waiting);
    }

    #[test]
    fn the_recording_stops_at_the_limit() {
        // 1 s at 32000/s is 250 blocks of 128.
        let mut r = Recorder::with_limit(RATE, 128, false, 1).unwrap();
        assert_eq!(feed(&mut r, LOUD, 249), Fed::Recording);
        assert_eq!(feed(&mut r, LOUD, 1), Fed::Stop(Stop::Limit));
    }

    #[test]
    fn the_level_is_the_loudest_sample_since_the_last_reading() {
        let mut r = recorder(true);
        feed(&mut r, QUIET, 1);
        feed(&mut r, LOUD, 1);
        assert_eq!(r.take_level(), 50);
        assert_eq!(r.take_level(), 0);
    }

    #[test]
    fn no_sound_gives_no_file() {
        let mut r = recorder(true);
        feed(&mut r, QUIET, 10);
        assert_eq!(r.finish(), None);
    }

    #[test]
    fn a_recording_becomes_mp3_frames_at_the_chosen_bitrate() {
        let mut r = Recorder::new(48_000, 192, false).unwrap();
        // One second at 48000/s: 375 blocks of 128.
        feed(&mut r, LOUD, 375);
        let mp3 = r.finish().unwrap();
        // The encoder writes an Info frame first, then one frame per 1152
        // samples. A 192 kbps frame at 48 kHz is 144 * 192000 / 48000 = 576
        // bytes; one second is 41.7 frames, padded to 42, plus the Info
        // frame: 43 * 576 = 24768 bytes.
        assert_eq!(mp3.len(), 24_768);
        // Frame sync, MPEG-1 Layer III without CRC, then the bitrate index
        // for 192 kbps (1011) and the rate index for 48 kHz (01).
        assert_eq!(&mp3[..3], &[0xff, 0xfb, 0xb4]);
    }
}
