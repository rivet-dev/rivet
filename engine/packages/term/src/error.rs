pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("io: {source}")]
	Io {
		#[from]
		source: std::io::Error,
	},
	#[error("join: {source}")]
	Join {
		#[from]
		source: tokio::task::JoinError,
	},
}
