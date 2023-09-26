use std::marker::PhantomData;

pub trait CacheComparator<C> {
    fn compare(a: &C, b: &C) -> bool;
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DefaultCacheComparator;

impl<C: PartialEq> CacheComparator<C> for DefaultCacheComparator {
    fn compare(a: &C, b: &C) -> bool {
        a == b
    }
}

#[derive(Default)]
pub struct CachedValue<T, C, Comparator: CacheComparator<C> = DefaultCacheComparator> {
    value: Option<T>,
    cache_key: Option<C>,
    comparator: PhantomData<Comparator>,
}

impl<T, C, Comparator: CacheComparator<C>> CachedValue<T, C, Comparator> {
    pub fn new(value: T, cache_key: C) -> Self {
        Self {
            value: value.into(),
            cache_key: cache_key.into(),
            comparator: PhantomData,
        }
    }

    pub fn get_or_insert_with(&mut self, cache_key: C, f: impl FnOnce() -> T) -> &T {
        if self.value.is_none()
            || self
                .cache_key
                .as_ref()
                .map(|c| !Comparator::compare(c, &cache_key))
                .unwrap_or(true)
        {
            self.cache_key = Some(cache_key);
            return self.value.insert(f());
        }

        return self.value.as_ref().unwrap();
    }
}
