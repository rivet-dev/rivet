use crate::generated::{sqlite_db_overflow_v1, sqlite_db_recent_v1};

/// A row of a SQLite database's `main.db-recent` KV inode.
pub enum SqliteDbRecentData {
	V1(sqlite_db_recent_v1::Data),
}

impl_versioned_v1!(SqliteDbRecentData, sqlite_db_recent_v1::Data);

/// A row of a SQLite database's `main.db-overflow` KV inode.
pub enum SqliteDbOverflowData {
	V1(sqlite_db_overflow_v1::OverflowRun),
}

impl_versioned_v1!(SqliteDbOverflowData, sqlite_db_overflow_v1::OverflowRun);
