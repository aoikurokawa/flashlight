use rand::Rng;

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

fn check_key(key: &u8) -> bool {
    return key.is_ascii_lowercase();
}

pub fn encode(key: &str, s: &str) -> Option<String> {
    let mut gaps = vec![];
    let mut encoded = String::new();

    if key.is_empty() {
        return None;
    }

    for key_ch in key.as_bytes().iter() {
        if !check_key(key_ch) {
            return None;
        }

        for (index, alpha) in ALPHABET.iter().enumerate() {
            if alpha == key_ch {
                gaps.push(index);
            }
        }
    }

    for (val_index, val_ch) in s.as_bytes().iter().enumerate() {
        for (index, alpha) in ALPHABET.iter().enumerate() {
            if alpha == val_ch {
                let pos = (26 + index + gaps[val_index % key.len()]) % 26;
                encoded = format!("{}{}", encoded, ALPHABET[pos] as char);
            }
        }
    }

    Some(encoded)
}

pub fn decode(key: &str, s: &str) -> Option<String> {
    let mut gaps = vec![];
    let mut decoded = String::new();

    if key.is_empty() {
        return None;
    }

    for key_ch in key.as_bytes().iter() {
        if !check_key(key_ch) {
            return None;
        }

        for (index, alpha) in ALPHABET.iter().enumerate() {
            if alpha == key_ch {
                gaps.push(index);
            }
        }
    }

    for (val_index, val_ch) in s.as_bytes().iter().enumerate() {
        for (index, alpha) in ALPHABET.iter().enumerate() {
            if alpha == val_ch {
                let pos = (26 + index as isize - gaps[val_index % key.len()] as isize).abs() % 26;
                decoded = format!("{}{}", decoded, ALPHABET[pos as usize] as char);
            }
        }
    }

    Some(decoded)
}

pub fn encode_random(s: &str) -> (String, String) {
    let s_len = 100;
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
