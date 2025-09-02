use anyhow::{Context, Result};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub trait TsToMillis {
	fn to_millis(self) -> Result<i64>;
}

impl TsToMillis for i64 {
	fn to_millis(self) -> Result<i64> {
		Ok(self)
	}
}

impl TsToMillis for Instant {
	fn to_millis(self) -> Result<i64> {
		let now_instant = Instant::now();

		let system_time = if self >= now_instant {
			SystemTime::now().checked_add(self.duration_since(now_instant))
		} else {
			SystemTime::now().checked_sub(now_instant.duration_since(self))
		};

		let ms = system_time
			.context("invalid timestamp")?
			.duration_since(SystemTime::UNIX_EPOCH)?
			.as_millis()
			.try_into()?;

		Ok(ms)
	}
}

impl TsToMillis for tokio::time::Instant {
	fn to_millis(self) -> Result<i64> {
		self.into_std().to_millis()
	}
}

impl TsToMillis for SystemTime {
	fn to_millis(self) -> Result<i64> {
		let ms = self
			.duration_since(SystemTime::UNIX_EPOCH)?
			.as_millis()
			.try_into()?;

		Ok(ms)
	}
}

pub trait DurationToMillis {
	fn to_millis(self) -> Result<u64>;
}

impl DurationToMillis for i64 {
	fn to_millis(self) -> Result<u64> {
		self.try_into().map_err(Into::into)
	}
}

impl DurationToMillis for i32 {
	fn to_millis(self) -> Result<u64> {
		self.try_into().map_err(Into::into)
	}
}

impl DurationToMillis for u64 {
	fn to_millis(self) -> Result<u64> {
		Ok(self)
	}
}

impl DurationToMillis for Duration {
	fn to_millis(self) -> Result<u64> {
		Ok(self.as_millis().try_into()?)
	}
}

#[tracing::instrument(skip_all)]
pub async fn sleep_until_ts(ts: u64) {
	let target_time = UNIX_EPOCH + Duration::from_millis(ts);
	if let Ok(sleep_duration) = target_time.duration_since(SystemTime::now()) {
		tokio::time::sleep(sleep_duration).await;
	}
}
