use std::{collections::HashMap, thread};

pub fn frequency(input: &[&str], worker_count: usize) -> HashMap<char, usize> {
    let mut counters = HashMap::new();
    if input.is_empty() {
        return counters;
    }

    let mut handlers = vec![];
    let chunk_size = input.len() / worker_count + 1;

    for chunk in input.chunks(chunk_size) {
        let temp_data = chunk
            .iter()
            .map(|s| (*s).to_string())
            .collect::<Vec<String>>();
        let handler = thread::spawn(move || do_count(&temp_data));
        handlers.push(handler);
    }

    for handler in handlers {
        for (c, count) in handler.join().unwrap() {
            let c_count = counters.entry(c).or_insert(0);
            *c_count += count;
        }
    }

    counters
}

fn do_count(input: &[String]) -> HashMap<char, usize> {
    let mut counter = HashMap::new();

    for phrase in input.iter() {
        for c in phrase.chars().filter(|l| l.is_alphabetic()) {
            let c_count = counter.entry(c.to_ascii_lowercase()).or_insert(0);
            *c_count += 1;
        }
    }

    counter
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::parallel_letter_frequency as frequency;

    // Poem by Friedrich Schiller. The corresponding music is the European Anthem.

    const ODE_AN_DIE_FREUDE: [&str; 8] = [
        "Freude schöner Götterfunken",
        "Tochter aus Elysium,",
        "Wir betreten feuertrunken,",
        "Himmlische, dein Heiligtum!",
        "Deine Zauber binden wieder",
        "Was die Mode streng geteilt;",
        "Alle Menschen werden Brüder,",
        "Wo dein sanfter Flügel weilt.",
    ];

    // Dutch national anthem

    const WILHELMUS: [&str; 8] = [
        "Wilhelmus van Nassouwe",
        "ben ik, van Duitsen bloed,",
        "den vaderland getrouwe",
        "blijf ik tot in den dood.",
        "Een Prinse van Oranje",
        "ben ik, vrij, onverveerd,",
        "den Koning van Hispanje",
        "heb ik altijd geëerd.",
    ];

    // American national anthem

    const STAR_SPANGLED_BANNER: [&str; 8] = [
        "O say can you see by the dawn's early light,",
        "What so proudly we hailed at the twilight's last gleaming,",
        "Whose broad stripes and bright stars through the perilous fight,",
        "O'er the ramparts we watched, were so gallantly streaming?",
        "And the rockets' red glare, the bombs bursting in air,",
        "Gave proof through the night that our flag was still there;",
        "O say does that star-spangled banner yet wave,",
        "O'er the land of the free and the home of the brave?",
    ];

    #[test]
    fn test_no_texts() {
        assert_eq!(frequency::frequency(&[], 4), HashMap::new());
    }

    #[test]
    fn test_one_letter() {
        let mut hm = HashMap::new();

        hm.insert('a', 1);

        assert_eq!(frequency::frequency(&["a"], 4), hm);
    }

    #[test]
    fn test_case_insensitivity() {
        let mut hm = HashMap::new();

        hm.insert('a', 2);

        assert_eq!(frequency::frequency(&["aA"], 4), hm);
    }

    #[test]
    fn test_many_empty_lines() {
        let v = vec![""; 1000];

        assert_eq!(frequency::frequency(&v[..], 4), HashMap::new());
    }

    #[test]
    fn test_many_times_same_text() {
        let v = vec!["abc"; 1000];

        let mut hm = HashMap::new();

        hm.insert('a', 1000);

        hm.insert('b', 1000);

        hm.insert('c', 1000);

        assert_eq!(frequency::frequency(&v[..], 4), hm);
    }

    #[test]
    fn test_punctuation_doesnt_count() {
        assert!(!frequency::frequency(&WILHELMUS, 4).contains_key(&','));
    }

    #[test]
    fn test_numbers_dont_count() {
        assert!(!frequency::frequency(&["Testing, 1, 2, 3"], 4).contains_key(&'1'));
    }

    #[test]
    fn test_all_three_anthems_1_worker() {
        let mut v = Vec::new();

        for anthem in [ODE_AN_DIE_FREUDE, WILHELMUS, STAR_SPANGLED_BANNER].iter() {
            for line in anthem.iter() {
                v.push(*line);
            }
        }

        let freqs = frequency::frequency(&v[..], 1);

        assert_eq!(freqs.get(&'a'), Some(&49));

        assert_eq!(freqs.get(&'t'), Some(&56));

        assert_eq!(freqs.get(&'ü'), Some(&2));
    }

    #[test]
    fn test_all_three_anthems_3_workers() {
        let mut v = Vec::new();

        for anthem in [ODE_AN_DIE_FREUDE, WILHELMUS, STAR_SPANGLED_BANNER].iter() {
            for line in anthem.iter() {
                v.push(*line);
            }
        }

        let freqs = frequency::frequency(&v[..], 3);

        assert_eq!(freqs.get(&'a'), Some(&49));

        assert_eq!(freqs.get(&'t'), Some(&56));

        assert_eq!(freqs.get(&'ü'), Some(&2));
    }

    #[test]
    fn test_non_integer_multiple_of_threads() {
        let v = vec!["abc"; 999];

        let mut hm = HashMap::new();

        hm.insert('a', 999);

        hm.insert('b', 999);

        hm.insert('c', 999);

        assert_eq!(frequency::frequency(&v[..], 4), hm);
    }
}
