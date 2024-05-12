pub fn sig_num(x: i128) -> i128 {
    if x.is_negative() {
        -1
    } else {
        1
    }
}
