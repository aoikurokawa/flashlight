macro_rules! echo_num {
    ($num:expr) => {
       println!("{}", $num);
    };
}

fn main() {
    echo_num!(10);
    echo_num![10];
    echo_num!{10};
}
