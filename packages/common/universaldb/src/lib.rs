pub(crate) mod atomic;
mod database;
pub mod driver;
pub mod future;
pub mod inherited;
mod transaction;
pub(crate) mod tx_ops;
mod types;
pub mod utils;
pub mod versionstamp;

// Export UDB-specific types
pub use database::Database;
pub use driver::DatabaseDriverHandle;

// Re-export FDB types
pub use foundationdb_tuple as tuple;
pub use future::{FdbKeyValue, FdbValue};
pub use inherited::options;
pub use inherited::{
	error::FdbBindingError, error::FdbError, error::FdbResult, keyselector::KeySelector,
	rangeoption::RangeOption,
};
pub use options::DatabaseOption;
pub use transaction::{RetryableTransaction, Transaction};
pub use types::*;
pub use utils::calculate_tx_retry_backoff;
