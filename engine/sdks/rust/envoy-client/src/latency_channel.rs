use std::time::Duration;

use tokio::sync::mpsc;

/// Debug-only wrapper around an `mpsc::UnboundedReceiver` that injects configurable
/// latency on each receive. Used for testing reconnection behavior under latency.
pub struct LatencyReceiver<T> {
	rx: mpsc::UnboundedReceiver<T>,
	latency: Option<Duration>,
}

impl<T> LatencyReceiver<T> {
	pub fn new(rx: mpsc::UnboundedReceiver<T>, latency_ms: Option<u64>) -> Self {
		Self {
			rx,
			latency: latency_ms.filter(|&ms| ms > 0).map(Duration::from_millis),
		}
	}

	pub async fn recv(&mut self) -> Option<T> {
		let item = self.rx.recv().await?;
		if let Some(latency) = self.latency {
			tokio::time::sleep(latency).await;
		}
		Some(item)
	}
}
