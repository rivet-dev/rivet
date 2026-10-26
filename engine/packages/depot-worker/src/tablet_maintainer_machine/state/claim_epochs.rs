//! Claim epochs for volume clients, derived from the tablet grant.
//!
//! An open, recovery, drain or delete claims its volume at an epoch above every epoch that
//! volume's replicas observed, and no two clients may ever be handed the same epoch. A
//! restarted maintainer cannot see every claim an earlier lifetime granted, so the epoch
//! carries the lifetime: `(grant << 32) | counter`. Every lifetime runs under a grant no
//! earlier lifetime of the tablet held. The supervisor asks for a fresh grant at every
//! start and after every stop, grants only rise along a tablet's lineage (a split child's
//! grant is above its parent's), and the tablet store accepts each grant's leadership
//! epoch once, which stops a second lifetime under one grant before it issues anything.
//! So every epoch a lifetime issues sorts above every epoch an earlier lifetime issued,
//! without a durable write. The replicas' strict claim check stays the safety net.

use super::MaintainerState;
use crate::tablet_maintainer_machine::Effects;
use serde::{Deserialize, Serialize};

/// The claim counter of this lifetime's grant.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ClaimEpochs {
	/// The low 32 bits of the last epoch this lifetime issued, or 0 before the first. At
	/// `u32::MAX` the grant's epochs are exhausted and `drive` fences the maintainer, so its
	/// supervisor asks for a fresh grant.
	issued: u32,
}

/// The first claim epoch of a lifetime under `grant_epoch`, or `None` when the grant does
/// not fit the high 32 bits.
pub(crate) fn band(grant_epoch: u64) -> Option<u64> {
	u32::try_from(grant_epoch)
		.ok()
		.map(|grant| u64::from(grant) << 32)
}

/// Issues a claim epoch of at least `min` from this lifetime's band, or `None` when none can
/// be issued: the grant does not fit, its band is exhausted, or `min` lies above the band,
/// which means a newer grant issued it. Every caller turns `None` into a retryable refusal.
pub(crate) fn issue(state: &mut MaintainerState, min: u64) -> Option<u64> {
	let grant_epoch = state.tablet.tablet_grant_epoch;
	let Some(band) = band(grant_epoch) else {
		tracing::error!(
			grant_epoch,
			"tablet grant does not fit a claim epoch; refusing new claims"
		);
		return None;
	};
	let band_end = band | u64::from(u32::MAX);
	if min > band_end {
		tracing::error!(
			grant_epoch,
			min,
			"a volume holds a claim epoch of a newer grant; refusing new claims"
		);
		return None;
	}
	let issued = u64::from(state.claim_epochs.issued);
	if issued == u64::from(u32::MAX) {
		tracing::error!(
			grant_epoch,
			"claim epochs of this grant are exhausted; refusing new claims until a fresh grant"
		);
		return None;
	}
	let below_band_min = min.saturating_sub(band);
	let low = (issued + 1).max(below_band_min);
	// `low` is at most `u32::MAX`: `issued` is below it and `min` is inside the band.
	let Ok(low) = u32::try_from(low) else {
		return None;
	};
	state.claim_epochs.issued = low;
	Some(band | u64::from(low))
}

/// Fences a maintainer whose grant has no claim epoch left, so its supervisor recovers the
/// tablet under a fresh grant with a new band.
pub(crate) fn drive(state: &mut MaintainerState, effects: &mut Effects) {
	if state.claim_epochs.issued == u32::MAX {
		super::fence(state, effects);
	}
}
