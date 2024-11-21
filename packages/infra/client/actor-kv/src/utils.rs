use foundationdb as fdb;
use futures_util::{FutureExt, TryStreamExt};

pub trait TransactionExt {
	/// Owned version of `Transaction.get_ranges`.
	fn get_ranges_owned<'a>(
		self,
		opt: fdb::RangeOption<'a>,
		snapshot: bool,
	) -> impl futures_util::Stream<Item = fdb::FdbResult<fdb::future::FdbValues>>
	       + Send
	       + Sync
	       + Unpin
	       + 'a;

	/// Owned version of `Transaction.get_ranges_keyvalues`.
	fn get_ranges_keyvalues_owned<'a>(
		self,
		opt: fdb::RangeOption<'a>,
		snapshot: bool,
	) -> impl futures_util::Stream<Item = fdb::FdbResult<fdb::future::FdbValue>> + Unpin + 'a;
}

impl TransactionExt for fdb::RetryableTransaction {
	fn get_ranges_owned<'a>(
		self,
		opt: fdb::RangeOption<'a>,
		snapshot: bool,
	) -> impl futures_util::Stream<Item = fdb::FdbResult<fdb::future::FdbValues>>
	       + Send
	       + Sync
	       + Unpin
	       + 'a {
		futures_util::stream::unfold((1, Some(opt)), move |(iteration, maybe_opt)| {
			if let Some(opt) = maybe_opt {
				futures_util::future::Either::Left(
					self.get_range(&opt, iteration as usize, snapshot)
						.map(move |maybe_values| {
							let next_opt = match &maybe_values {
								Ok(values) => opt.next_range(values),
								Err(..) => None,
							};
							Some((maybe_values, (iteration + 1, next_opt)))
						}),
				)
			} else {
				futures_util::future::Either::Right(std::future::ready(None))
			}
		})
	}

	fn get_ranges_keyvalues_owned<'a>(
		self,
		opt: fdb::RangeOption<'a>,
		snapshot: bool,
	) -> impl futures_util::Stream<Item = fdb::FdbResult<fdb::future::FdbValue>> + Unpin + 'a {
		self.get_ranges_owned(opt, snapshot)
			.map_ok(|values| futures_util::stream::iter(values.into_iter().map(Ok)))
			.try_flatten()
	}
}

pub fn now() -> i64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_else(|err| unreachable!("time is broken: {}", err))
		.as_millis()
		.try_into()
		.expect("now doesn't fit in i64")
}
