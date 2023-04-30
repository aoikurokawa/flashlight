/// Check a Luhn checksum.
pub fn is_valid(code: &str) -> bool {
    code.chars()
        .rev()
        .filter(|c| !c.is_whitespace())
        .try_fold((0, 0), |(sum, count), val| {
            val.to_digit(10)
                .map(|num| if count % 2 == 1 { num * 2 } else { num })
                .map(|num| if num > 9 { num - 9 } else { num })
                .map(|num| (num + sum, count + 1))
        })
        .map_or(false, |(sum, count)| sum % 10 == 0 && count > 1)
}

#[cfg(test)]
mod tests {
    use crate::luhn::*;

    fn process_valid_case(number: &str, is_luhn_expected: bool) {
        assert_eq!(is_valid(number), is_luhn_expected);
    }

    #[test]
    fn test_single_digit_strings_can_not_be_valid() {
        process_valid_case("1", false);
    }

    #[test]
    fn test_a_single_zero_is_invalid() {
        process_valid_case("0", false);
    }

    #[test]
    fn test_a_simple_valid_sin_that_remains_valid_if_reversed() {
        process_valid_case("059", true);
    }

    #[test]
    fn test_a_simple_valid_sin_that_becomes_invalid_if_reversed() {
        process_valid_case("59", true);
    }

    #[test]
    fn test_a_valid_canadian_sin() {
        process_valid_case("055 444 285", true);
    }

    #[test]
    fn test_invalid_canadian_sin() {
        process_valid_case("055 444 286", false);
    }

    #[test]
    fn test_invalid_credit_card() {
        process_valid_case("8273 1232 7352 0569", false);
    }

    #[test]
    fn test_valid_number_with_an_even_number_of_digits() {
        process_valid_case("095 245 88", true);
    }

    #[test]
    fn strings_that_contain_non_digits_are_invalid() {
        process_valid_case("055a 444 285", false);
    }

    #[test]
    fn test_valid_strings_with_punctuation_included_become_invalid() {
        process_valid_case("055-444-285", false);
    }

    #[test]
    fn symbols_are_invalid() {
        process_valid_case("055£ 444$ 285", false);
    }

    #[test]
    fn test_single_zero_with_space_is_invalid() {
        process_valid_case(" 0", false);
    }

    #[test]
    fn test_more_than_a_single_zero_is_valid() {
        process_valid_case("0000 0", true);
    }

    #[test]
    fn test_input_digit_9_is_correctly_converted_to_output_digit_9() {
        process_valid_case("091", true);
    }

    #[test]
    fn test_using_ascii_value_for_doubled_nondigit_isnt_allowed() {
        process_valid_case(":9", false);
    }

    #[test]
    fn test_valid_strings_with_a_nondigit_added_at_the_end_become_invalid() {
        process_valid_case("059a", false);
    }

    #[test]
    fn test_valid_strings_with_symbols_included_become_invalid() {
        process_valid_case("055# 444$ 285", false);
    }

    #[test]
    fn test_using_ascii_value_for_nondoubled_nondigit_isnt_allowed() {
        process_valid_case("055b 444 285", false);
    }

    #[test]
    fn test_valid_number_with_an_odd_number_of_spaces() {
        process_valid_case("234 567 891 234", true);
    }

    #[test]
    fn test_invalid_char_in_middle_with_sum_divisible_by_10_isnt_allowed() {
        process_valid_case("59%59", false);
    }

    #[test]
    fn test_valid_strings_with_numeric_unicode_characters_become_invalid() {
        process_valid_case("1249①", false);
    }
}
