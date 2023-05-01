pub struct Luhn {
    code: String,
}

impl Luhn {
    pub fn is_valid(&self) -> bool {
        self.code
            .chars()
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
}

/// Here is the example of how the From trait could be implemented
/// for the &str type. Naturally, you can implement this trait
/// by hand for the every other type presented in the test suite,
/// but your solution will fail if a new type is presented.
/// Perhaps there exists a better solution for this problem?
impl<T> From<T> for Luhn
where
    T: ToString,
{
    fn from(input: T) -> Self {
        Self {
            code: input.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::luhn_from::*;

    #[test]
    fn you_can_validate_from_a_str() {
        let valid = Luhn::from("046 454 286");

        let invalid = Luhn::from("046 454 287");

        assert!(valid.is_valid());

        assert!(!invalid.is_valid());
    }

    #[test]
    fn you_can_validate_from_a_string() {
        let valid = Luhn::from(String::from("046 454 286"));

        let invalid = Luhn::from(String::from("046 454 287"));

        assert!(valid.is_valid());

        assert!(!invalid.is_valid());
    }

    #[test]
    fn you_can_validate_from_a_u8() {
        let valid = Luhn::from(240u8);

        let invalid = Luhn::from(241u8);

        assert!(valid.is_valid());

        assert!(!invalid.is_valid());
    }

    #[test]
    fn you_can_validate_from_a_u16() {
        let valid = Luhn::from(64_436u16);

        let invalid = Luhn::from(64_437u16);

        assert!(valid.is_valid());

        assert!(!invalid.is_valid());
    }

    #[test]
    fn you_can_validate_from_a_u32() {
        let valid = Luhn::from(46_454_286u32);

        let invalid = Luhn::from(46_454_287u32);

        assert!(valid.is_valid());

        assert!(!invalid.is_valid());
    }

    #[test]
    fn you_can_validate_from_a_u64() {
        let valid = Luhn::from(8273_1232_7352_0562u64);

        let invalid = Luhn::from(8273_1232_7352_0569u64);

        assert!(valid.is_valid());

        assert!(!invalid.is_valid());
    }

    #[test]
    fn you_can_validate_from_a_usize() {
        let valid = Luhn::from(8273_1232_7352_0562usize);

        let invalid = Luhn::from(8273_1232_7352_0569usize);

        assert!(valid.is_valid());

        assert!(!invalid.is_valid());
    }

    #[test]
    fn single_digit_string_is_invalid() {
        assert!(!Luhn::from("1").is_valid());
    }

    #[test]
    fn single_zero_string_is_invalid() {
        assert!(!Luhn::from("0").is_valid());
    }

    #[test]
    fn valid_canadian_sin_is_valid() {
        assert!(Luhn::from("046 454 286").is_valid());
    }

    #[test]
    fn invalid_canadian_sin_is_invalid() {
        assert!(!Luhn::from("046 454 287").is_valid());
    }

    #[test]
    fn invalid_credit_card_is_invalid() {
        assert!(!Luhn::from("8273 1232 7352 0569").is_valid());
    }

    #[test]
    fn strings_that_contain_non_digits_are_invalid() {
        assert!(!Luhn::from("046a 454 286").is_valid());
    }
}
