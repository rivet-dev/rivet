#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("missing ups pool")]
	MissingUpsPool,
}
