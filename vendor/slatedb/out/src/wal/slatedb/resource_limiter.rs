use std::sync::{Arc, Mutex};

/// A cloneable, shared budget whose usage is released by dropping allocation guards.
///
/// Forced allocations may take usage above the configured limit. This is useful for
/// operations that must reserve resources to make forward progress.
#[derive(Clone, Debug)]
pub(crate) struct ResourceLimiter {
    inner: Arc<Mutex<ResourceLimiterInner>>,
}

impl ResourceLimiter {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ResourceLimiterInner { limit, usage: 0 })),
        }
    }

    pub(crate) fn allocate(&self, usage: usize, force: bool) -> Option<ResourceGuard> {
        let mut inner = self.inner.lock().expect("lock poisoned");
        inner.allocate(usage, force).then(|| ResourceGuard {
            usage,
            inner: self.inner.clone(),
        })
    }
}

pub(crate) struct ResourceGuard {
    usage: usize,
    inner: Arc<Mutex<ResourceLimiterInner>>,
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        self.inner
            .lock()
            .expect("lock poisoned")
            .release(self.usage);
    }
}

#[derive(Debug)]
struct ResourceLimiterInner {
    limit: usize,
    usage: usize,
}

impl ResourceLimiterInner {
    fn can_allocate(&self, usage: usize) -> bool {
        self.usage
            .checked_add(usage)
            .is_some_and(|new_usage| new_usage <= self.limit)
    }

    fn allocate(&mut self, usage: usize, force: bool) -> bool {
        if !force && !self.can_allocate(usage) {
            return false;
        }

        let Some(new_usage) = self.usage.checked_add(usage) else {
            return false;
        };
        self.usage = new_usage;
        true
    }

    fn release(&mut self, usage: usize) {
        self.usage = self
            .usage
            .checked_sub(usage)
            .expect("released more resources than were allocated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_enforce_limit_and_release_on_drop() {
        let limiter = ResourceLimiter::new(2);

        let first = limiter.allocate(2, false).expect("allocation should fit");
        assert!(limiter.allocate(1, false).is_none());

        drop(first);
        assert!(limiter.allocate(2, false).is_some());
    }

    #[test]
    fn should_allow_forced_allocation_over_limit() {
        let limiter = ResourceLimiter::new(1);

        let forced = limiter
            .allocate(2, true)
            .expect("forced allocation should exceed the limit");
        assert!(limiter.allocate(1, false).is_none());

        drop(forced);
        assert!(limiter.allocate(1, false).is_some());
    }
}
