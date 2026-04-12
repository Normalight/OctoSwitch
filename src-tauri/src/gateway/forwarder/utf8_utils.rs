//! Safe UTF-8 accumulation for streaming byte chunks (SSE and similar).
//! Avoids U+FFFD when a multi-byte character is split across TCP chunks.

/// Append raw bytes to a UTF-8 `String` buffer, correctly handling multi-byte
/// characters split across chunk boundaries.
pub(crate) fn append_utf8_safe(buffer: &mut String, remainder: &mut Vec<u8>, new_bytes: &[u8]) {
    let (owned, bytes): (Option<Vec<u8>>, &[u8]) = if remainder.is_empty() {
        (None, new_bytes)
    } else {
        if remainder.len() > 3 {
            buffer.push_str(&String::from_utf8_lossy(remainder));
            remainder.clear();
            (None, new_bytes)
        } else {
            let mut combined = std::mem::take(remainder);
            combined.extend_from_slice(new_bytes);
            (Some(combined), &[])
        }
    };
    let input = owned.as_deref().unwrap_or(bytes);

    let mut pos = 0;
    loop {
        match std::str::from_utf8(&input[pos..]) {
            Ok(s) => {
                buffer.push_str(s);
                return;
            }
            Err(e) => {
                let valid_up_to = pos + e.valid_up_to();
                buffer.push_str(std::str::from_utf8(&input[pos..valid_up_to]).unwrap());
                if let Some(invalid_len) = e.error_len() {
                    buffer.push('\u{FFFD}');
                    pos = valid_up_to + invalid_len;
                } else {
                    *remainder = input[valid_up_to..].to_vec();
                    return;
                }
            }
        }
    }
}

/// On stream end, flush any trailing incomplete UTF-8 bytes (lossy).
pub(crate) fn flush_utf8_remainder(buffer: &mut String, remainder: &mut Vec<u8>) {
    if remainder.is_empty() {
        return;
    }
    buffer.push_str(&String::from_utf8_lossy(remainder));
    remainder.clear();
}

#[cfg(test)]
mod tests {
    use super::{append_utf8_safe, flush_utf8_remainder};

    #[test]
    fn split_multibyte_across_two_chunks() {
        let bytes = "你".as_bytes();
        let mut buf = String::new();
        let mut rem = Vec::new();
        append_utf8_safe(&mut buf, &mut rem, &bytes[..2]);
        assert_eq!(buf, "");
        assert_eq!(rem.len(), 2);
        append_utf8_safe(&mut buf, &mut rem, &bytes[2..]);
        assert_eq!(buf, "你");
        assert!(rem.is_empty());
    }

    #[test]
    fn flush_remainder_on_eof() {
        let mut buf = String::new();
        let mut rem = vec![0xE4, 0xBD];
        flush_utf8_remainder(&mut buf, &mut rem);
        assert!(rem.is_empty());
        assert!(!buf.is_empty());
    }
}
