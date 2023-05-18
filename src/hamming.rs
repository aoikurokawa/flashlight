/// Return the Hamming distance between the strings,
/// or None if the lengths are mismatched.
pub fn hamming_distance(s1: &str, s2: &str) -> Option<usize> {
    if s1.len() != s2.len() {
        None
    } else {
        let distance = s1
            .chars()
            .zip(s2.chars())
            .filter(|(s1_c, s2_c)| s1_c != s2_c)
            .count();
        Some(distance)
    }
}

#[cfg(test)]
mod tests {
    use crate::hamming;

    fn process_distance_case(strand_pair: [&str; 2], expected_distance: Option<usize>) {
        assert_eq!(
            hamming::hamming_distance(strand_pair[0], strand_pair[1]),
            expected_distance
        );
    }

    #[test]
    fn test_empty_strands() {
        process_distance_case(["", ""], Some(0));
    }

    #[test]
    fn test_disallow_first_strand_longer() {
        process_distance_case(["AATG", "AAA"], None);
    }

    #[test]
    fn test_disallow_second_strand_longer() {
        process_distance_case(["ATA", "AGTG"], None);
    }

    #[test]
    fn test_first_string_is_longer() {
        process_distance_case(["AAA", "AA"], None);
    }

    #[test]
    fn test_second_string_is_longer() {
        process_distance_case(["A", "AA"], None);
    }

    #[test]
    fn test_single_letter_identical_strands() {
        process_distance_case(["A", "A"], Some(0));
    }

    #[test]
    fn test_single_letter_different_strands() {
        process_distance_case(["G", "T"], Some(1));
    }

    #[test]
    fn test_long_identical_strands() {
        process_distance_case(["GGACTGAAATCTG", "GGACTGAAATCTG"], Some(0));
    }

    #[test]
    fn test_no_difference_between_identical_strands() {
        process_distance_case(["GGACTGA", "GGACTGA"], Some(0));
    }

    #[test]
    fn test_complete_hamming_distance_in_small_strand() {
        process_distance_case(["ACT", "GGA"], Some(3));
    }

    #[test]
    fn test_small_hamming_distance_in_the_middle_somewhere() {
        process_distance_case(["GGACG", "GGTCG"], Some(1));
    }

    #[test]
    fn test_larger_distance() {
        process_distance_case(["ACCAGGG", "ACTATGG"], Some(2));
    }

    #[test]
    fn test_long_different_strands() {
        process_distance_case(["GGACGGATTCTG", "AGGACGGATTCT"], Some(9));
    }
}
