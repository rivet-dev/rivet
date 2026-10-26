//! Outcomes of SlateDB capability calls that state machines consume. Kept outside the
//! `slatedb` capability so wasm builds without SlateDB can still name them.

/// A retained checkpoint is removable only after every other checkpoint has gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DeleteDatabaseResult {
	Pinned,
	Deleted,
}
