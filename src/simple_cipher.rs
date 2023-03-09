use rand::Rng;

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

pub fn encode(key: &str, s: &str) -> Option<String> {
    let mut gaps = vec![];
    let mut encoded = String::new();

    for key_ch in key.as_bytes().iter() {
        for (index, alpha) in ALPHABET.iter().enumerate() {
            if alpha == key_ch {
                gaps.push(index);
            }
        }
    }

    for (val_index, val_ch) in s.as_bytes().iter().enumerate() {
        for (index, alpha) in ALPHABET.iter().enumerate() {
             if alpha == val_ch {
                encoded = format!(
                    "{}{}",
                    encoded,
                    ALPHABET[(index + gaps[val_index]) % 26] as char
                );
             }
        }
    }

    Some(encoded)
}

pub fn decode(key: &str, s: &str) -> Option<String> {
    let mut gaps = vec![];
    let mut decoded = String::new();

    for key_ch in key.as_bytes().iter() {
        for (index, alpha) in ALPHABET.iter().enumerate() {
            if alpha == key_ch {
                gaps.push(index);
            }
        }
    }

    for (val_index, val_ch) in s.as_bytes().iter().enumerate() {
        for (index, alpha) in ALPHABET.iter().enumerate() {
            if alpha == val_ch {
                decoded = format!(
                    "{}{}",
                    decoded,
                    ALPHABET[(index - gaps[val_index % key.len()]) % 26] as char
                );
            }
        }
    }

    Some(decoded)
}

pub fn encode_random(s: &str) -> (String, String) {
    let s_len = s.len();
    let mut rng = rand::thread_rng();

    let random_str: String = (0..s_len)
        .map(|_| {
            let idx = rng.gen_range(0..ALPHABET.len());
            ALPHABET[idx] as char
        })
        .collect();

    let encoded = encode(random_str.as_str(), s).unwrap();

    (random_str, encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN_TEXT: &str = "thisismysecret";
    const KEY: &str = "abcdefghij";

    #[test]
    fn cipher_can_encode_with_given_key() {
        assert_eq!(encode(KEY, "aaaaaaaaaa"), Some(KEY.to_string()));
    }

    #[test]
    fn cipher_can_decode_with_given_key() {
        assert_eq!(decode(KEY, "abcdefghij"), Some("aaaaaaaaaa".to_string()));
    }

    #[test]
    fn cipher_can_double_shift_encode() {
        let plain_text = "iamapandabear";
        assert_eq!(
            encode(plain_text, plain_text),
            Some("qayaeaagaciai".to_string())
        );
    }

    #[test]
    fn cipher_can_wrap_encode() {
        assert_eq!(encode(KEY, "zzzzzzzzzz"), Some("zabcdefghi".to_string()));
    }

    #[test]
    fn cipher_is_reversible_given_key() {
        assert_eq!(
            decode(KEY, &encode(KEY, PLAIN_TEXT).unwrap()),
            Some(PLAIN_TEXT.to_string())
        );
    }
}
