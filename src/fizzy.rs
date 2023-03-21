// the PhantomData instances in this file are just to stop compiler complaints
// about missing generics; feel free to remove them

use std::ops::Rem;

/// A Matcher is a single rule of fizzbuzz: given a function on T, should
/// a word be substituted in? If yes, which word?
pub struct Matcher<T> {
    matcher: Box<dyn Fn(T) -> bool>,
    substitute: String,
}

impl<T> Matcher<T> {
    pub fn new<M, S>(matcher: M, subs: S) -> Matcher<T>
    where
        M: Fn(T) -> bool + 'static,
        S: ToString,
    {
        Matcher {
            matcher: Box::new(matcher),
            substitute: subs.to_string(),
        }
    }
}

/// A Fizzy is a set of matchers, which may be applied to an iterator.
///
/// Strictly speaking, it's usually more idiomatic to use `iter.map()` than to
/// consume an iterator with an `apply` method. Given a Fizzy instance, it's
/// pretty straightforward to construct a closure which applies it to all
/// elements of the iterator. However, we're using the `apply` pattern
/// here because it's a simpler interface for students to implement.
///
/// Also, it's a good excuse to try out using impl trait.
pub struct Fizzy<T>(Vec<Matcher<T>>);

impl<T> Fizzy<T>
where
    T: ToString + Copy + PartialEq,
{
    pub fn new() -> Self {
        Fizzy(Vec::new())
    }

    #[must_use]
    pub fn add_matcher(mut self, matcher: Matcher<T>) -> Self {
        self.0.push(matcher);
        self
    }

    pub fn apply<I>(self, iter: I) -> impl Iterator<Item = String>
    where
        I: IntoIterator<Item = T>,
    {
        iter.into_iter().map(move |n: T| {
            let matches: String = self
                .0
                .iter()
                .filter_map(|matcher| {
                    (matcher.matcher)(n).then_some(matcher.substitute.to_string())
                })
                .collect();
            if matches.is_empty() {
                n.to_string()
            } else {
                matches
            }
        })
    }
}

/// convenience function: return a Fizzy which applies the standard fizz-buzz rules
pub fn fizz_buzz<T>() -> Fizzy<T>
where
    u8: Into<T>,
    T: ToString + Copy + PartialEq + Rem<Output = T>,
{
    Fizzy::new()
        .add_matcher(Matcher::new(|n: T| n % 3.into() == 0.into(), "fizz"))
        .add_matcher(Matcher::new(|n: T| n % 5.into() == 0.into(), "buzz"))
}

#[cfg(test)]
mod tests {
    use crate::fizzy::*;

    macro_rules! expect {
        () => {
            vec![
                "1", "2", "fizz", "4", "buzz", "fizz", "7", "8", "fizz", "buzz", "11", "fizz",
                "13", "14", "fizzbuzz", "16",
            ]
        };
    }

    #[test]
    fn test_simple() {
        let got = fizz_buzz::<i32>().apply(1..=16).collect::<Vec<_>>();
        assert_eq!(expect!(), got);
    }

    #[test]
    fn test_u8() {
        let got = fizz_buzz::<u8>().apply(1_u8..=16).collect::<Vec<_>>();
        assert_eq!(expect!(), got);
    }

    #[test]
    fn test_u64() {
        let got = fizz_buzz::<u64>().apply(1_u64..=16).collect::<Vec<_>>();
        assert_eq!(expect!(), got);
    }

    #[test]
    fn test_nonsequential() {
        let collatz_12 = &[12, 6, 3, 10, 5, 16, 8, 4, 2, 1];
        let expect = vec![
            "fizz", "fizz", "fizz", "buzz", "buzz", "16", "8", "4", "2", "1",
        ];
        let got = fizz_buzz::<i32>()
            .apply(collatz_12.iter().cloned())
            .collect::<Vec<_>>();
        assert_eq!(expect, got);
    }
    #[test]
    fn test_custom() {
        let expect = vec![
            "1", "2", "Fizz", "4", "Buzz", "Fizz", "Bam", "8", "Fizz", "Buzz", "11", "Fizz", "13",
            "Bam", "BuzzFizz", "16",
        ];
        let fizzer: Fizzy<i32> = Fizzy::new()
            .add_matcher(Matcher::new(|n: i32| n % 5 == 0, "Buzz"))
            .add_matcher(Matcher::new(|n: i32| n % 3 == 0, "Fizz"))
            .add_matcher(Matcher::new(|n: i32| n % 7 == 0, "Bam"));
        let got = fizzer.apply(1..=16).collect::<Vec<_>>();
        assert_eq!(expect, got);
    }
    #[test]
    fn test_f64() {
        // a tiny bit more complicated becuase range isn't natively implemented on floats
        // NOTE: this test depends on a language feature introduced in Rust 1.34. If you
        // have an older compiler, upgrade. If you have an older compiler and cannot upgrade,
        // feel free to ignore this test.
        let got = fizz_buzz::<f64>()
            .apply(std::iter::successors(Some(1.0), |prev| Some(prev + 1.0)))
            .take(16)
            .collect::<Vec<_>>();
        assert_eq!(expect!(), got);
    }
    #[test]
    fn test_minimal_generic_bounds() {
        // NOTE: this test depends on a language feature introduced in Rust 1.34. If you
        // have an older compiler, upgrade. If you have an older compiler and cannot upgrade,
        // feel free to ignore this test.
        use std::fmt;
        use std::ops::{Add, Rem};
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        struct Fizzable(u8);
        impl From<u8> for Fizzable {
            fn from(i: u8) -> Fizzable {
                Fizzable(i)
            }
        }
        impl fmt::Display for Fizzable {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let Fizzable(ref n) = self;
                write!(f, "{n}")
            }
        }
        impl Add for Fizzable {
            type Output = Fizzable;
            fn add(self, rhs: Fizzable) -> Fizzable {
                let Fizzable(n1) = self;
                let Fizzable(n2) = rhs;
                Fizzable(n1 + n2)
            }
        }
        impl Rem for Fizzable {
            type Output = Fizzable;
            fn rem(self, rhs: Fizzable) -> Fizzable {
                let Fizzable(n1) = self;
                let Fizzable(n2) = rhs;
                Fizzable(n1 % n2)
            }
        }
        let got = fizz_buzz::<Fizzable>()
            .apply(std::iter::successors(Some(Fizzable(1)), |prev| {
                Some(*prev + 1.into())
            }))
            .take(16)
            .collect::<Vec<_>>();
        assert_eq!(expect!(), got);
    }
}
