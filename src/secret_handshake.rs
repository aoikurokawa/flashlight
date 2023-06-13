pub fn actions(n: u8) -> Vec<&'static str> {
    let mut secrets = Vec::new();

    let binary = u8_to_binary_slice(n);
    println!("{:?}", binary);
    for (idx, bit) in binary.iter().enumerate() {
        match idx {
            0 => {
                if *bit == 1 {
                    secrets.push("wink");
                }
            }
            1 => {
                if *bit == 1 {
                    secrets.push("double blink");
                }
            }
            2 => {
                if *bit == 1 {
                    secrets.push("close your eyes");
                }
            }
            3 => {
                if *bit == 1 {
                    secrets.push("jump");
                }
            }
            4 => {
                if *bit == 1 {
                    secrets.reverse();
                }
            }
            _ => {}
        }
    }

    secrets
}

fn u8_to_binary_slice(value: u8) -> [u8; 8] {
    let mut result = [0u8; 8];
    for i in 0..8 {
        result[7 - i] = (value >> i) & 1;
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use crate::secret_handshake::*;

    #[test]
    fn wink_for_1() {
        assert_eq!(actions(1), vec!["wink"])
    }

    #[test]
    fn double_blink_for_10() {
        assert_eq!(actions(2), vec!["double blink"])
    }

    #[test]
    fn close_your_eyes_for_100() {
        assert_eq!(actions(4), vec!["close your eyes"])
    }

    #[test]
    fn jump_for_1000() {
        assert_eq!(actions(8), vec!["jump"])
    }

    #[test]
    fn combine_two_actions() {
        assert_eq!(actions(3), vec!["wink", "double blink"])
    }

    #[test]
    fn reverse_two_actions() {
        assert_eq!(actions(19), vec!["double blink", "wink"])
    }

    #[test]
    fn reversing_one_action_gives_the_same_action() {
        assert_eq!(actions(24), vec!["jump"])
    }

    #[test]
    fn reversing_no_actions_still_gives_no_actions() {
        assert_eq!(actions(16), Vec::<&'static str>::new())
    }

    #[test]
    fn all_possible_actions() {
        assert_eq!(
            actions(15),
            vec!["wink", "double blink", "close your eyes", "jump"]
        )
    }

    #[test]
    fn reverse_all_possible_actions() {
        assert_eq!(
            actions(31),
            vec!["jump", "close your eyes", "double blink", "wink"]
        )
    }

    #[test]
    fn do_nothing_for_zero() {
        assert_eq!(actions(0), Vec::<&'static str>::new())
    }
}
