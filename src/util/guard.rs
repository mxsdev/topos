use std::{
    borrow::Borrow,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub trait ReadLockable<T> {
    fn read_lock(&self) -> ReadableLock<'_, T>;
}

impl<'a, T> ReadLockable<T> for T {
    fn read_lock(&self) -> ReadableLock<'_, T> {
        ReadableLock::Ref(self)
    }
}

impl<'a, T> ReadLockable<T> for &T {
    fn read_lock(&self) -> ReadableLock<'_, T> {
        ReadableLock::Ref(self)
    }
}

impl<'a, T> ReadLockable<T> for &mut T {
    fn read_lock(&self) -> ReadableLock<'_, T> {
        ReadableLock::Ref(self)
    }
}

impl<'a, T> ReadLockable<T> for RwLock<T> {
    fn read_lock(&self) -> ReadableLock<'_, T> {
        ReadableLock::Rw(self.read().unwrap())
    }
}

impl<'a, T> ReadLockable<T> for &RwLock<T> {
    fn read_lock(&self) -> ReadableLock<'_, T> {
        ReadableLock::Rw(self.read().unwrap())
    }
}

pub enum ReadableLock<'a, T> {
    Rw(RwLockReadGuard<'a, T>),
    Ref(&'a T),
}

impl<'a, T> Deref for ReadableLock<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            Self::Rw(value) => value.deref(),
            Self::Ref(value) => value,
        }
    }
}

pub enum WritableLock<'a, T> {
    Rw(RwLockWriteGuard<'a, T>),
    Ref(&'a mut T),
}

pub trait WriteLockable<T>: ReadLockable<T> {
    fn write_lock(&mut self) -> WritableLock<'_, T>;
}

impl<T> WriteLockable<T> for T {
    fn write_lock(&mut self) -> WritableLock<'_, T> {
        WritableLock::Ref(self)
    }
}

impl<T> WriteLockable<T> for &mut T {
    fn write_lock(&mut self) -> WritableLock<'_, T> {
        WritableLock::Ref(self)
    }
}

impl<T> WriteLockable<T> for &RwLock<T> {
    fn write_lock(&mut self) -> WritableLock<'_, T> {
        WritableLock::Rw(self.write().unwrap())
    }
}

impl<T> WriteLockable<T> for RwLock<T> {
    fn write_lock(&mut self) -> WritableLock<'_, T> {
        WritableLock::Rw(self.write().unwrap())
    }
}

// impl<T, X: WriteLockable<T>> WriteLockable<T> for &mut X {
//     fn write_lock(&mut self) -> WritableLock<'_, T> {
//         WritableLock::Ref(self)
//     }
// }

impl<T> Deref for WritableLock<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Rw(value) => value.deref(),
            Self::Ref(value) => value,
        }
    }
}

impl<T> DerefMut for WritableLock<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Rw(value) => value.deref_mut(),
            Self::Ref(value) => value,
        }
    }
}
