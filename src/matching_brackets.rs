use std::collections::HashMap;

const LEFT_BRACKETS: &[char] = &['[', '{', '('];
const RIGHT_BRACKETS: &[char] = &[']', '}', ')'];

pub fn brackets_are_balanced(string: &str) -> bool {
    let mut brackets_map: HashMap<char, u32> = HashMap::new();
    let mut bracket_order = 0;
    let mut brace_order = 0;
    let mut parenthese_order = 0;

    for ch in string.chars() {
        match ch {
            '[' => {
                bracket_order += 1;
                brackets_map.entry(ch).and_modify(|c| *c += 1).or_insert(1);
            }
            '{' => {
                brace_order += 1;
                brackets_map.entry(ch).and_modify(|c| *c += 1).or_insert(1);
            }
            '(' => {
                parenthese_order += 1;
                brackets_map.entry(ch).and_modify(|c| *c += 1).or_insert(1);
            }
            ']' => {
                if bracket_order < 0 {
                    return false;
                }
                bracket_order -= 1;
            }
            '}' => {
                if brace_order < 0 {
                    return false;
                }
                brace_order -= 1;
            }
            ')' => {
                if parenthese_order < 0 {
                    return false;
                }
                parenthese_order -= 1;
            }
            _ => {}
        }
    }

    for (index, left_bra) in LEFT_BRACKETS.iter().enumerate() {
        match brackets_map.get(left_bra) {
            Some(left_count) => match brackets_map.get(&RIGHT_BRACKETS[index]) {
                Some(right_count) => {
                    if left_count != right_count {
                        return false;
                    }
                }
                None => {
                    return false;
                }
            },
            None => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::matching_brackets::brackets_are_balanced;
    #[test]
    fn paired_square_brackets() {
        assert!(brackets_are_balanced("[]"));
    }
    #[test]
    fn empty_string() {
        assert!(brackets_are_balanced(""));
    }
    #[test]
    fn unpaired_brackets() {
        assert!(!brackets_are_balanced("[["));
    }
    #[test]
    fn wrong_ordered_brackets() {
        assert!(!brackets_are_balanced("}{"));
    }
    #[test]
    fn wrong_closing_bracket() {
        assert!(!brackets_are_balanced("{]"));
    }
    #[test]
    fn paired_with_whitespace() {
        assert!(brackets_are_balanced("{ }"));
    }
    #[test]
    fn partially_paired_brackets() {
        assert!(!brackets_are_balanced("{[])"));
    }
    #[test]
    fn simple_nested_brackets() {
        assert!(brackets_are_balanced("{[]}"));
    }
    #[test]
    fn several_paired_brackets() {
        assert!(brackets_are_balanced("{}[]"));
    }
    #[test]
    fn paired_and_nested_brackets() {
        assert!(brackets_are_balanced("([{}({}[])])"));
    }
    #[test]
    fn unopened_closing_brackets() {
        assert!(!brackets_are_balanced("{[)][]}"));
    }
    #[test]
    fn unpaired_and_nested_brackets() {
        assert!(!brackets_are_balanced("([{])"));
    }
    #[test]
    fn paired_and_wrong_nested_brackets() {
        assert!(!brackets_are_balanced("[({]})"));
    }
    #[test]
    fn paired_and_incomplete_brackets() {
        assert!(!brackets_are_balanced("{}["));
    }
    #[test]
    fn too_many_closing_brackets() {
        assert!(!brackets_are_balanced("[]]"));
    }
    #[test]
    fn early_incomplete_brackets() {
        assert!(!brackets_are_balanced(")()"));
    }
    #[test]
    fn early_mismatched_brackets() {
        assert!(!brackets_are_balanced("{)()"));
    }
    #[test]
    fn math_expression() {
        assert!(brackets_are_balanced("(((185 + 223.85) * 15) - 543)/2"));
    }
    #[test]
    fn complex_latex_expression() {
        let input = "\\left(\\begin{array}{cc} \\frac{1}{3} & x\\\\ \\mathrm{e}^{x} &... x^2 \
                 \\end{array}\\right)";
        assert!(brackets_are_balanced(input));
    }
}
