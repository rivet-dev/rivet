use std::{convert::TryInto, time};

use anyhow::*;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};

pub fn now() -> i64 {
	time::SystemTime::now()
		.duration_since(time::UNIX_EPOCH)
		.unwrap_or_else(|err| unreachable!("time is broken: {}", err))
		.as_millis()
		.try_into()
		.expect("now doesn't fit in i64")
}

pub fn end_of_month(ts: i64) -> Result<NaiveDateTime> {
	let nanos = (ts % 1000 * 1_000_000).try_into()?;

	// Get year and month of current month
	let current_date =
		DateTime::from_timestamp(ts / 1000, nanos).context("failed converting ms to datetime")?;
	let year = current_date.year();
	let month = current_date.month();

	let date = NaiveDate::from_ymd_opt(year, month + 1, 1)
		.or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1))
		.and_then(|date| date.and_hms_opt(0, 0, 0))
		.context("failed constructing eom date")?;

	Ok(date - Duration::milliseconds(1))
}

pub fn to_chrono(ts: i64) -> Result<DateTime<Utc>> {
	let nanos = (ts % 1000 * 1_000_000).try_into()?;
	let local = Utc.timestamp_opt(ts / 1000, nanos).latest();
	local.context("failed converting ms to chrono")
}

pub fn to_string(ts: i64) -> Result<String> {
	Ok(to_chrono(ts)?.to_rfc3339_openapi())
}

pub trait DateTimeExt {
	fn to_rfc7231(&self) -> String;
	fn to_rfc3339_openapi(&self) -> String;
}

impl<Tz: TimeZone> DateTimeExt for DateTime<Tz>
where
	Tz::Offset: core::fmt::Display,
{
	fn to_rfc7231(&self) -> String {
		self.naive_utc().format("%a, %d %b %Y %T GMT").to_string()
	}

	fn to_rfc3339_openapi(&self) -> String {
		self.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
	}
}
