pub fn abbreviate1(phrase: &str) -> String {
    let mut acronym = String::new();

    for (_index, word) in phrase.split_whitespace().enumerate() {
        let mut index_uppercase: i8 = -1;
        let mut after_punctuation = false;

        for (index, ch) in word.chars().enumerate() {
            if index == 0 && ch.is_alphabetic() {
                acronym.push(ch);
                index_uppercase = index as i8;
                continue;
            }

            if ch.is_uppercase() && index_uppercase != index as i8 - 1 {
                acronym.push(ch);
            }

            if ch.is_uppercase() {
                index_uppercase = index as i8;
            }

            if after_punctuation {
                acronym.push(ch);
                after_punctuation = false;
            }

            if ch == '-' {
                after_punctuation = true;
            }
        }
    }

    acronym.to_uppercase()
}

pub fn abbreviate(phrase: &str) -> String {
    phrase
        .split(|c: char| c.is_ascii_whitespace() || c == '_' || c == '-')
        .flat_map(|word| {
            word.chars().take(1).chain(
                word.chars()
                    .skip_while(|c| c.is_ascii_uppercase())
                    .filter(|c| c.is_ascii_uppercase()),
            )
        })
        .collect::<String>()
        .to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use crate::acronym;

    #[test]
    fn empty() {
        assert_eq!(acronym::abbreviate(""), "");
    }
    #[test]
    fn basic() {
        assert_eq!(acronym::abbreviate("Portable Network Graphics"), "PNG");
    }
    #[test]
    fn lowercase_words() {
        assert_eq!(acronym::abbreviate("Ruby on Rails"), "ROR");
    }
    #[test]
    fn camelcase() {
        assert_eq!(acronym::abbreviate("HyperText Markup Language"), "HTML");
    }
    #[test]
    fn punctuation() {
        assert_eq!(acronym::abbreviate("First In, First Out"), "FIFO");
    }
    #[test]
    fn all_caps_word() {
        assert_eq!(
            acronym::abbreviate("GNU Image Manipulation Program"),
            "GIMP"
        );
    }
    #[test]
    fn all_caps_word_with_punctuation() {
        assert_eq!(acronym::abbreviate("PHP: Hypertext Preprocessor"), "PHP");
    }
    #[test]
    fn punctuation_without_whitespace() {
        assert_eq!(
            acronym::abbreviate("Complementary metal-oxide semiconductor"),
            "CMOS"
        );
    }
    #[test]
    fn very_long_abbreviation() {
        assert_eq!(
            acronym::abbreviate(
                "Rolling On The Floor Laughing So Hard That My Dogs Came Over And Licked Me"
            ),
            "ROTFLSHTMDCOALM"
        );
    }
    #[test]
    fn consecutive_delimiters() {
        assert_eq!(
            acronym::abbreviate("Something - I made up from thin air"),
            "SIMUFTA"
        );
    }
    #[test]
    fn apostrophes() {
        assert_eq!(acronym::abbreviate("Halley's Comet"), "HC");
    }
    #[test]
    fn underscore_emphasis() {
        assert_eq!(acronym::abbreviate("The Road _Not_ Taken"), "TRNT");
    }
}
