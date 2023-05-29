use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[macro_export]
macro_rules! debug_panic {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        panic!($($arg)*);

        #[cfg(not(debug_assertions))]
        log::error!($($arg)*);
    };
}

pub trait HashU64 {
    fn hash_u64(&self) -> u64;
}

impl<T: Hash> HashU64 for T {
    fn hash_u64(&self) -> u64 {
        let mut hasher = DefaultHasher::default();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

pub trait DebugAssert {
    type R;
    fn debug_assert(self) -> Self::R;
}

impl<T> DebugAssert for Option<T> {
    type R = Self;

    fn debug_assert(self) -> Self::R {
        if self.is_none() {
            debug_panic!("Option should not be empty");
        }

        self
    }
}
