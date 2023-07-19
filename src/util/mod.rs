mod markers;
pub use markers::*;

mod traits;
pub use traits::*;

pub mod layout;
pub mod math;
pub mod svg;
pub mod taffy;
pub mod text;

pub fn min<T: PartialOrd>(x: T, y: T) -> T {
    if x <= y {
        x
    } else {
        y
    }
}

pub fn max<T: PartialOrd>(x: T, y: T) -> T {
    if x >= y {
        x
    } else {
        y
    }
}
