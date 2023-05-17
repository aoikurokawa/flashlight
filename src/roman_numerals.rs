use std::fmt::{Display, Formatter, Result};

pub struct Roman {
    roman: String,
}

impl Display for Roman {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.roman)
    }
}

impl From<u32> for Roman {
    fn from(num: u32) -> Self {
        let arabic = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
        let roman = [
            "M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I",
        ];

        let mut result = String::new();
        let mut number = num;

        for i in 0..arabic.len() {
            while number >= arabic[i] {
                result += roman[i];
                number -= arabic[i];
            }
        }

        Self { roman: result }
    }
}

#[cfg(test)]
mod tests {
    use crate::roman_numerals::*;

    #[test]
    fn test_one() {
        assert_eq!("I", Roman::from(1).to_string());
    }

    #[test]
    fn test_two() {
        assert_eq!("II", Roman::from(2).to_string());
    }

    #[test]
    fn test_three() {
        assert_eq!("III", Roman::from(3).to_string());
    }

    #[test]
    fn test_four() {
        assert_eq!("IV", Roman::from(4).to_string());
    }

    #[test]
    fn test_five() {
        assert_eq!("V", Roman::from(5).to_string());
    }

    #[test]
    fn test_six() {
        assert_eq!("VI", Roman::from(6).to_string());
    }

    #[test]
    fn test_nine() {
        assert_eq!("IX", Roman::from(9).to_string());
    }

    #[test]
    fn test_twenty_seven() {
        assert_eq!("XXVII", Roman::from(27).to_string());
    }

    #[test]
    fn test_forty_eight() {
        assert_eq!("XLVIII", Roman::from(48).to_string());
    }

    #[test]
    fn test_fifty_nine() {
        assert_eq!("LIX", Roman::from(59).to_string());
    }

    #[test]
    fn test_ninety_three() {
        assert_eq!("XCIII", Roman::from(93).to_string());
    }

    #[test]
    fn test_141() {
        assert_eq!("CXLI", Roman::from(141).to_string());
    }

    #[test]
    fn test_163() {
        assert_eq!("CLXIII", Roman::from(163).to_string());
    }

    #[test]
    fn test_402() {
        assert_eq!("CDII", Roman::from(402).to_string());
    }

    #[test]
    fn test_575() {
        assert_eq!("DLXXV", Roman::from(575).to_string());
    }

    #[test]
    fn test_911() {
        assert_eq!("CMXI", Roman::from(911).to_string());
    }

    #[test]
    fn test_1024() {
        assert_eq!("MXXIV", Roman::from(1024).to_string());
    }

    #[test]
    fn test_3000() {
        assert_eq!("MMM", Roman::from(3000).to_string());
    }
}