/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! A blocking exclusive lock.
//!
//! Only one thread can hold the lock at a time. Any thread that attempts
//! to acquire a held lock will block until the holder releases it, then
//! receive `None` (indicating the work was already done). Callers are
//! expected to re-check whether the work still needs doing after `None`.

use std::sync::Arc;
use std::sync::Once;

use crate::lock::Mutex;

/// A blocking exclusive lock. If the lock is unheld, `lock()` acquires it
/// and returns `Some(guard)`. If held, `lock()` blocks until release,
/// then returns `None`.
#[derive(Debug)]
pub struct ExclusiveLock {
    exclusive: Mutex<Option<Arc<Once>>>,
}

impl Default for ExclusiveLock {
    fn default() -> Self {
        Self {
            exclusive: Mutex::new(None),
        }
    }
}

pub struct ExclusiveLockGuard<'a> {
    inner: Option<&'a ExclusiveLock>,
}

impl Drop for ExclusiveLockGuard<'_> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            let mut lock = inner.exclusive.lock();
            if let Some(once) = &*lock {
                once.call_once(|| ());
                *lock = None;
            }
        }
    }
}

impl ExclusiveLock {
    /// Attempt to acquire the lock.
    ///
    /// - If unheld: acquires and returns `Some(guard)`.
    /// - If held: blocks until released, then returns `None`.
    pub fn lock(&self) -> Option<ExclusiveLockGuard<'_>> {
        let mut exclusive = self.exclusive.lock();
        match &*exclusive {
            None => {
                *exclusive = Some(Arc::new(Once::new()));
                Some(ExclusiveLockGuard { inner: Some(self) })
            }
            Some(once) => {
                let once = Arc::clone(once);
                drop(exclusive);
                once.wait();
                None
            }
        }
    }
}
