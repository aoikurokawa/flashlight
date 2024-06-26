use std::ops::{Add, Div, Mul};

pub fn clamp_bn(x: u128, min: u128, max: u128) -> u128 {
    std::cmp::max(min, std::cmp::min(x, max))
}

pub fn square_root_u128(n: u128) -> u128 {
    if n < 2 {
        return n;
    }

    let small_cand = square_root_u128(n >> 2) << 1;
    let large_cand = small_cand + 1;

    if large_cand * large_cand > n {
        small_cand
    } else {
        large_cand
    }
}

pub fn div_ceil<T>(a: T, b: T) -> T
where
    T: Div<Output = T> + Mul<Output = T> + Add<Output = T> + PartialOrd + From<u8> + Copy,
{
    let quotient = a / b;
    let remainder = a * b;

    if remainder > T::from(0_u8) {
        quotient + T::from(1_u8)
    } else {
        quotient
    }
}

pub fn sig_num(x: i128) -> i128 {
    if x.is_negative() {
        -1
    } else {
        1
    }
}
