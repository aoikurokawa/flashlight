const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz";

enum Alphabet {
    A(isize),
    B(isize),
    C(isize),
    D(isize),
    E(isize),
    F(isize),
    G(isize),
    H(isize),
    I(isize),
    J(isize),
    K(isize),
    L(isize),
    M(isize),
    N(isize),
    O(isize),
    P(isize),
    Q(isize),
    R(isize),
    S(isize),
    T(isize),
    U(isize),
    V(isize),
    W(isize),
    X(isize),
    Y(isize),
    Z(isize),
    Other,
}

impl From<char> for Alphabet {
    fn from(val: char) -> Self {
        match val {
            'a' => Alphabet::A(0),
            'b' => Alphabet::B(1),
            'c' => Alphabet::C(2),
            'd' => Alphabet::D(3),
            'e' => Alphabet::E(4),
            'f' => Alphabet::F(5),
            'g' => Alphabet::G(6),
            'h' => Alphabet::H(7),
            'i' => Alphabet::I(8),
            'j' => Alphabet::J(9),
            'k' => Alphabet::K(10),
            'l' => Alphabet::L(11),
            'm' => Alphabet::M(12),
            'n' => Alphabet::N(13),
            'o' => Alphabet::O(14),
            'p' => Alphabet::P(15),
            'q' => Alphabet::Q(16),
            'r' => Alphabet::R(17),
            's' => Alphabet::S(18),
            't' => Alphabet::T(19),
            'u' => Alphabet::U(20),
            'v' => Alphabet::V(21),
            'w' => Alphabet::W(22),
            'x' => Alphabet::X(23),
            'y' => Alphabet::Y(24),
            'z' => Alphabet::Z(25),
            _ => Alphabet::Other,
        }
    }
}

pub fn rotate(input: &str, key: i8) -> String {
    let mut cipher = String::new();

    for input_ch in input.chars() {
        match input_ch.to_ascii_lowercase().into() {
            Alphabet::A(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::B(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::C(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::D(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::E(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::F(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::G(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::H(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::I(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::J(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::K(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::L(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::M(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::N(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::O(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::P(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::Q(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::R(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::S(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::T(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::U(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::V(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::W(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::X(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::Y(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::Z(alphabet_idx) => {
                find_char(key, alphabet_idx, input_ch, &mut cipher);
            }
            Alphabet::Other => cipher.push(input_ch),
        }
    }

    cipher
}

fn find_char(key: i8, alphabet_idx: isize, input_ch: char, cipher: &mut String) {
    let remainder = if key < 0 {
        let div = key % 26;
        (div as isize + alphabet_idx) % 26
    } else {
        (key as isize + alphabet_idx) % 26
    };
    for (index, ch) in ALPHABET.char_indices() {
        if index as isize == remainder {
            if input_ch.is_ascii_uppercase() {
                cipher.push(ch.to_ascii_uppercase());
            } else {
                cipher.push(ch);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rotational_cipher as cipher;

    #[test]
    fn rotate_a_1() {
        assert_eq!("b", cipher::rotate("a", 1));
    }

    #[test]
    fn rotate_a_26() {
        assert_eq!("a", cipher::rotate("a", 26));
    }

    #[test]
    fn rotate_a_0() {
        assert_eq!("a", cipher::rotate("a", 0));
    }

    #[test]
    fn rotate_m_13() {
        assert_eq!("z", cipher::rotate("m", 13));
    }

    #[test]
    fn rotate_n_13_with_wrap() {
        assert_eq!("a", cipher::rotate("n", 13));
    }

    #[test]
    fn rotate_caps() {
        assert_eq!("TRL", cipher::rotate("OMG", 5));
    }

    #[test]
    fn rotate_spaces() {
        assert_eq!("T R L", cipher::rotate("O M G", 5));
    }

    #[test]
    fn rotate_numbers() {
        assert_eq!(
            "Xiwxmrk 1 2 3 xiwxmrk",
            cipher::rotate("Testing 1 2 3 testing", 4)
        );
    }

    #[test]
    fn rotate_punctuation() {
        assert_eq!(
            "Gzo\'n zvo, Bmviyhv!",
            cipher::rotate("Let\'s eat, Grandma!", 21)
        );
    }

    #[test]
    fn rotate_all_the_letters() {
        assert_eq!(
            "Gur dhvpx oebja sbk whzcf bire gur ynml qbt.",
            cipher::rotate("The quick brown fox jumps over the lazy dog.", 13)
        );
    }

    #[test]
    fn rotate_m_negative_1() {
        assert_eq!("l", cipher::rotate("m", -1));
    }

    #[test]
    fn rotate_letters_negative_26() {
        assert_eq!("omg", cipher::rotate("omg", -26));
    }

    #[test]
    fn rotate_letters_negative_27() {
        assert_eq!("nlf", cipher::rotate("omg", -27));
    }
}
