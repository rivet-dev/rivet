use include_dir::Dir;

mod migrate;

pub use migrate::*;

#[derive(Clone, Debug)]
pub struct SqlService {
	pub kind: SqlServiceKind,
	pub migrations: Dir<'static>,
	pub db_name: &'static str,
}

#[derive(Clone, Debug)]
pub enum SqlServiceKind {
	CockroachDB,
	ClickHouse,
}
