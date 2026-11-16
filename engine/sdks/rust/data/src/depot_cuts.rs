use crate::generated::shared_depot_types_v1::VolumeLogPosition;
use anyhow::{Result, ensure};

/// The claim log of a volume's default claim on `/`. Its log keeps the volume's base key, and
/// every volume written before claims existed is this log alone.
pub const ROOT_CLAIM_LOG: u16 = 0;

/// The cut helpers, implemented for every schema version that defines cuts. Only version 2,
/// the latest, has cuts so far.
macro_rules! cut_helpers {
	($module:ident) => {
		impl crate::generated::$module::FsCut {
			/// A cut over claim log 0 alone: the cut of a volume with one claim.
			pub fn single(position: VolumeLogPosition) -> Self {
				Self {
					logs: vec![crate::generated::$module::FsClaimLogPosition {
						claim_log: ROOT_CLAIM_LOG,
						position,
					}],
				}
			}

			/// The position of `claim_log` in this cut, if the cut names it.
			pub fn position(&self, claim_log: u16) -> Option<&VolumeLogPosition> {
				self.logs
					.iter()
					.find(|log| log.claim_log == claim_log)
					.map(|log| &log.position)
			}

			/// The position of claim log 0 in this cut, if the cut names it.
			pub fn root_position(&self) -> Option<&VolumeLogPosition> {
				self.position(ROOT_CLAIM_LOG)
			}

			/// Whether the version at `seq` written by `claim_log` is inside this cut: the cut names
			/// the log and the version is at or below the log's position.
			pub fn contains(&self, claim_log: u16, seq: u64) -> bool {
				self.position(claim_log)
					.is_some_and(|position| seq <= position.volume_seq)
			}

			/// The one position of a cut over claim log 0 alone, borrowed. Refuses a cut over any other
			/// log, which a volume with one claim never has.
			pub fn single_position(&self) -> Result<&VolumeLogPosition> {
				let [only] = self.logs.as_slice() else {
					anyhow::bail!("the cut does not name exactly one claim log");
				};
				ensure!(
					only.claim_log == ROOT_CLAIM_LOG,
					"the cut names claim log {}, not claim log 0",
					only.claim_log
				);
				Ok(&only.position)
			}

			/// The one position of a cut over claim log 0 alone. Refuses a cut over any other log, which
			/// a version 1 position cannot carry.
			pub fn into_single(self) -> Result<VolumeLogPosition> {
				let mut logs = self.logs.into_iter();
				let (Some(first), None) = (logs.next(), logs.next()) else {
					anyhow::bail!("the cut does not name exactly one claim log");
				};
				ensure!(
					first.claim_log == ROOT_CLAIM_LOG,
					"the cut names claim log {}, not claim log 0",
					first.claim_log
				);
				Ok(first.position)
			}
		}
	};
}

cut_helpers!(shared_depot_fs_v2);
