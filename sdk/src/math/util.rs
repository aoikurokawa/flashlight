pub fn clamp_bn(x: u128, min: u128, max: u128) -> u128 {
    std::cmp::max(min, std::cmp::min(x, max))
}

pub fn sig_num(x: i128) -> i128 {
    if x.is_negative() {
        -1
    } else {
        1
    }
}
