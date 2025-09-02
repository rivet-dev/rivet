use crate::{KeySelector, RangeOption, future::FdbValues};

pub fn calculate_tx_retry_backoff(attempt: usize) -> u64 {
	// TODO: Update this to mirror fdb 1:1:
	// https://github.com/apple/foundationdb/blob/21407341d9b49e1d343514a7a5f395bd5f232079/fdbclient/NativeAPI.actor.cpp#L3162

	let base_backoff_ms = 2_u64.pow((attempt as u32).min(10)) * 10;

	let jitter_ms = rand::random::<u64>() % 100;

	base_backoff_ms + jitter_ms
}

pub fn next_range<'a>(mut range: RangeOption<'a>, kvs: &'a FdbValues) -> Option<RangeOption<'a>> {
	if !kvs.more() {
		return None;
	}

	let last = kvs.iter().last()?;
	let last_key = last.key();

	if let Some(limit) = range.limit.as_mut() {
		*limit = limit.saturating_sub(kvs.len());
		if *limit == 0 {
			return None;
		}
	}

	if range.reverse {
		range.end = KeySelector::first_greater_or_equal(last_key);
	} else {
		range.begin = KeySelector::first_greater_than(last_key);
	}

	Some(range)
}
