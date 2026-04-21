use std::ops::{Shl, Sub};

use num_traits::Num;

pub fn bits<T>(bits: T) -> T
where
    T: Num + Shl<T, Output = T> + Sub<T, Output = T> + Copy,
{
    (T::one() << bits) - T::one()
}
