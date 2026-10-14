//! Reading the samples the page captures. The page hands over each channel as
//! the bytes of a little-endian IEEE 754 single-precision array, which is the
//! layout an `AudioWorklet` receives. The core never names a float type
//! (reference default D2): it reads a sample's bit pattern with integer
//! arithmetic, and passes the bytes on to the encoder untouched.

/// Bytes in one sample.
pub const SAMPLE_BYTES: usize = 4;

/// Full scale in the integer reading: a sample of 1.0 reads as this.
pub const FULL_SCALE: u32 = 32768;

/// The magnitude of one sample, scaled so that 1.0 reads as `FULL_SCALE` and
/// rounded toward zero. Louder than full scale reads as full scale; zero,
/// subnormals and not-a-number read as 0.
pub fn magnitude(bits: u32) -> u32 {
    let exponent = (bits >> 23) & 0xff;
    let fraction = bits & 0x7f_ffff;
    if exponent == 0xff {
        // Infinity is louder than full scale; not-a-number is no sound at all.
        return if fraction == 0 { FULL_SCALE } else { 0 };
    }
    if exponent == 0 {
        return 0;
    }
    // value = (1.fraction) * 2^(exponent - 127); times 2^15 for full scale.
    // As an integer: (0x80_0000 | fraction) * 2^(exponent - 127 + 15 - 23).
    let significand = 0x80_0000 | fraction;
    if exponent >= 135 {
        return FULL_SCALE;
    }
    let shift = 135 - exponent;
    if shift >= 32 {
        0
    } else {
        (significand >> shift).min(FULL_SCALE)
    }
}

/// The bit pattern of the sample starting at `at`.
fn bits_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// The loudest sample in a block of little-endian samples. A trailing partial
/// sample is ignored.
pub fn peak(bytes: &[u8]) -> u32 {
    let (samples, _partial) = bytes.as_chunks::<SAMPLE_BYTES>();
    samples
        .iter()
        .map(|c| magnitude(u32::from_le_bytes(*c)))
        .max()
        .unwrap_or(0)
}

/// Interleaves two channels of little-endian samples into one stereo block:
/// left, right, left, right. The shorter channel sets the length, so a
/// channel the page sent short cannot shift the other out of step.
pub fn interleave(left: &[u8], right: &[u8]) -> Vec<u8> {
    let frames = left.len().min(right.len()) / SAMPLE_BYTES;
    let mut out = Vec::with_capacity(frames * 2 * SAMPLE_BYTES);
    for k in 0..frames {
        let at = k * SAMPLE_BYTES;
        out.extend_from_slice(&bits_at(left, at).to_le_bytes());
        out.extend_from_slice(&bits_at(right, at).to_le_bytes());
    }
    out
}

/// The number of sample frames in a channel's bytes.
pub fn frames_in(channel: &[u8]) -> usize {
    channel.len() / SAMPLE_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    // Bit patterns worked out by hand from IEEE 754 single precision:
    // sign (1 bit), exponent (8 bits, bias 127), fraction (23 bits).
    const ONE: u32 = 0x3f80_0000; // 1.0
    const HALF: u32 = 0x3f00_0000; // 0.5
    const MINUS_HALF: u32 = 0xbf00_0000; // -0.5
    const TWO: u32 = 0x4000_0000; // 2.0
    const QUARTER_AND_EIGHTH: u32 = 0x3ec0_0000; // 0.375 = 1.5 * 2^-2
    const ONE_STEP: u32 = 0x3800_0000; // 2^-15, one step of the integer reading
    const BELOW_ONE_STEP: u32 = 0x37ff_ffff; // just under 2^-15
    const INFINITY: u32 = 0x7f80_0000;
    const NAN: u32 = 0x7fc0_0000;
    const SUBNORMAL: u32 = 0x0000_0001;

    fn bytes(samples: &[u32]) -> Vec<u8> {
        samples.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    #[test]
    fn magnitude_reads_hand_worked_values() {
        assert_eq!(magnitude(ONE), 32768);
        assert_eq!(magnitude(HALF), 16384);
        assert_eq!(magnitude(MINUS_HALF), 16384);
        assert_eq!(magnitude(QUARTER_AND_EIGHTH), 12288);
        assert_eq!(magnitude(ONE_STEP), 1);
        assert_eq!(magnitude(BELOW_ONE_STEP), 0);
        assert_eq!(magnitude(0), 0);
        assert_eq!(magnitude(0x8000_0000), 0, "minus zero");
    }

    #[test]
    fn magnitude_caps_loud_samples_at_full_scale() {
        assert_eq!(magnitude(TWO), 32768);
        assert_eq!(magnitude(INFINITY), 32768);
        assert_eq!(magnitude(0x7f7f_ffff), 32768, "the largest finite value");
    }

    #[test]
    fn magnitude_reads_not_a_number_and_subnormals_as_silence() {
        assert_eq!(magnitude(NAN), 0);
        assert_eq!(magnitude(SUBNORMAL), 0);
    }

    #[test]
    fn peak_finds_the_loudest_sample_and_ignores_a_partial_one() {
        let mut b = bytes(&[ONE_STEP, MINUS_HALF, QUARTER_AND_EIGHTH]);
        b.extend_from_slice(&[0xff, 0xff]);
        assert_eq!(peak(&b), 16384);
        assert_eq!(peak(&[]), 0);
    }

    #[test]
    fn interleave_alternates_channels_and_stops_at_the_shorter() {
        let left = bytes(&[ONE, HALF, TWO]);
        let right = bytes(&[MINUS_HALF, ONE_STEP]);
        assert_eq!(
            interleave(&left, &right),
            bytes(&[ONE, MINUS_HALF, HALF, ONE_STEP])
        );
        assert_eq!(frames_in(&left), 3);
    }
}
