use std::{any::Any, future::Future, pin::Pin, sync::Arc};

use crate::{
	FdbBindingError, FdbError, FdbResult, KeySelector, RangeOption, RetryableTransaction,
	Transaction,
	future::{FdbSlice, FdbValues},
	options::{ConflictRangeType, DatabaseOption, MutationType},
	types::{MaybeCommitted, TransactionCommitError, TransactionCommitted},
};

mod postgres;
pub mod rocksdb;

pub use postgres::PostgresDatabaseDriver;
pub use rocksdb::RocksDbDatabaseDriver;

pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type Erased = Box<dyn Any + Send>;

pub type DatabaseDriverHandle = Arc<dyn DatabaseDriver>;

pub trait DatabaseDriver: Send + Sync {
	fn create_trx(&self) -> FdbResult<Transaction>;
	fn run<'a>(
		&'a self,
		closure: Box<
			dyn Fn(
					RetryableTransaction,
					MaybeCommitted,
				) -> BoxFut<'a, Result<Erased, FdbBindingError>>
				+ Send
				+ Sync
				+ 'a,
		>,
	) -> BoxFut<'a, Result<Erased, FdbBindingError>>;
	fn set_option(&self, opt: DatabaseOption) -> FdbResult<()>;
}

pub trait TransactionDriver: Send + Sync {
	fn atomic_op(&self, key: &[u8], param: &[u8], op_type: MutationType);

	// Read operations
	fn get<'a>(
		&'a self,
		key: &[u8],
		snapshot: bool,
	) -> Pin<Box<dyn Future<Output = FdbResult<Option<FdbSlice>>> + Send + 'a>>;
	fn get_key<'a>(
		&'a self,
		selector: &KeySelector<'a>,
		snapshot: bool,
	) -> Pin<Box<dyn Future<Output = FdbResult<FdbSlice>> + Send + 'a>>;
	fn get_range<'a>(
		&'a self,
		opt: &RangeOption<'a>,
		iteration: usize,
		snapshot: bool,
	) -> Pin<Box<dyn Future<Output = FdbResult<FdbValues>> + Send + 'a>>;
	fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		snapshot: bool,
	) -> crate::future::FdbStream<'a, crate::future::FdbValue>;

	// Write operations
	fn set(&self, key: &[u8], value: &[u8]);
	fn clear(&self, key: &[u8]);
	fn clear_range(&self, begin: &[u8], end: &[u8]);

	// Transaction management
	fn commit(
		self: Box<Self>,
	) -> Pin<Box<dyn Future<Output = Result<TransactionCommitted, TransactionCommitError>> + Send>>;
	fn reset(&mut self);
	fn cancel(&self);
	fn add_conflict_range(
		&self,
		begin: &[u8],
		end: &[u8],
		conflict_type: ConflictRangeType,
	) -> FdbResult<()>;
	fn get_estimated_range_size_bytes<'a>(
		&'a self,
		begin: &'a [u8],
		end: &'a [u8],
	) -> Pin<Box<dyn Future<Output = FdbResult<i64>> + Send + 'a>>;

	// Helper for committing without consuming self (for database drivers that need it)
	fn commit_owned(&self) -> Pin<Box<dyn Future<Output = FdbResult<()>> + Send + '_>> {
		Box::pin(async move {
			// Default implementation returns error - drivers that need this should override
			Err(FdbError::from_code(1510))
		})
	}
}
