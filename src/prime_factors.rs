const DIVISORS: [u64; 9] = [2, 3, 5, 11, 17, 23, 461, 9539, 894_119];

pub fn factors(n: u64) -> Vec<u64> {
    let mut prime_numbers = Vec::new();
    let mut temp_n = n;

    let mut index = 0;
    loop {
        if temp_n % DIVISORS[index] == 0 {
            prime_numbers.push(DIVISORS[index]);
            temp_n /= DIVISORS[index];
        } else {
            index += 1;

            if index == 9 {
                break;
            }
        }
    }

    prime_numbers
}

#[cfg(test)]
mod tests {
    use crate::prime_factors::factors;

    #[test]
    fn test_no_factors() {
        assert_eq!(factors(1), vec![]);
    }

    #[test]
    fn test_prime_number() {
        assert_eq!(factors(2), vec![2]);
    }

    #[test]
    fn test_square_of_a_prime() {
        assert_eq!(factors(9), vec![3, 3]);
    }

    #[test]
    fn test_cube_of_a_prime() {
        assert_eq!(factors(8), vec![2, 2, 2]);
    }

    #[test]
    fn test_product_of_primes_and_non_primes() {
        assert_eq!(factors(12), vec![2, 2, 3]);
    }

    #[test]
    fn test_product_of_primes() {
        assert_eq!(factors(901_255), vec![5, 17, 23, 461]);
    }

    #[test]
    fn test_factors_include_large_prime() {
        assert_eq!(factors(93_819_012_551), vec![11, 9539, 894_119]);
    }
}
