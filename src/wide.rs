// Conversion
pub fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn from_wide(text: &[u16]) -> String {
    let end = text
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(text.len());
    String::from_utf16_lossy(&text[..end])
}

pub fn units_from_bytes(data: &[u8]) -> Vec<u16> {
    data.as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .collect()
}

pub fn copy_to_buffer(buffer: &mut [u16], text: &str) {
    let Some(capacity) = buffer.len().checked_sub(1) else {
        return;
    };
    let mut length = 0;
    for unit in text.encode_utf16().take(capacity) {
        buffer[length] = unit;
        length += 1;
    }
    if length > 0 && (0xD800..0xDC00).contains(&buffer[length - 1]) {
        length -= 1;
    }
    buffer[length..].fill(0);
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_text() {
        let wide = to_wide("Grüße");
        assert_eq!(wide.last(), Some(&0));
        assert_eq!(from_wide(&wide), "Grüße");
    }

    #[test]
    fn truncates_into_a_buffer() {
        let mut buffer = [1_u16; 4];
        copy_to_buffer(&mut buffer, "abcdef");
        assert_eq!(buffer, [97, 98, 99, 0]);
    }

    #[test]
    fn does_not_split_surrogate_pairs() {
        let mut buffer = [1_u16; 3];
        copy_to_buffer(&mut buffer, "a😀");
        assert_eq!(buffer, [97, 0, 0]);
    }
}
