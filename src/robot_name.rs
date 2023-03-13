use std::collections::HashSet;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use rand::Rng;

const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

static SET: Lazy<Mutex<HashSet<Robot>>> = Lazy::new(|| Mutex::new(HashSet::new()));

#[derive(Eq, PartialEq, Default, Hash)]
pub struct Robot {
    name: String,
}

impl Robot {
    pub fn new() -> Self {
        let mut robot = Robot::default();
        robot.reset_name();
        robot
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn reset_name(&mut self) {
        loop {
            let mut rng = rand::thread_rng();
            let letter: String = (0..2)
                .map(|_| {
                    let idx = rng.gen_range(0..CHARSET.len());
                    CHARSET[idx] as char
                })
                .collect();
            let num1 = rng.gen_range(0..10);
            let num2 = rng.gen_range(0..10);
            let num3 = rng.gen_range(0..10);

            let temp_name = format!("{}{}{}{}", letter, num1, num2, num3);

            if SET.lock().unwrap().insert(Robot {
                name: temp_name.clone(),
            }) {
                self.name = temp_name;
                break;
            }
        }
    }
}
