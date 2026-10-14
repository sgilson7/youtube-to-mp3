//! The tag at the front of the MP3 that carries the title and artist: ID3
//! version 2.3, which players on every common system read. Text that fits in
//! ISO-8859-1 is written in it; anything else is written as UTF-16 with a
//! byte-order mark, the two encodings version 2.3 allows.

/// A text frame: four-letter id, then the text with its encoding byte.
fn text_frame(id: &[u8; 4], text: &str) -> Vec<u8> {
    let mut body = Vec::new();
    if text.chars().all(|c| (c as u32) < 0x100) {
        body.push(0); // ISO-8859-1
        body.extend(text.chars().map(|c| c as u32 as u8));
    } else {
        body.push(1); // UTF-16 with a byte-order mark
        body.extend_from_slice(&[0xff, 0xfe]);
        for unit in text.encode_utf16() {
            body.extend_from_slice(&unit.to_le_bytes());
        }
    }
    let mut frame = Vec::with_capacity(10 + body.len());
    frame.extend_from_slice(id);
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&[0, 0]); // no frame flags
    frame.extend(body);
    frame
}

/// A size in the tag header: four bytes of seven bits each.
fn synchsafe(n: u32) -> [u8; 4] {
    [
        ((n >> 21) & 0x7f) as u8,
        ((n >> 14) & 0x7f) as u8,
        ((n >> 7) & 0x7f) as u8,
        (n & 0x7f) as u8,
    ]
}

/// The tag for a title and an artist. Blank fields get no frame, and with
/// both blank there is no tag at all.
pub fn tag(title: &str, artist: &str) -> Vec<u8> {
    let mut frames = Vec::new();
    if !title.trim().is_empty() {
        frames.extend(text_frame(b"TIT2", title.trim()));
    }
    if !artist.trim().is_empty() {
        frames.extend(text_frame(b"TPE1", artist.trim()));
    }
    if frames.is_empty() {
        return frames;
    }
    let mut out = Vec::with_capacity(10 + frames.len());
    out.extend_from_slice(b"ID3");
    out.extend_from_slice(&[3, 0, 0]); // version 2.3.0, no flags
    out.extend_from_slice(&synchsafe(frames.len() as u32));
    out.extend(frames);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_and_artist_give_the_bytes_worked_out_by_hand() {
        // A frame's size counts its encoding byte: "Song" is 1 + 4 = 5 bytes,
        // "B" is 1 + 1 = 2. The tag's size counts the frames with their
        // ten-byte headers: 15 + 12 = 27 = 0x1b.
        let mut both = b"ID3\x03\x00\x00\x00\x00\x00\x1b".to_vec();
        both.extend_from_slice(b"TIT2\x00\x00\x00\x05\x00\x00\x00Song");
        both.extend_from_slice(b"TPE1\x00\x00\x00\x02\x00\x00\x00B");
        assert_eq!(tag(" Song ", "B"), both);

        let mut title_only = b"ID3\x03\x00\x00\x00\x00\x00\x0f".to_vec();
        title_only.extend_from_slice(b"TIT2\x00\x00\x00\x05\x00\x00\x00Song");
        assert_eq!(tag("Song", ""), title_only);
    }

    #[test]
    fn latin_text_stays_in_one_byte_a_letter() {
        let t = tag("Café", "");
        assert_eq!(&t[10..], b"TIT2\x00\x00\x00\x05\x00\x00\x00Caf\xe9");
    }

    #[test]
    fn other_text_is_written_as_utf16_with_a_byte_order_mark() {
        let t = tag("", "東");
        // U+6771 is 0x71 0x67 in little-endian; size = 1 + 2 + 2 = 5.
        assert_eq!(&t[10..], b"TPE1\x00\x00\x00\x05\x00\x00\x01\xff\xfe\x71\x67");
    }

    #[test]
    fn blank_fields_give_no_tag() {
        assert!(tag("", "  ").is_empty());
    }

    #[test]
    fn a_large_size_is_split_into_seven_bit_bytes() {
        // 300 = 0b10_0101100: low seven bits 0x2c, next 0x02.
        assert_eq!(synchsafe(300), [0, 0, 2, 0x2c]);
        assert_eq!(synchsafe(0x0fff_ffff), [0x7f, 0x7f, 0x7f, 0x7f]);
    }
}
