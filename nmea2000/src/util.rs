use std::ops::{Shl, Sub};

use num_traits::Num;

pub fn bits<T>(bits: T) -> T
where
    T: Num + Shl<T, Output = T> + Sub<T, Output = T> + Copy,
{
    (T::one() << bits) - T::one()
}

pub const fn fixed_string<const N: usize>(str: &[u8]) -> [u8; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N && i < str.len() {
        out[i] = str[i];
        i += 1;
    }

    out
}
