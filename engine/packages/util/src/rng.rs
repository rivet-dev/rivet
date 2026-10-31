//! Extensions for `rivet_env::RngStream` that build Rivet identifiers.

use rivet_env::RngStream;

use crate::Id;

/// Draws identifiers from a labelled environment stream so they replay under simulation.
/// Identifiers from this stream are not secrets; use
/// `rivet_env::backends::production::secure_random` for anything that must be unguessable.
pub trait RngStreamIdExt {
	fn id_v1(&mut self, label: u16) -> Id;
}

impl RngStreamIdExt for RngStream {
	fn id_v1(&mut self, label: u16) -> Id {
		Id::v1(self.uuid_v4(), label)
	}
}
