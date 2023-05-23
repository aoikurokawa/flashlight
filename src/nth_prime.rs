pub fn nth(n: u32) -> u32 {
    let mut prime_numbers = Vec::new();
    for i in 0.. {
        if is_prime(i) {
            prime_numbers.push(i);

            match prime_numbers.get(n as usize) {
                Some(num) => return *num,
                None => continue,
            }
        }
    }
    0
}

fn is_prime(n: u32) -> bool {
    if n <= 1 {
        return false;
    }
    for a in 2..n {
        if n % a == 0 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::nth_prime as np;

    #[test]
    fn test_first_prime() {
        assert_eq!(np::nth(0), 2);
    }

    #[test]
    fn test_second_prime() {
        assert_eq!(np::nth(1), 3);
    }

    #[test]
    fn test_sixth_prime() {
        assert_eq!(np::nth(5), 13);
    }

    #[test]
    fn test_big_prime() {
        assert_eq!(np::nth(10_000), 104_743);
    }
}
