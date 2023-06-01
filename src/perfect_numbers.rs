use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq)]
pub enum Classification {
    Abundant,
    Perfect,
    Deficient,
}

pub fn classify(num: u64) -> Option<Classification> {
    if num == 0 {
        None
    } else {
        match (1..num).filter(|&f| num % f == 0).sum::<u64>().cmp(&num) {
            Ordering::Less => Some(Classification::Deficient),
            Ordering::Equal => Some(Classification::Perfect),
            Ordering::Greater => Some(Classification::Abundant),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::perfect_numbers::{classify, Classification};

    macro_rules! tests {
    ($property_test_func:ident {
        $( $(#[$attr:meta])* $test_name:ident( $( $param:expr ),* ); )+
    }) => {
        $(
            $(#[$attr])*
            #[test]
            fn $test_name() {
                $property_test_func($( $param ),* )
            }
        )+
    }
}
    fn test_classification(num: u64, result: Classification) {
        assert_eq!(classify(num), Some(result));
    }

    #[test]
    fn basic() {
        assert_eq!(classify(0), None);
    }

    tests! {
        test_classification {
            #[ignore] test_1(1, Classification::Deficient);
            #[ignore] test_2(2, Classification::Deficient);
            #[ignore] test_4(4, Classification::Deficient);
            #[ignore] test_6(6, Classification::Perfect);
            #[ignore] test_12(12, Classification::Abundant);
            #[ignore] test_28(28, Classification::Perfect);
            #[ignore] test_30(30, Classification::Abundant);
            #[ignore] test_32(32, Classification::Deficient);
            #[ignore] test_33550335(33_550_335, Classification::Abundant);
            #[ignore] test_33550336(33_550_336, Classification::Perfect);
            #[ignore] test_33550337(33_550_337, Classification::Deficient);
        }
    }
}
