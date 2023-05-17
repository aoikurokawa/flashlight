pub type Value = i32;
pub type ForthResult = std::result::Result<(), Error>;

#[derive(Clone, Debug)]
enum Instruction {
    Add,
    Sub,
    Mul,
    Div,
    Dup,
    Swap,
    Drop,
    Over,
    Number(Value),
    Call(Value),
}

struct Definition {
    name: String,
    body: Vec<Instruction>,
}

pub struct Forth {
    dict: Vec<Definition>,
    stack: Vec<Value>,
}

impl Default for Forth {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    DivisionByZero,
    StackUnderflow,
    UnknownWord,
    InvalidWord,
}

fn parse_buildn(word: &str) -> Result<Instruction, Error> {
    match word {
        "+" => Ok(Instruction::Add),
        "-" => Ok(Instruction::Sub),
        "*" => Ok(Instruction::Mul),
        "/" => Ok(Instruction::Div),
        "DUP" => Ok(Instruction::Dup),
        "SWAP" => Ok(Instruction::Swap),
        "DROP" => Ok(Instruction::Drop),
        "OVER" => Ok(Instruction::Over),
        _ => {
            if let Ok(num) = Value::from_str_radix(word, 10) {
                Ok(Instruction::Number(num))
            } else {
                Err(Error::UnknownWord)
            }
        }
    }
}

impl Forth {
    fn parse_word<'a>(
        &mut self,
        word: &'a str,
        remaining_input: &mut impl Iterator<Item = &'a str>,
    ) -> ForthResult {
        if word == ":" {
            self.parse_definition(remaining_input)
        } else {
            let instr = self.parse_normal_word(word)?;
            self.eval_instruction(instr)
        }
    }

    fn parse_normal_word(&mut self, word: &str) -> Result<Instruction, Error> {
        if word == ":" || word == ";" {
            Err(Error::InvalidWord)
        } else {
            let canonical = word.to_ascii_uppercase();
            if let Some(call) = self.find_defn(&canonical) {
                Ok(call)
            } else {
                parse_buildn(&canonical)
            }
        }
    }

    fn parse_definition<'a>(&mut self, iter: &mut impl Iterator<Item = &'a str>) -> ForthResult {
        if let Some(new_word) = iter.next() {
            if Value::from_str_radix(new_word, 10).is_ok() {
                return Err(Error::InvalidWord);
            }

            let name = new_word.to_ascii_uppercase();
            let mut body = Vec::new();
            for word in iter {
                if word == ";" {
                    self.dict.push(Definition { name, body });
                    return Ok(());
                } else {
                    body.push(self.parse_normal_word(word)?)
                }
            }
        }

        Err(Error::InvalidWord)
    }

    fn eval_instruction(&mut self, instr: Instruction) -> ForthResult {
        match instr {
            Instruction::Add => self.arith(|a, b| Ok(a + b)),
            Instruction::Sub => self.arith(|a, b| Ok(a - b)),
            Instruction::Mul => self.arith(|a, b| Ok(a * b)),
            Instruction::Div => self.arith(|a, b| {
                if b == 0 {
                    Err(Error::DivisionByZero)
                } else {
                    Ok(a / b)
                }
            }),
            Instruction::Dup => self.dup(),
            Instruction::Swap => self.swap(),
            Instruction::Drop => self.drop(),
            Instruction::Over => self.over(),
            Instruction::Number(n) => {
                self.push(n);
                Ok(())
            }
            Instruction::Call(idx) => self.call(idx),
        }
    }

    fn push(&mut self, val: Value) {
        self.stack.push(val);
    }

    fn pop(&mut self) -> Result<Value, Error> {
        if let Some(v) = self.stack.pop() {
            Ok(v)
        } else {
            Err(Error::StackUnderflow)
        }
    }

    fn arith<F: FnOnce(Value, Value) -> Result<Value, Error>>(&mut self, op: F) -> ForthResult {
        let rhs = self.pop()?;
        let lhs = self.pop()?;
        self.push(op(lhs, rhs)?);
        Ok(())
    }

    fn dup(&mut self) -> ForthResult {
        let v = self.pop()?;
        self.push(v);
        self.push(v);
        Ok(())
    }

    fn swap(&mut self) -> ForthResult {
        let top = self.pop()?;
        let bottom = self.pop()?;
        self.push(top);
        self.push(bottom);
        Ok(())
    }

    fn drop(&mut self) -> ForthResult {
        self.pop()?;
        Ok(())
    }

    fn over(&mut self) -> ForthResult {
        let top = self.pop()?;
        let bottom = self.pop()?;
        self.push(bottom);
        self.push(top);
        self.push(bottom);
        Ok(())
    }

    fn call(&mut self, idx: Value) -> ForthResult {
        let idx = idx.try_into().unwrap();
        if self.dict.len() <= idx {
            Err(Error::UnknownWord)
        } else {
            let def = self.dict[idx].body.clone();
            for instr in def {
                self.eval_instruction(instr)?;
            }
            Ok(())
        }
    }

    fn find_defn(&self, word: &str) -> Option<Instruction> {
        for (idx, defn) in self.dict.iter().enumerate().rev() {
            if defn.name == word {
                return Some(Instruction::Call(idx.try_into().unwrap()));
            }
        }
        None
    }

    pub fn new() -> Forth {
        Self {
            dict: Vec::new(),
            stack: Vec::new(),
        }
    }

    pub fn stack(&self) -> &[Value] {
        &self.stack
    }

    pub fn eval(&mut self, input: &str) -> ForthResult {
        let mut iter = input.split_ascii_whitespace();
        while let Some(word) = iter.next() {
            self.parse_word(word, &mut iter)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::forth::{Error, Forth, Value};

    #[test]
    fn no_input_no_stack() {
        assert_eq!(Vec::<Value>::new(), Forth::new().stack());
    }

    #[test]
    fn numbers_just_get_pushed_onto_the_stack() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 3 4 5").is_ok());

        assert_eq!(vec![1, 2, 3, 4, 5], f.stack());
    }

    #[test]
    fn can_add_two_numbers() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 +").is_ok());

        assert_eq!(vec![3], f.stack());
    }

    #[test]
    #[ignore]

    fn addition_error() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::StackUnderflow), f.eval("1 +"));

        assert_eq!(Err(Error::StackUnderflow), f.eval("+"));
    }

    #[test]
    fn can_subtract_two_numbers() {
        let mut f = Forth::new();

        assert!(f.eval("3 4 -").is_ok());

        assert_eq!(vec![-1], f.stack());
    }

    #[test]
    fn subtraction_error() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::StackUnderflow), f.eval("1 -"));

        assert_eq!(Err(Error::StackUnderflow), f.eval("-"));
    }

    #[test]
    fn can_multiply_two_numbers() {
        let mut f = Forth::new();

        assert!(f.eval("2 4 *").is_ok());

        assert_eq!(vec![8], f.stack());
    }

    #[test]
    fn multiplication_error() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::StackUnderflow), f.eval("1 *"));

        assert_eq!(Err(Error::StackUnderflow), f.eval("*"));
    }

    #[test]
    fn can_divide_two_numbers() {
        let mut f = Forth::new();

        assert!(f.eval("12 3 /").is_ok());

        assert_eq!(vec![4], f.stack());
    }

    #[test]
    fn performs_integer_division() {
        let mut f = Forth::new();

        assert!(f.eval("8 3 /").is_ok());

        assert_eq!(vec![2], f.stack());
    }

    #[test]
    fn division_error() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::StackUnderflow), f.eval("1 /"));

        assert_eq!(Err(Error::StackUnderflow), f.eval("/"));
    }

    #[test]
    fn errors_if_dividing_by_zero() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::DivisionByZero), f.eval("4 0 /"));
    }

    #[test]
    fn addition_and_subtraction() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 + 4 -").is_ok());

        assert_eq!(vec![-1], f.stack());
    }

    #[test]
    fn multiplication_and_division() {
        let mut f = Forth::new();

        assert!(f.eval("2 4 * 3 /").is_ok());

        assert_eq!(vec![2], f.stack());
    }

    #[test]
    fn dup() {
        let mut f = Forth::new();

        assert!(f.eval("1 dup").is_ok());

        assert_eq!(vec![1, 1], f.stack());
    }

    #[test]
    fn dup_top_value_only() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 dup").is_ok());

        assert_eq!(vec![1, 2, 2], f.stack());
    }

    #[test]
    fn dup_case_insensitive() {
        let mut f = Forth::new();

        assert!(f.eval("1 DUP Dup dup").is_ok());

        assert_eq!(vec![1, 1, 1, 1], f.stack());
    }

    #[test]
    fn dup_error() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::StackUnderflow), f.eval("dup"));
    }

    #[test]
    fn drop() {
        let mut f = Forth::new();

        assert!(f.eval("1 drop").is_ok());

        assert_eq!(Vec::<Value>::new(), f.stack());
    }

    #[test]
    fn drop_with_two() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 drop").is_ok());

        assert_eq!(vec![1], f.stack());
    }

    #[test]
    fn drop_case_insensitive() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 3 4 DROP Drop drop").is_ok());

        assert_eq!(vec![1], f.stack());
    }

    #[test]
    fn drop_error() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::StackUnderflow), f.eval("drop"));
    }

    #[test]
    fn swap() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 swap").is_ok());

        assert_eq!(vec![2, 1], f.stack());
    }

    #[test]
    fn swap_with_three() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 3 swap").is_ok());

        assert_eq!(vec![1, 3, 2], f.stack());
    }

    #[test]
    fn swap_case_insensitive() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 SWAP 3 Swap 4 swap").is_ok());

        assert_eq!(vec![2, 3, 4, 1], f.stack());
    }

    #[test]
    fn swap_error() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::StackUnderflow), f.eval("1 swap"));

        assert_eq!(Err(Error::StackUnderflow), f.eval("swap"));
    }

    #[test]
    fn over() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 over").is_ok());

        assert_eq!(vec![1, 2, 1], f.stack());
    }

    #[test]
    fn over_with_three() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 3 over").is_ok());

        assert_eq!(vec![1, 2, 3, 2], f.stack());
    }

    #[test]
    fn over_case_insensitive() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 OVER Over over").is_ok());

        assert_eq!(vec![1, 2, 1, 2, 1], f.stack());
    }

    #[test]
    fn over_error() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::StackUnderflow), f.eval("1 over"));

        assert_eq!(Err(Error::StackUnderflow), f.eval("over"));
    }

    // User-defined words

    #[test]
    fn can_consist_of_built_in_words() {
        let mut f = Forth::new();

        assert!(f.eval(": dup-twice dup dup ;").is_ok());

        assert!(f.eval("1 dup-twice").is_ok());

        assert_eq!(vec![1, 1, 1], f.stack());
    }

    #[test]
    fn execute_in_the_right_order() {
        let mut f = Forth::new();

        assert!(f.eval(": countup 1 2 3 ;").is_ok());

        assert!(f.eval("countup").is_ok());

        assert_eq!(vec![1, 2, 3], f.stack());
    }

    #[test]
    fn redefining_an_existing_word() {
        let mut f = Forth::new();

        assert!(f.eval(": foo dup ;").is_ok());

        assert!(f.eval(": foo dup dup ;").is_ok());

        assert!(f.eval("1 foo").is_ok());

        assert_eq!(vec![1, 1, 1], f.stack());
    }

    #[test]
    fn redefining_an_existing_built_in_word() {
        let mut f = Forth::new();

        assert!(f.eval(": swap dup ;").is_ok());

        assert!(f.eval("1 swap").is_ok());

        assert_eq!(vec![1, 1], f.stack());
    }

    #[test]
    fn user_defined_words_are_case_insensitive() {
        let mut f = Forth::new();

        assert!(f.eval(": foo dup ;").is_ok());

        assert!(f.eval("1 FOO Foo foo").is_ok());

        assert_eq!(vec![1, 1, 1, 1], f.stack());
    }

    #[test]
    fn definitions_are_case_insensitive() {
        let mut f = Forth::new();

        assert!(f.eval(": SWAP DUP Dup dup ;").is_ok());

        assert!(f.eval("1 swap").is_ok());

        assert_eq!(vec![1, 1, 1, 1], f.stack());
    }

    #[test]
    fn redefining_a_built_in_operator() {
        let mut f = Forth::new();

        assert!(f.eval(": + * ;").is_ok());

        assert!(f.eval("3 4 +").is_ok());

        assert_eq!(vec![12], f.stack());
    }

    #[test]
    fn can_use_different_words_with_the_same_name() {
        let mut f = Forth::new();

        assert!(f.eval(": foo 5 ;").is_ok());

        assert!(f.eval(": bar foo ;").is_ok());

        assert!(f.eval(": foo 6 ;").is_ok());

        assert!(f.eval("bar foo").is_ok());

        assert_eq!(vec![5, 6], f.stack());
    }

    #[test]
    fn can_define_word_that_uses_word_with_the_same_name() {
        let mut f = Forth::new();

        assert!(f.eval(": foo 10 ;").is_ok());

        assert!(f.eval(": foo foo 1 + ;").is_ok());

        assert!(f.eval("foo").is_ok());

        assert_eq!(vec![11], f.stack());
    }

    #[test]
    fn defining_a_number() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::InvalidWord), f.eval(": 1 2 ;"));
    }

    #[test]
    fn malformed_word_definition() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::InvalidWord), f.eval(":"));

        assert_eq!(Err(Error::InvalidWord), f.eval(": foo"));

        assert_eq!(Err(Error::InvalidWord), f.eval(": foo 1"));
    }

    #[test]
    fn calling_non_existing_word() {
        let mut f = Forth::new();

        assert_eq!(Err(Error::UnknownWord), f.eval("1 foo"));
    }

    #[test]
    fn multiple_definitions() {
        let mut f = Forth::new();

        assert!(f.eval(": one 1 ; : two 2 ; one two +").is_ok());

        assert_eq!(vec![3], f.stack());
    }

    #[test]
    fn definitions_after_ops() {
        let mut f = Forth::new();

        assert!(f.eval("1 2 + : addone 1 + ; addone").is_ok());

        assert_eq!(vec![4], f.stack());
    }

    #[test]
    fn redefine_an_existing_word_with_another_existing_word() {
        let mut f = Forth::new();

        assert!(f.eval(": foo 5 ;").is_ok());

        assert!(f.eval(": bar foo ;").is_ok());

        assert!(f.eval(": foo 6 ;").is_ok());

        assert!(f.eval(": bar foo ;").is_ok());

        assert!(f.eval("bar foo").is_ok());

        assert_eq!(vec![6, 6], f.stack());
    }
}
