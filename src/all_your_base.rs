#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InputBase,
    OutputBase,
    Digit(u32),
}

///
/// Convert a number between two bases.
///
/// A number is any slice of digits.
/// A digit is any unsigned integer (e.g. u8, u16, u32, u64, or usize).
/// Bases are specified as unsigned integers.
///
/// Return an `Err(.)` if the conversion is impossible.
/// The tests do not test for specific values inside the `Err(.)`.
///
///
/// You are allowed to change the function signature as long as all test still pass.
///
///
/// Example:
/// Input
///   number: &[4, 2]
///   from_base: 10
///   to_base: 2
/// Result
///   Ok(vec![1, 0, 1, 0, 1, 0])
///
/// The example corresponds to converting the number 42 from decimal
/// which is equivalent to 101010 in binary.
///
///
/// Notes:
///  * The empty slice ( "[]" ) is equal to the number 0.
///  * Never output leading 0 digits, unless the input number is 0, in which the output must be `[0]`.
///    However, your function must be able to process input with leading 0 digits.
///
pub fn convert(numbers: &[u32], from_base: u32, to_base: u32) -> Result<Vec<u32>, Error> {
    if from_base < 2 {
        return Err(Error::InputBase);
    }

    if to_base < 2 {
        return Err(Error::OutputBase);
    }

    let mut output = Vec::new();
    let mut base_num = 0;

    let numbers = if numbers.is_empty() {
        vec![0]
    } else {
        numbers.to_vec()
    };

    for (index, num) in numbers.iter().enumerate() {
        if *num >= from_base {
            return Err(Error::Digit(*num));
        }

        let num_from = num * from_base.pow(numbers.len() as u32 - index as u32 - 1);
        base_num += num_from;
    }

    println!("Base num: {base_num}");

    if base_num == 0 {
        output.push(0);
    } else {
        while base_num > 0 {
            output.push(base_num % to_base);

            if base_num > to_base {
                base_num /= to_base;
            } else {
                if to_base == 2 {
                    output.push(base_num / to_base);
                }
                break;
            }
        }
    }

    output.reverse();
    Ok(output)
}

#[cfg(test)]
mod tests {
    use crate::all_your_base as ayb;

    #[test]
    fn single_bit_one_to_decimal() {
        let input_base = 2;
        let input_digits = &[1];
        let output_base = 10;
        let output_digits = vec![1];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn binary_to_single_decimal() {
        let input_base = 2;
        let input_digits = &[1, 0, 1];
        let output_base = 10;
        let output_digits = vec![5];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn single_decimal_to_binary() {
        let input_base = 10;
        let input_digits = &[5];
        let output_base = 2;
        let output_digits = vec![1, 0, 1];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn binary_to_multiple_decimal() {
        let input_base = 2;
        let input_digits = &[1, 0, 1, 0, 1, 0];
        let output_base = 10;
        let output_digits = vec![4, 2];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn decimal_to_binary() {
        let input_base = 10;
        let input_digits = &[4, 2];
        let output_base = 2;
        let output_digits = vec![1, 0, 1, 0, 1, 0];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn trinary_to_hexadecimal() {
        let input_base = 3;
        let input_digits = &[1, 1, 2, 0];
        let output_base = 16;
        let output_digits = vec![2, 10];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn hexadecimal_to_trinary() {
        let input_base = 16;
        let input_digits = &[2, 10];
        let output_base = 3;
        let output_digits = vec![1, 1, 2, 0];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn fifteen_bit_integer() {
        let input_base = 97;
        let input_digits = &[3, 46, 60];
        let output_base = 73;
        let output_digits = vec![6, 10, 45];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn empty_list() {
        let input_base = 2;
        let input_digits = &[];
        let output_base = 10;
        let output_digits = vec![0];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn single_zero() {
        let input_base = 10;
        let input_digits = &[0];
        let output_base = 2;
        let output_digits = vec![0];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn multiple_zeros() {
        let input_base = 10;
        let input_digits = &[0, 0, 0];
        let output_base = 2;
        let output_digits = vec![0];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn leading_zeros() {
        let input_base = 7;
        let input_digits = &[0, 6, 0];
        let output_base = 10;
        let output_digits = vec![4, 2];
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Ok(output_digits)
        );
    }
    #[test]
    fn invalid_positive_digit() {
        let input_base = 2;
        let input_digits = &[1, 2, 1, 0, 1, 0];
        let output_base = 10;
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Err(ayb::Error::Digit(2))
        );
    }
    #[test]
    fn input_base_is_one() {
        let input_base = 1;
        let input_digits = &[];
        let output_base = 10;
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Err(ayb::Error::InputBase)
        );
    }
    #[test]
    fn output_base_is_one() {
        let input_base = 2;
        let input_digits = &[1, 0, 1, 0, 1, 0];
        let output_base = 1;
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Err(ayb::Error::OutputBase)
        );
    }
    #[test]
    fn input_base_is_zero() {
        let input_base = 0;
        let input_digits = &[];
        let output_base = 10;
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Err(ayb::Error::InputBase)
        );
    }
    #[test]
    fn output_base_is_zero() {
        let input_base = 10;
        let input_digits = &[7];
        let output_base = 0;
        assert_eq!(
            ayb::convert(input_digits, input_base, output_base),
            Err(ayb::Error::OutputBase)
        );
    }
}
