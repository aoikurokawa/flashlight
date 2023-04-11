macro_rules! echo_num {
    ($($num:expr), *) => {
        $(
       print!("{}, ", $num);
) *
        println!("");
    }
}

fn main() {
    echo_num![10, 20, 30];
}
