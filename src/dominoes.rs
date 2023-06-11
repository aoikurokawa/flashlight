pub fn chain(input: &[(u8, u8)]) -> Option<Vec<(u8, u8)>> {
    let input: Vec<(u8, u8)> = input.to_vec();
    let result = Vec::new();

    chain_body(input, result)
}

fn chain_body(input: Vec<(u8, u8)>, result: Vec<(u8, u8)>) -> Option<Vec<(u8, u8)>> {
    if input.is_empty() {
        match (result.first(), result.last()) {
            (Some((h, _)), Some((_, t))) if *h == *t => Some(result),
            (None, None) => Some(result),
            _ => None,
        }
    } else if result.is_empty() {
        for i in 0..input.len() {
            let mut new_input = input.clone();
            let new_result = vec![new_input.swap_remove(i)];
            if let r @ Some(_) = chain_body(new_input, new_result) {
                return r;
            }
        }
        None
    } else {
        let tail = result.last().unwrap().1;
        for i in 0..input.len() {
            if input[i].0 == tail || input[i].1 == tail {
                let other = if input[i].0 == tail {
                    input[i].1
                } else {
                    input[i].0
                };
                let mut new_input = input.clone();
                new_input.swap_remove(i);
                let new_result = [result.clone(), vec![(tail, other)]].concat();
                if let r @ Some(_) = chain_body(new_input, new_result) {
                    return r;
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::dominoes;

    type Domino = (u8, u8);

    #[derive(Debug)]
    enum CheckResult {
        GotInvalid, // chain returned None

        Correct,

        ChainingFailure(Vec<Domino>), // failure to match the dots at the right side of one domino with

        // the one on the left side of the next
        LengthMismatch(Vec<Domino>),

        DominoMismatch(Vec<Domino>), // different dominoes are used in input and output
    }

    fn normalize(d: Domino) -> Domino {
        match d {
            (m, n) if m > n => (n, m),

            (m, n) => (m, n),
        }
    }

    fn check(input: &[Domino]) -> CheckResult {
        let output = match dominoes::chain(input) {
            None => return CheckResult::GotInvalid,

            Some(o) => o,
        };

        if input.len() != output.len() {
            return CheckResult::LengthMismatch(output);
        } else if input.is_empty() {
            // and thus output.is_empty()

            return CheckResult::Correct;
        }

        let mut output_sorted = output
            .iter()
            .map(|&d| normalize(d))
            .collect::<Vec<Domino>>();

        output_sorted.sort_unstable();

        let mut input_sorted = input.iter().map(|&d| normalize(d)).collect::<Vec<Domino>>();

        input_sorted.sort_unstable();

        if input_sorted != output_sorted {
            return CheckResult::DominoMismatch(output);
        }

        // both input and output have at least 1 element

        // This essentially puts the first element after the last one, thereby making it

        // easy to check whether the domino chains "wraps around".

        let mut fail = false;

        {
            let mut n = output[0].1;

            let iter = output.iter().skip(1).chain(output.iter().take(1));

            for &(first, second) in iter {
                if n != first {
                    fail = true;

                    break;
                }

                n = second
            }
        }

        if fail {
            CheckResult::ChainingFailure(output)
        } else {
            CheckResult::Correct
        }
    }

    fn assert_correct(input: &[Domino]) {
        match check(input) {
            CheckResult::Correct => (),

            CheckResult::GotInvalid => panic!("Unexpectedly got invalid on input {input:?}"),

            CheckResult::ChainingFailure(output) => {
                panic!("Chaining failure for input {input:?}, output {output:?}")
            }

            CheckResult::LengthMismatch(output) => {
                panic!("Length mismatch for input {input:?}, output {output:?}")
            }

            CheckResult::DominoMismatch(output) => {
                panic!("Domino mismatch for input {input:?}, output {output:?}")
            }
        }
    }

    #[test]
    fn empty_input_empty_output() {
        let input = &[];

        assert_eq!(dominoes::chain(input), Some(vec![]));
    }

    #[test]
    fn singleton_input_singleton_output() {
        let input = &[(1, 1)];

        assert_correct(input);
    }

    #[test]
    fn singleton_that_cant_be_chained() {
        let input = &[(1, 2)];

        assert_eq!(dominoes::chain(input), None);
    }

    #[test]
    fn no_repeat_numbers() {
        let input = &[(1, 2), (3, 1), (2, 3)];

        assert_correct(input);
    }

    #[test]
    fn can_reverse_dominoes() {
        let input = &[(1, 2), (1, 3), (2, 3)];

        assert_correct(input);
    }

    #[test]
    fn no_chains() {
        let input = &[(1, 2), (4, 1), (2, 3)];

        assert_eq!(dominoes::chain(input), None);
    }

    #[test]
    fn disconnected_simple() {
        let input = &[(1, 1), (2, 2)];

        assert_eq!(dominoes::chain(input), None);
    }

    #[test]
    fn disconnected_double_loop() {
        let input = &[(1, 2), (2, 1), (3, 4), (4, 3)];

        assert_eq!(dominoes::chain(input), None);
    }

    #[test]
    fn disconnected_single_isolated() {
        let input = &[(1, 2), (2, 3), (3, 1), (4, 4)];

        assert_eq!(dominoes::chain(input), None);
    }

    #[test]
    fn need_backtrack() {
        let input = &[(1, 2), (2, 3), (3, 1), (2, 4), (2, 4)];

        assert_correct(input);
    }

    #[test]
    fn separate_loops() {
        let input = &[(1, 2), (2, 3), (3, 1), (1, 1), (2, 2), (3, 3)];

        assert_correct(input);
    }

    #[test]
    fn pop_same_value_first() {
        let input = &[(2, 3), (3, 1), (1, 1), (2, 2), (3, 3), (2, 1)];

        assert_correct(input);
    }

    #[test]
    fn nine_elements() {
        let input = &[
            (1, 2),
            (5, 3),
            (3, 1),
            (1, 2),
            (2, 4),
            (1, 6),
            (2, 3),
            (3, 4),
            (5, 6),
        ];

        assert_correct(input);
    }
}
