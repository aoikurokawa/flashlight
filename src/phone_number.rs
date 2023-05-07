pub fn number(user_number: &str) -> Option<String> {
    let mut divided: Vec<String> = user_number
        .split(&[' ', '-', '.'])
        .map(|num| num.to_string())
        .collect();

    println!("{:?}", divided);

    match divided.len() {
        1 => {
            if divided[0].len() > 10 {
                return None;
            }
            return Some(divided[0].to_string());
        }
        3 => {
            for (index, num) in divided.iter_mut().enumerate() {
                match index {
                    0 => {
                        // if num.starts_with("(") && num.ends_with(")") {

                        // }
                        if !is_valid(num, 3) {
                            return None;
                        }
                        *num = num.chars().filter(|ch| ch.is_digit(10)).collect();
                    }
                    1 => {
                        if !is_valid(num, 3) {
                            return None;
                        }
                        *num = num.chars().filter(|ch| ch.is_digit(10)).collect();
                    }
                    2 => {
                        if !is_valid(num, 4) {
                            return None;
                        }
                        *num = num.chars().filter(|ch| ch.is_digit(10)).collect();
                    }
                    _ => return None,
                }
            }
        }
        4 => {
            for (index, num) in divided.iter_mut().enumerate() {
                match index {
                    0 => {
                        if !num.contains('+') || !is_valid(num, 1) {
                            return None;
                        }
                        *num = num.chars().filter(|ch| ch.is_digit(10)).collect();
                    }
                    1 => {
                        if !is_valid(num, 3) {
                            return None;
                        }
                        *num = num.chars().filter(|ch| ch.is_digit(10)).collect();
                    }
                    2 => {
                        if !is_valid(num, 3) {
                            return None;
                        }
                        *num = num.chars().filter(|ch| ch.is_digit(10)).collect();
                    }
                    3 => {
                        if !is_valid(num, 4) {
                            return None;
                        }
                        *num = num.chars().filter(|ch| ch.is_digit(10)).collect();
                    }
                    _ => return None,
                }
            }
        }
        _ => {}
    }

    Some(divided.join(""))
}

fn is_valid(num: &str, count: usize) -> bool {
    let mut ret = String::new();
    for ch in num.chars() {
        if ch.is_digit(10) {
            ret.push(ch);
        }
    }
    if ret.len() != count {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::phone_number as phone;

    fn process_clean_case(number: &str, expected: Option<&str>) {
        assert_eq!(phone::number(number), expected.map(|x| x.to_string()));
    }

    #[test]
    fn test_cleans_the_number() {
        process_clean_case("(223) 456-7890", Some("2234567890"));
    }

    #[test]
    fn test_cleans_numbers_with_dots() {
        process_clean_case("223.456.7890", Some("2234567890"));
    }

    #[test]
    fn test_cleans_numbers_with_multiple_spaces() {
        process_clean_case("223 456   7890   ", Some("2234567890"));
    }

    #[test]
    fn test_invalid_when_9_digits() {
        process_clean_case("123456789", None);
    }

    #[test]
    fn test_invalid_when_11_digits_does_not_start_with_a_1() {
        process_clean_case("22234567890", None);
    }

    #[test]
    fn test_valid_when_11_digits_and_starting_with_1() {
        process_clean_case("12234567890", Some("2234567890"));
    }

    #[test]
    fn test_valid_when_11_digits_and_starting_with_1_even_with_punctuation() {
        process_clean_case("+1 (223) 456-7890", Some("2234567890"));
    }

    #[test]
    fn test_invalid_when_more_than_11_digits() {
        process_clean_case("321234567890", None);
    }

    #[test]
    fn test_invalid_with_letters() {
        process_clean_case("123-abc-7890", None);
    }

    #[test]
    fn test_invalid_with_punctuations() {
        process_clean_case("123-@:!-7890", None);
    }

    #[test]
    fn test_invalid_if_area_code_starts_with_1_on_valid_11digit_number() {
        process_clean_case("1 (123) 456-7890", None);
    }

    #[test]
    fn test_invalid_if_area_code_starts_with_0_on_valid_11digit_number() {
        process_clean_case("1 (023) 456-7890", None);
    }

    #[test]
    fn test_invalid_if_area_code_starts_with_1() {
        process_clean_case("(123) 456-7890", None);
    }

    #[test]
    fn test_invalid_if_exchange_code_starts_with_1() {
        process_clean_case("(223) 156-7890", None);
    }

    #[test]
    fn test_invalid_if_exchange_code_starts_with_0() {
        process_clean_case("(223) 056-7890", None);
    }

    #[test]
    fn test_invalid_if_exchange_code_starts_with_1_on_valid_11digit_number() {
        process_clean_case("1 (223) 156-7890", None);
    }

    #[test]
    fn test_invalid_if_exchange_code_starts_with_0_on_valid_11digit_number() {
        process_clean_case("1 (223) 056-7890", None);
    }

    #[test]
    fn test_invalid_if_area_code_starts_with_0() {
        process_clean_case("(023) 456-7890", None);
    }
}
