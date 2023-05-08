use std::cmp::Ordering;

pub fn number(user_number: &str) -> Option<String> {
    let mut divided: Vec<String> = user_number
        .split(&[' ', '-', '.'])
        .map(|num| num.to_string())
        .collect();

    println!("{:?}", divided);

    match divided.len() {
        1 => match divided[0].len().cmp(&10) {
            Ordering::Less => return None,
            Ordering::Equal => return Some(divided[0].to_string()),
            Ordering::Greater => {
                if divided[0].len() == 11 && divided[0].starts_with('1') {
                    return Some(divided[0][1..].to_string());
                }
                return None;
            }
        },
        3 => {
            for (index, num) in divided.iter_mut().enumerate() {
                match index {
                    0 => {
                        if !is_valid(num, 3, true, false) {
                            return None;
                        }
                    }
                    1 => {
                        if !is_valid(num, 3, false, true) {
                            return None;
                        }
                    }
                    2 => {
                        if !is_valid(num, 4, false, false) {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
        }
        4 => {
            for (index, num) in divided.iter_mut().enumerate() {
                match index {
                    0 => {
                        if !num.contains('+') || !is_valid(num, 1, false, false) {
                            return None;
                        }
                        *num = "".to_string();
                    }
                    1 => {
                        if !is_valid(num, 3, false, false) {
                            return None;
                        }
                    }
                    2 => {
                        if !is_valid(num, 3, false, false) {
                            return None;
                        }
                    }
                    3 => {
                        if !is_valid(num, 4, false, false) {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
        }
        _ => {}
    }

    Some(divided.join(""))
}

fn is_valid(num: &mut String, count: usize, area_code: bool, exchange_code: bool) -> bool {
    let mut cleaned = String::new();
    for (idx, ch) in num
        .clone()
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .enumerate()
    {
        if area_code && idx == 0 && (ch == '0' || ch == '1') {
            return false;
        }
        if exchange_code && idx == 0 && (ch == '0' || ch == '1') {
            return false;
        }
        cleaned.push(ch);
    }
    if cleaned.len() != count {
        return false;
    }
    *num = cleaned;
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
