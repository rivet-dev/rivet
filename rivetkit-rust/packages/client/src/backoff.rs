use std::{cmp, time::Duration};

pub struct Backoff {
    max_delay: Duration,
    delay: Duration,
}

impl Backoff {
    pub fn new(initial: Duration, max_delay: Duration) -> Self {
        Self {
            max_delay,
            delay: initial,
        }
    }

    pub fn delay(&self) -> Duration {
        self.delay
    }

    pub async fn tick(&mut self) {
        tokio::time::sleep(self.delay).await;
        self.delay = cmp::min(self.delay * 2, self.max_delay);
    }
}
