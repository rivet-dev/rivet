use chrono::Duration;

pub trait ToChrono {
	fn to_chrono(self) -> Duration;
}

impl ToChrono for i64 {
	fn to_chrono(self) -> Duration {
		Duration::milliseconds(self)
	}
}

pub const fn seconds(v: i64) -> i64 {
	v * 1000
}

pub const fn minutes(v: i64) -> i64 {
	seconds(v) * 60
}

pub const fn hours(v: i64) -> i64 {
	minutes(v) * 60
}

pub const fn days(v: i64) -> i64 {
	hours(v) * 24
}

/// Returns a short readable duration string for a given duration of milliseconds (500000 -> 5m 20s)
pub fn format(v: i64, with_ms: bool) -> String {
	if with_ms && v < seconds(1) {
		return format!("{}ms", v);
	}

	let seconds = v % minutes(1) / seconds(1);
	let minutes = v % hours(1) / minutes(1);
	let hours = v % days(1) / hours(1);
	let days = v / days(1);

	let segments = IntoIterator::into_iter([days, hours, minutes])
		.enumerate()
		.filter_map(|(i, x)| {
			if x != 0 {
				let suffix = match i {
					0 => "d",
					1 => "h",
					2 => "m",
					_ => "null",
				};

				Some(format!("{}{} ", x, suffix))
			} else {
				None
			}
		})
		.collect::<String>();

	format!("{}{}s", segments, seconds)
}


