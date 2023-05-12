pub struct RailFence {
    rails: usize,
}

impl RailFence {
    pub fn new(rails: u32) -> RailFence {
        Self {
            rails: rails as usize,
        }
    }

    pub fn encode(&self, text: &str) -> String {
        let mut rows = vec!["".to_string(); self.rails];
        let mut current_row = 0;
        let mut direction = 1;

        for ch in text.chars() {
            rows[current_row].push(ch);

            if current_row == 0 {
                direction = 1;
            } else if current_row == self.rails - 1 {
                direction = -1;
            }

            current_row = (current_row as isize + direction) as usize;
        }

        rows.join("")
    }

    pub fn decode(&self, cipher: &str) -> String {
        let mut rails: Vec<Vec<usize>> = vec![Vec::new(); self.rails];

        // Build the rail structure with indices
        let mut row = 0;
        let mut direction = 1;
        for (i, _) in cipher.chars().enumerate() {
            rails[row].push(i);

            // Toggle direction when we reach the ends of the rails
            if row == 0 {
                direction = 1;
            } else if row == self.rails - 1 {
                direction = -1;
            }

            row = (row as i32 + direction) as usize;
        }

        // Reorder the indices according to the rail structure
        let mut indices: Vec<usize> = Vec::new();
        for rail in rails {
            indices.extend(rail);
        }

        // Build the decoded message
        let mut plain = vec![' '; cipher.len()];
        for (i, ch) in cipher.chars().enumerate() {
            plain[indices[i]] = ch;
        }

        plain.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::rail_fence_cipher::*;

    fn process_encode_case(input: &str, rails: u32, expected: &str) {
        let rail_fence = RailFence::new(rails);

        assert_eq!(rail_fence.encode(input), expected);
    }

    fn process_decode_case(input: &str, rails: u32, expected: &str) {
        let rail_fence = RailFence::new(rails);

        assert_eq!(rail_fence.decode(input), expected);
    }

    // encode

    #[test]
    fn test_encode_with_two_rails() {
        process_encode_case("XOXOXOXOXOXOXOXOXO", 2, "XXXXXXXXXOOOOOOOOO");
    }

    #[test]
    fn test_encode_with_three_rails() {
        process_encode_case("WEAREDISCOVEREDFLEEATONCE", 3, "WECRLTEERDSOEEFEAOCAIVDEN");
    }

    #[test]
    fn test_encode_with_ending_in_the_middle() {
        process_encode_case("EXERCISES", 4, "ESXIEECSR");
    }

    // decode

    #[test]
    fn test_decode_with_three_rails() {
        process_decode_case("TEITELHDVLSNHDTISEIIEA", 3, "THEDEVILISINTHEDETAILS");
    }

    #[test]
    fn test_decode_with_five_rails() {
        process_decode_case("EIEXMSMESAORIWSCE", 5, "EXERCISMISAWESOME");
    }

    #[test]
    fn test_decode_with_six_rails() {
        process_decode_case(
            "133714114238148966225439541018335470986172518171757571896261",
            6,
            "112358132134558914423337761098715972584418167651094617711286",
        );
    }

    #[test]
    fn test_encode_wide_characters() {
        process_encode_case("古池蛙飛び込む水の音", 3, "古びの池飛込水音蛙む");
    }
}
