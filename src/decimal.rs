use std::cmp::{max, min, Ordering};
use std::fmt::Display;
use std::iter;
use std::ops::{Add, Mul, Sub};

type Digit = u8;
const BASE: Digit = 10;

/// Type implementing arbitrary-precision decimal arithmetic
#[derive(Debug, Clone)]
pub struct Decimal {
    negative: bool,
    power: isize,
    digits: Vec<Digit>,
}

impl Add for Decimal {
    type Output = Decimal;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = Decimal::new();
        let (num_a, num_b) = (self.clean(), rhs.clean());

        result.power = min(num_a.power, num_b.power);

        if num_a.negative && num_b.negative {
            result.negative = true;
        } else if num_a.negative {
            return num_b.sub(num_a.flip_sign());
        } else if num_b.negative {
            return num_a.sub(num_b.flip_sign());
        }

        let mut carry = 0;
        for (a, b) in Decimal::make_equal_digits(&num_a, &num_b) {
            result.digits.push((a + b + carry) % BASE);
            carry = (a + b + carry) / BASE;
        }

        if carry != 0 {
            result.digits.push(carry);
        }
        result.clean()
    }
}

impl Sub for Decimal {
    type Output = Decimal;

    fn sub(self, rhs: Self) -> Self::Output {
        if rhs.negative {
            return self.add(rhs.flip_sign());
        } else if self < rhs {
            return rhs.sub(self).flip_sign();
        }

        let mut result = Decimal::new();
        let (num_a, num_b) = (self.clean(), rhs.clean());
        result.power = min(num_a.power, num_b.power);
        let mut carry = 0;
        for (a, b) in Decimal::make_equal_digits(&num_a, &num_b) {
            if a >= b + carry {
                result.digits.push(a - b - carry);
                carry = 0;
            } else {
                result.digits.push(a + BASE - b - carry);
                carry = 1;
            }
        }
        result.clean()
    }
}

impl Mul for Decimal {
    type Output = Decimal;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = Decimal::new();
        let (num_a, num_b) = (self.clean(), rhs.clean());

        let power = num_a.power + num_b.power;
        result.power = power;
        for (p, a) in num_a.digits.iter().enumerate() {
            let mut step = Decimal::new();
            step.power = (p as isize) + power;
            let mut carry = 0;
            for b in &num_b.digits {
                step.digits.push((a * b + carry) % BASE);
                carry = (a * b + carry) / BASE;
            }
            if carry > 0 {
                step.digits.push(carry);
            }
            result = step.add(result);
        }

        result.negative = num_a.negative != num_b.negative;
        result.clean()
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.clean().partial_cmp(&other.clean()) == Some(Ordering::Equal)
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.negative && !other.negative {
            return Some(Ordering::Less);
        } else if !self.negative && other.negative {
            return Some(Ordering::Greater);
        }

        for &(a, b) in Decimal::make_equal_digits(self, other).iter().rev() {
            if a != b {
                if !self.negative {
                    return a.partial_cmp(&b);
                } else {
                    return b.partial_cmp(&a);
                }
            }
        }

        Some(Ordering::Equal)
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let num = self
            .digits
            .iter()
            .rev()
            .map(|d| d.to_string())
            .collect::<Vec<String>>()
            .join("");

        if self.negative {
            write!(f, "-{}x10^{}", num, self.power)
        } else {
            write!(f, "{}x10^{}", num, self.power)
        }
    }
}

impl Decimal {
    fn new() -> Self {
        Self {
            negative: false,
            power: 0,
            digits: Vec::new(),
        }
    }

    pub fn try_from(input: &str) -> Option<Decimal> {
        let mut number = Decimal::new();
        let mut start = 0;
        if input.starts_with('-') {
            start = 1;
            number.negative = true;
        } else if input.starts_with('+') {
            start = 1;
        }

        for (i, digit) in input[start..].chars().rev().enumerate() {
            if digit == '.' {
                number.power = -(i as isize);
            } else if let Some(d) = digit.to_digit(BASE as u32) {
                number.digits.push(d as Digit);
            } else {
                return None;
            }
        }
        Some(number.clean())
    }

    fn flip_sign(&self) -> Self {
        let mut result = self.clone();
        result.negative = !self.negative;
        result
    }

    fn make_equal_digits(lhs: &Decimal, rhs: &Decimal) -> Vec<(Digit, Digit)> {
        let mut result = Decimal::new();
        if lhs.power < rhs.power {
            return Decimal::make_equal_digits(rhs, lhs)
                .iter()
                .map(|&(a, b)| (b, a))
                .collect();
        }

        result.digits = iter::repeat(0)
            .take((lhs.power - rhs.power) as usize)
            .chain(lhs.digits.clone())
            .collect();
        result.power = rhs.power;
        Decimal::make_digits(&result, rhs)
    }

    fn make_digits(lhs: &Decimal, rhs: &Decimal) -> Vec<(Digit, Digit)> {
        let mut result = Vec::new();
        for i in 0..max(lhs.digits.len(), rhs.digits.len()) {
            result.push((
                *lhs.digits.get(i).unwrap_or(&0),
                *rhs.digits.get(i).unwrap_or(&0),
            ));
        }
        result
    }

    fn clean(&self) -> Self {
        let mut result = self.clone();
        while let Some(&d) = result.digits.last() {
            if d != 0 {
                break;
            }
            result.digits.pop();
        }
        while let Some(&d) = result.digits.first() {
            if d != 0 {
                break;
            }
            result.power += 1;
            result.digits.remove(0);
        }
        if result.digits.is_empty() {
            result.digits = vec![0];
            result.power = 0;
            result.negative = false;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::decimal::Decimal;

    fn decimal(input: &str) -> Decimal {
        Decimal::try_from(input).expect("That was supposed to be a valid value")
    }

    /// Some big and precise values we can use for testing. [0] + [1] == [2]

    const BIGS: [&str; 3] = [
        "100000000000000000000000000000000000000000000.00000000000000000000000000000000000000001",
        "100000000000000000000000000000000000000000000.00000000000000000000000000000000000000002",
        "200000000000000000000000000000000000000000000.00000000000000000000000000000000000000003",
    ];

    // test simple properties of required operations

    #[test]
    fn test_eq() {
        assert!(decimal("0.0") == decimal("0.0"));

        assert!(decimal("1.0") == decimal("1.0"));

        for big in BIGS.iter() {
            assert!(decimal(big) == decimal(big));
        }
    }

    #[test]
    fn test_ne() {
        assert!(decimal("0.0") != decimal("1.0"));

        assert!(decimal(BIGS[0]) != decimal(BIGS[1]));
    }

    #[test]
    fn test_gt() {
        for slice_2 in BIGS.windows(2) {
            assert!(decimal(slice_2[1]) > decimal(slice_2[0]));
        }
    }

    #[test]
    fn test_lt() {
        for slice_2 in BIGS.windows(2) {
            assert!(decimal(slice_2[0]) < decimal(slice_2[1]));
        }
    }

    #[test]
    fn test_add() {
        assert_eq!(decimal("0.1") + decimal("0.2"), decimal("0.3"));

        assert_eq!(decimal(BIGS[0]) + decimal(BIGS[1]), decimal(BIGS[2]));

        assert_eq!(decimal(BIGS[1]) + decimal(BIGS[0]), decimal(BIGS[2]));
    }

    #[test]
    fn test_sub() {
        assert_eq!(decimal(BIGS[2]) - decimal(BIGS[1]), decimal(BIGS[0]));

        assert_eq!(decimal(BIGS[2]) - decimal(BIGS[0]), decimal(BIGS[1]));
    }

    #[test]
    fn test_mul() {
        for big in BIGS.iter() {
            assert_eq!(decimal(big) * decimal("2"), decimal(big) + decimal(big));
        }
    }

    // test identities

    #[test]
    fn test_add_id() {
        assert_eq!(decimal("1.0") + decimal("0.0"), decimal("1.0"));

        assert_eq!(decimal("0.1") + decimal("0.0"), decimal("0.1"));

        assert_eq!(decimal("0.0") + decimal("1.0"), decimal("1.0"));

        assert_eq!(decimal("0.0") + decimal("0.1"), decimal("0.1"));
    }

    #[test]
    fn test_sub_id() {
        assert_eq!(decimal("1.0") - decimal("0.0"), decimal("1.0"));

        assert_eq!(decimal("0.1") - decimal("0.0"), decimal("0.1"));
    }

    #[test]
    fn test_mul_id() {
        assert_eq!(decimal("2.1") * decimal("1.0"), decimal("2.1"));

        assert_eq!(decimal("1.0") * decimal("2.1"), decimal("2.1"));
    }

    #[test]
    fn test_gt_positive_and_zero() {
        assert!(decimal("1.0") > decimal("0.0"));

        assert!(decimal("0.1") > decimal("0.0"));
    }

    #[test]
    fn test_gt_negative_and_zero() {
        assert!(decimal("0.0") > decimal("-0.1"));

        assert!(decimal("0.0") > decimal("-1.0"));
    }

    // tests of arbitrary precision behavior

    #[test]
    fn test_add_uneven_position() {
        assert_eq!(decimal("0.1") + decimal("0.02"), decimal("0.12"));
    }

    #[test]
    fn test_eq_vary_sig_digits() {
        assert!(decimal("0") == decimal("0000000000000.0000000000000000000000"));

        assert!(decimal("1") == decimal("00000000000000001.000000000000000000"));
    }

    #[test]
    fn test_add_vary_precision() {
        assert_eq!(
            decimal("100000000000000000000000000000000000000000000")
                + decimal("0.00000000000000000000000000000000000000001"),
            decimal(BIGS[0])
        )
    }

    #[test]
    fn test_cleanup_precision() {
        assert_eq!(
            decimal("10000000000000000000000000000000000000000000000.999999999999999999999999998",)
                + decimal(
                    "10000000000000000000000000000000000000000000000.000000000000000000000000002",
                ),
            decimal("20000000000000000000000000000000000000000000001")
        )
    }

    #[test]
    fn test_gt_varying_positive_precisions() {
        assert!(decimal("1.1") > decimal("1.01"));

        assert!(decimal("1.01") > decimal("1.0"));

        assert!(decimal("1.0") > decimal("0.1"));

        assert!(decimal("0.1") > decimal("0.01"));
    }

    #[test]
    fn test_gt_positive_and_negative() {
        assert!(decimal("1.0") > decimal("-1.0"));

        assert!(decimal("1.1") > decimal("-1.1"));

        assert!(decimal("0.1") > decimal("-0.1"));
    }

    #[test]
    fn test_gt_varying_negative_precisions() {
        assert!(decimal("-0.01") > decimal("-0.1"));

        assert!(decimal("-0.1") > decimal("-1.0"));

        assert!(decimal("-1.0") > decimal("-1.01"));

        assert!(decimal("-1.01") > decimal("-1.1"));
    }

    // test signed properties

    #[test]
    fn test_negatives() {
        assert!(Decimal::try_from("-1").is_some());

        assert_eq!(decimal("0") - decimal("1"), decimal("-1"));

        assert_eq!(decimal("5.5") + decimal("-6.5"), decimal("-1"));
    }

    #[test]
    fn test_explicit_positive() {
        assert_eq!(decimal("+1"), decimal("1"));

        assert_eq!(decimal("+2.0") - decimal("-0002.0"), decimal("4"));
    }

    #[test]
    fn test_multiply_by_negative() {
        assert_eq!(decimal("5") * decimal("-0.2"), decimal("-1"));

        assert_eq!(decimal("-20") * decimal("-0.2"), decimal("4"));
    }

    #[test]
    fn test_simple_partial_cmp() {
        assert!(decimal("1.0") < decimal("1.1"));

        assert!(decimal("0.00000000000000000000001") > decimal("-20000000000000000000000000000"));
    }

    // test carrying rules

    // these tests are designed to ensure correctness of implementations for which the

    // integer and fractional parts of the number are stored separately

    #[test]
    fn test_carry_into_integer() {
        assert_eq!(decimal("0.901") + decimal("0.1"), decimal("1.001"))
    }

    #[test]
    fn test_carry_into_fractional_with_digits_to_right() {
        assert_eq!(decimal("0.0901") + decimal("0.01"), decimal("0.1001"))
    }

    #[test]
    fn test_add_carry_over_negative() {
        assert_eq!(decimal("-1.99") + decimal("-0.01"), decimal("-2.0"))
    }

    #[test]
    fn test_sub_carry_over_negative() {
        assert_eq!(decimal("-1.99") - decimal("0.01"), decimal("-2.0"))
    }

    #[test]
    fn test_add_carry_over_negative_with_fractional() {
        assert_eq!(decimal("-1.99") + decimal("-0.02"), decimal("-2.01"))
    }

    #[test]
    fn test_sub_carry_over_negative_with_fractional() {
        assert_eq!(decimal("-1.99") - decimal("0.02"), decimal("-2.01"))
    }

    #[test]
    fn test_carry_from_rightmost_one() {
        assert_eq!(decimal("0.09") + decimal("0.01"), decimal("0.1"))
    }

    #[test]
    fn test_carry_from_rightmost_more() {
        assert_eq!(decimal("0.099") + decimal("0.001"), decimal("0.1"))
    }

    #[test]
    fn test_carry_from_rightmost_into_integer() {
        assert_eq!(decimal("0.999") + decimal("0.001"), decimal("1.0"))
    }

    // test arithmetic borrow rules

    #[test]
    fn test_add_borrow() {
        assert_eq!(decimal("0.01") + decimal("-0.0001"), decimal("0.0099"))
    }

    #[test]
    fn test_sub_borrow() {
        assert_eq!(decimal("0.01") - decimal("0.0001"), decimal("0.0099"))
    }

    #[test]
    fn test_add_borrow_integral() {
        assert_eq!(decimal("1.0") + decimal("-0.01"), decimal("0.99"))
    }

    #[test]
    fn test_sub_borrow_integral() {
        assert_eq!(decimal("1.0") - decimal("0.01"), decimal("0.99"))
    }

    #[test]
    fn test_add_borrow_integral_zeroes() {
        assert_eq!(decimal("1.0") + decimal("-0.99"), decimal("0.01"))
    }

    #[test]
    fn test_sub_borrow_integral_zeroes() {
        assert_eq!(decimal("1.0") - decimal("0.99"), decimal("0.01"))
    }

    #[test]
    fn test_borrow_from_negative() {
        assert_eq!(decimal("-1.0") + decimal("0.01"), decimal("-0.99"))
    }

    #[test]
    fn test_add_into_fewer_digits() {
        assert_eq!(decimal("0.011") + decimal("-0.001"), decimal("0.01"))
    }

    // misc tests of arithmetic properties

    #[test]
    fn test_sub_into_fewer_digits() {
        assert_eq!(decimal("0.011") - decimal("0.001"), decimal("0.01"))
    }

    #[test]
    fn test_add_away_decimal() {
        assert_eq!(decimal("1.1") + decimal("-0.1"), decimal("1.0"))
    }

    #[test]
    fn test_sub_away_decimal() {
        assert_eq!(decimal("1.1") - decimal("0.1"), decimal("1.0"))
    }
}
