use std::{
    collections::{HashMap, HashSet},
    iter::once,
};

const DIGITS: [i64; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

struct Permutations<T> {
    vec: Vec<T>,
    subsize: usize,
    first: bool,
}

impl<T: Clone + Ord> Iterator for Permutations<T> {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let n = self.vec.len();
        let r = self.subsize;
        if n == 0 || r == 0 || r > n {
            return None;
        }
        if self.first {
            self.vec.sort();
            self.first = false;
        } else if self.vec[r - 1] < self.vec[n - 1] {
            let mut j = r;
            while self.vec[j] <= self.vec[r - 1] {
                j += 1;
            }
            self.vec.swap(r - 1, j);
        } else {
            self.vec[r..n].reverse();
            let mut j = r - 1;
            while j > 0 && self.vec[j - 1] >= self.vec[j] {
                j -= 1;
            }

            if j == 0 {
                return None;
            }
            let mut l = n - 1;
            while self.vec[j - 1] >= self.vec[l] {
                l -= 1;
            }
            self.vec.swap(j - 1, l);
            self.vec[j..n].reverse();
        }
        Some(self.vec[0..r].to_vec())
    }
}

fn permutation<T: Clone + Ord>(s: &[T], subsize: usize) -> Permutations<T> {
    Permutations {
        vec: s.to_vec(),
        subsize,
        first: true,
    }
}

struct LetterSetup {
    letter: char,
    leading: bool,
    signature: i64,
}

fn distinct_letters(s: &str) -> Vec<char> {
    let letter_set: HashSet<_> = s.chars().filter(|c| c.is_alphabetic()).collect();
    letter_set.iter().cloned().collect()
}

fn str_value(s: &str, letter: char) -> i64 {
    let mut r = 0i64;
    let mut p = 1i64;
    for c in s.chars().rev() {
        if c == letter {
            r += p;
        }
        p *= 10;
    }
    r
}

fn calc_signature(components: &Vec<(&str, i64)>, letter: char) -> LetterSetup {
    let mut signature = 0i64;
    let mut leading = false;
    for (s, factor) in components {
        leading = leading || (s.chars().next().unwrap() == letter);
        signature += factor * str_value(s, letter);
    }

    LetterSetup {
        letter,
        leading,
        signature,
    }
}

pub fn solve(input: &str) -> Option<HashMap<char, u8>> {
    let lhs_rhs: Vec<&str> = input.split("==").map(|s| s.trim()).collect();
    let components: Vec<(&str, i64)> = once((lhs_rhs[1], -1))
        .chain(lhs_rhs[0].split('+').map(|s| s.trim()).map(|s| (s, 1)))
        .collect();
    let letters = distinct_letters(input);
    let setup: Vec<_> = letters.iter().map(|&c| calc_signature(&components, c)).collect();

    for permutation in permutation(&DIGITS, setup.len()) {
        if setup.iter().zip(permutation.iter()).all(|(letter_setup, &digit)| !letter_setup.leading || digit != 0) {
            let value: i64 = setup.iter().zip(permutation.iter()).map(|(letter_setup, &digit)| letter_setup.signature * digit).sum();
            if value == 0 {
                let char_map: HashMap<char, u8> = setup.iter().zip(permutation.iter()).map(|(letter_setup, &digit)| (letter_setup.letter, digit as u8)).collect();
                return Some(char_map)
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::alphametics;
    use std::collections::HashMap;

    fn assert_alphametic_solution_eq(puzzle: &str, solution: &[(char, u8)]) {
        let answer = alphametics::solve(puzzle);
        let solution: HashMap<char, u8> = solution.iter().cloned().collect();
        assert_eq!(answer, Some(solution));
    }
    #[test]
    fn test_with_three_letters() {
        assert_alphametic_solution_eq("I + BB == ILL", &[('I', 1), ('B', 9), ('L', 0)]);
    }
    #[test]
    fn test_must_have_unique_value_for_each_letter() {
        let answer = alphametics::solve("A == B");
        assert_eq!(answer, None);
    }
    #[test]
    fn test_leading_zero_solution_is_invalid() {
        let answer = alphametics::solve("ACA + DD == BD");
        assert_eq!(answer, None);
    }
    #[test]
    fn test_sum_must_be_wide_enough() {
        let answer = alphametics::solve("ABC + DEF == GH");
        assert_eq!(answer, None);
    }
    #[test]
    fn puzzle_with_two_digits_final_carry() {
        assert_alphametic_solution_eq(
            "A + A + A + A + A + A + A + A + A + A + A + B == BCC",
            &[('A', 9), ('B', 1), ('C', 0)],
        );
    }
    #[test]
    fn test_puzzle_with_four_letters() {
        assert_alphametic_solution_eq("AS + A == MOM", &[('A', 9), ('S', 2), ('M', 1), ('O', 0)]);
    }
    #[test]
    fn test_puzzle_with_six_letters() {
        assert_alphametic_solution_eq(
            "NO + NO + TOO == LATE",
            &[('N', 7), ('O', 4), ('T', 9), ('L', 1), ('A', 0), ('E', 2)],
        );
    }
    #[test]
    fn test_puzzle_with_seven_letters() {
        assert_alphametic_solution_eq(
            "HE + SEES + THE == LIGHT",
            &[
                ('E', 4),
                ('G', 2),
                ('H', 5),
                ('I', 0),
                ('L', 1),
                ('S', 9),
                ('T', 7),
            ],
        );
    }
    #[test]
    fn test_puzzle_with_eight_letters() {
        assert_alphametic_solution_eq(
            "SEND + MORE == MONEY",
            &[
                ('S', 9),
                ('E', 5),
                ('N', 6),
                ('D', 7),
                ('M', 1),
                ('O', 0),
                ('R', 8),
                ('Y', 2),
            ],
        );
    }
    #[test]
    fn test_puzzle_with_ten_letters() {
        assert_alphametic_solution_eq(
            "AND + A + STRONG + OFFENSE + AS + A + GOOD == DEFENSE",
            &[
                ('A', 5),
                ('D', 3),
                ('E', 4),
                ('F', 7),
                ('G', 8),
                ('N', 0),
                ('O', 2),
                ('R', 1),
                ('S', 6),
                ('T', 9),
            ],
        );
    }
    #[test]
    fn test_puzzle_with_ten_letters_and_199_addends() {
        assert_alphametic_solution_eq(
        "THIS + A + FIRE + THEREFORE + FOR + ALL + HISTORIES + I + TELL + A + TALE + THAT + FALSIFIES + ITS + TITLE + TIS + A + LIE + THE + TALE + OF + THE + LAST + FIRE + HORSES + LATE + AFTER + THE + FIRST + FATHERS + FORESEE + THE + HORRORS + THE + LAST + FREE + TROLL + TERRIFIES + THE + HORSES + OF + FIRE + THE + TROLL + RESTS + AT + THE + HOLE + OF + LOSSES + IT + IS + THERE + THAT + SHE + STORES + ROLES + OF + LEATHERS + AFTER + SHE + SATISFIES + HER + HATE + OFF + THOSE + FEARS + A + TASTE + RISES + AS + SHE + HEARS + THE + LEAST + FAR + HORSE + THOSE + FAST + HORSES + THAT + FIRST + HEAR + THE + TROLL + FLEE + OFF + TO + THE + FOREST + THE + HORSES + THAT + ALERTS + RAISE + THE + STARES + OF + THE + OTHERS + AS + THE + TROLL + ASSAILS + AT + THE + TOTAL + SHIFT + HER + TEETH + TEAR + HOOF + OFF + TORSO + AS + THE + LAST + HORSE + FORFEITS + ITS + LIFE + THE + FIRST + FATHERS + HEAR + OF + THE + HORRORS + THEIR + FEARS + THAT + THE + FIRES + FOR + THEIR + FEASTS + ARREST + AS + THE + FIRST + FATHERS + RESETTLE + THE + LAST + OF + THE + FIRE + HORSES + THE + LAST + TROLL + HARASSES + THE + FOREST + HEART + FREE + AT + LAST + OF + THE + LAST + TROLL + ALL + OFFER + THEIR + FIRE + HEAT + TO + THE + ASSISTERS + FAR + OFF + THE + TROLL + FASTS + ITS + LIFE + SHORTER + AS + STARS + RISE + THE + HORSES + REST + SAFE + AFTER + ALL + SHARE + HOT + FISH + AS + THEIR + AFFILIATES + TAILOR + A + ROOFS + FOR + THEIR + SAFE == FORTRESSES",
        &[
            ('A', 1),
            ('E', 0),
            ('F', 5),
            ('H', 8),
            ('I', 7),
            ('L', 2),
            ('O', 6),
            ('R', 3),
            ('S', 4),
            ('T', 9),
        ],
    );
    }
}
