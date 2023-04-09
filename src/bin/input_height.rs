fn main() {
    let mut height;

    loop {
        println!("Enter height: ");
        height = input_f(0.0);
        if height > 0.0 {
            break;
        }
        println!("Please enter correct value");
    }

    let weight = 22.0 * (height / 100.0).powf(2.0);
    println!("normal weight is {:.1} kg", weight);
}

fn input_str() -> String {
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).expect("error");
    s.trim_end().to_string()
}

fn input_f(def: f64) -> f64 {
    let s = input_str();
    match s.trim().parse() {
        Ok(v) => v,
        Err(_) => def,
    }
}
