pub fn square_of_sum(n: u32) -> u32 {
    let mut sum: u32 = 0;

    for i in 1..=n {
        sum += i;
    }

    sum.pow(2)
}

pub fn sum_of_squares(n: u32) -> u32 {
    let mut sum = 0;

    for i in 1..=n {
        sum += i.pow(2);
    }

    sum
}

pub fn difference(n: u32) -> u32 {
    square_of_sum(n) - sum_of_squares(n)
}

#[cfg(test)]
mod tests {
    use crate::difference_of_squares as squares;

    #[test]
    fn test_square_of_sum_1() {
        assert_eq!(1, squares::square_of_sum(1));
    }
    #[test]
    fn test_square_of_sum_5() {
        assert_eq!(225, squares::square_of_sum(5));
    }
    #[test]
    fn test_square_of_sum_100() {
        assert_eq!(25_502_500, squares::square_of_sum(100));
    }
    #[test]
    fn test_sum_of_squares_1() {
        assert_eq!(1, squares::sum_of_squares(1));
    }
    #[test]
    fn test_sum_of_squares_5() {
        assert_eq!(55, squares::sum_of_squares(5));
    }
    #[test]
    fn test_sum_of_squares_100() {
        assert_eq!(338_350, squares::sum_of_squares(100));
    }
    #[test]
    fn test_difference_1() {
        assert_eq!(0, squares::difference(1));
    }
    #[test]
    fn test_difference_5() {
        assert_eq!(170, squares::difference(5));
    }
    #[test]
    fn test_difference_100() {
        assert_eq!(25_164_150, squares::difference(100));
    }
}
