use std::sync::{Mutex, MutexGuard};

pub(crate) trait ForceLock<T> {
    fn force_lock(&self) -> MutexGuard<'_, T>;
}

impl<T> ForceLock<T> for Mutex<T> {
    fn force_lock(&self) -> MutexGuard<'_, T> {
        match self.lock() {
            Ok(i) => i,
            Err(e) => e.into_inner(),
        }
    }
}
