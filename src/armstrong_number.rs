pub fn is_armstrong_number(num: u32) -> bool {
    let mut temp_num = num;
    let mut digits = vec![];

    while temp_num > 0 {
        digits.push(temp_num % 10);

        temp_num /= 10;
    }

    digits.reverse();

    let mut sum: u64 = 0;
    let mut prev_sum = 0;
    let mut count = 1;
    loop {
        for digit in digits.iter() {
            sum += digit.pow(count) as u64;
        }

        if sum == num as u64 {
            return true;
        }

        if sum > num as u64 || prev_sum == sum {
            break;
        }

        count += 1;
        prev_sum = sum;
        sum = 0;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_armstrong_number_9() {
        let num = 9;
        let is_valid = is_armstrong_number(num);
        assert!(is_valid);
    }

    #[test]
    fn test_armstrong_number_10() {
        let num = 10;
        let is_not_valid = is_armstrong_number(num);
        assert!(is_not_valid == false);
    }

    #[test]
    fn test_armstrong_number_153() {
        let num = 153;
        let is_valid = is_armstrong_number(num);
        assert!(is_valid);
    }

    #[test]
    fn test_armstrong_number_3_999_999_999() {
        let num = 3_999_999_999;
        let is_not_valid = is_armstrong_number(num);
        assert!(is_not_valid == false);
    }
}
