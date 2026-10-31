//! Alarm-index initialization accompanies the original manifest in the same CAS.
use super::*;
use rivet_data::generated::{depot_tablet_coordinator_protocol_v1::*, depot_tablet_manifest_v1::*};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct AlarmManifestState {
	pub enabled: bool,
	initialization: BTreeMap<vbare::Str, AlarmIndexInitialization>,
	holds: Vec<AlarmIndexHold>,
	pub commands: BTreeMap<u64, AlarmCoordinatorRequest>,
}
pub(crate) fn normalize(
	request: AlarmCoordinatorRequest,
) -> (Request, Option<AlarmCoordinatorRequest>) {
	match request {
		AlarmCoordinatorRequest::TabletCoordinatorRequest(request) => (request, None),
		AlarmCoordinatorRequest::AlarmSplitPrepared(ref report) => {
			(Request::SplitPrepared(report.report.clone()), Some(request))
		}
		AlarmCoordinatorRequest::AlarmIndexReady(ref report) => (
			Request::Repair(
				rivet_data::generated::depot_tablet_coordinator_protocol_v1::Repair {
					tablet_id: report.tablet_id,
				},
			),
			Some(request),
		),
	}
}
impl AlarmManifestState {
	pub(crate) fn install(&mut self, envelope: &AlarmTabletManifest) {
		self.enabled = true;
		self.initialization = envelope.initialization.clone();
		self.holds = envelope.holds.clone();
	}
	pub(crate) fn envelope(&self, manifest: TabletManifest) -> AlarmTabletManifest {
		AlarmTabletManifest {
			manifest,
			initialization: self.initialization.clone(),
			holds: self.holds.clone(),
		}
	}
	pub(crate) fn apply(
		&mut self,
		manifest: &mut TabletManifest,
		command: &Command,
		special: Option<AlarmCoordinatorRequest>,
		discovery: Option<&super::StopTopology>,
	) -> Result<bool, Reject> {
		if let Some(AlarmCoordinatorRequest::AlarmIndexReady(report)) = special.as_ref() {
			let tablet = manifest
				.tablets
				.iter()
				.find(|t| t.id == report.tablet_id)
				.ok_or(Reject::UnknownTablet)?;
			if tablet.maintainer != Some(report.maintainer)
				|| tablet.tablet_grant_epoch != report.tablet_grant_epoch
			{
				return Err(Reject::InvalidCommand);
			}
			let init = self
				.initialization
				.get_mut(&key(tablet.id))
				.ok_or(Reject::InvalidCommand)?;
			if init.source.as_ref() != Some(&report.source) {
				return Err(Reject::InvalidCommand);
			}
			if init.phase == AlarmIndexPhase::Ready {
				return Ok(false);
			}
			if init.phase != AlarmIndexPhase::CloneNeedsBootstrap {
				return Err(Reject::InvalidCommand);
			}
			init.phase = AlarmIndexPhase::Ready;
			self.holds.retain(|hold| {
				![hold.left_id, hold.right_id].iter().all(|id| {
					self.initialization
						.get(&key(*id))
						.is_some_and(|i| i.phase == AlarmIndexPhase::Ready)
				})
			});
			return Ok(true);
		}
		if let Request::SplitTablet(report) = &command.request {
			if self
				.holds
				.iter()
				.any(|h| [h.left_id, h.right_id].contains(&report.tablet_id))
				|| self
					.initialization
					.get(&key(report.tablet_id))
					.is_none_or(|i| i.phase == AlarmIndexPhase::CloneNeedsBootstrap)
			{
				return Err(Reject::InvalidCommand);
			}
		}
		if let Request::RetiredTabletDeleted(report) = &command.request {
			if self.holds.iter().any(|h| h.parent_id == report.tablet_id) {
				return Err(Reject::InvalidCommand);
			}
		}
		let parent = if let Request::SplitPrepared(report) = &command.request {
			let Some(AlarmCoordinatorRequest::AlarmSplitPrepared(aware)) = special.as_ref() else {
				return Err(Reject::InvalidCommand);
			};
			let parent = manifest
				.tablets
				.iter()
				.find(|t| t.id == report.tablet_id)
				.cloned();
			if parent
				.as_ref()
				.is_some_and(|p| p.maintainer != Some(aware.maintainer))
			{
				return Err(Reject::InvalidCommand);
			}
			parent
		} else {
			None
		};
		let changed = super::apply(manifest, command, discovery)?;
		if !changed {
			return Ok(false);
		}
		if let (Some(parent), Some(AlarmCoordinatorRequest::AlarmSplitPrepared(report))) =
			(parent, special)
		{
			self.split_prepared(&parent, manifest, report.manifest_id)?;
		}
		self.fill_native_empty(manifest);
		Ok(true)
	}

	/// Records the children of `parent`'s published split as clones of its checkpoint whose
	/// alarm index still needs a bootstrap, and holds the parent's retirement until both
	/// report ready. `manifest` already lists the children in place of the parent.
	pub(crate) fn split_prepared(
		&mut self,
		parent: &Tablet,
		manifest: &TabletManifest,
		manifest_id: u64,
	) -> Result<(), Reject> {
		let split = parent.split.as_ref().ok_or(Reject::InvalidCommand)?;
		self.initialization.remove(&key(parent.id));
		for id in [split.left_tablet_id, split.right_tablet_id] {
			let child = manifest
				.tablets
				.iter()
				.find(|t| t.id == id)
				.ok_or(Reject::InvalidCommand)?;
			self.initialization.insert(
				key(id),
				AlarmIndexInitialization {
					phase: AlarmIndexPhase::CloneNeedsBootstrap,
					source: Some(AlarmIndexSource {
						parent_id: parent.id,
						checkpoint_id: split.checkpoint_id,
						manifest_id,
						hash_range: child.hash_range.clone(),
					}),
				},
			);
		}
		self.holds.push(AlarmIndexHold {
			parent_id: parent.id,
			left_id: split.left_tablet_id,
			right_id: split.right_tablet_id,
		});
		Ok(())
	}

	/// Gives every tablet of `manifest` without an alarm index entry an empty native one.
	pub(crate) fn fill_native_empty(&mut self, manifest: &TabletManifest) {
		for tablet in &manifest.tablets {
			self.initialization
				.entry(key(tablet.id))
				.or_insert(AlarmIndexInitialization {
					phase: AlarmIndexPhase::NativeEmpty,
					source: None,
				});
		}
	}
}
pub(crate) fn validate(envelope: &AlarmTabletManifest) -> anyhow::Result<()> {
	anyhow::ensure!(
		envelope.initialization.len() == envelope.tablets.len(),
		"incomplete alarm initialization map"
	);
	for tablet in &envelope.tablets {
		let init = envelope
			.initialization
			.get(&key(tablet.id))
			.ok_or_else(|| anyhow::anyhow!("missing alarm initialization"))?;
		match init.phase {
			AlarmIndexPhase::LegacyEmpty | AlarmIndexPhase::NativeEmpty => {
				anyhow::ensure!(init.source.is_none(), "unexpected clone source")
			}
			AlarmIndexPhase::CloneNeedsBootstrap | AlarmIndexPhase::Ready => {
				let source = init
					.source
					.as_ref()
					.ok_or_else(|| anyhow::anyhow!("missing clone source"))?;
				anyhow::ensure!(
					source.parent_id != tablet.id
						&& source.parent_id != [0; 16]
						&& source.checkpoint_id != [0; 16]
						&& source.hash_range == tablet.hash_range,
					"invalid clone source"
				);
				if init.phase == AlarmIndexPhase::CloneNeedsBootstrap {
					anyhow::ensure!(
						envelope
							.holds
							.iter()
							.any(|h| h.parent_id == source.parent_id
								&& [h.left_id, h.right_id].contains(&tablet.id)),
						"unheld clone source"
					);
				}
			}
		}
	}
	for hold in &envelope.holds {
		anyhow::ensure!(
			hold.left_id != hold.right_id
				&& envelope
					.retired_tablets
					.iter()
					.any(|t| t.tablet_id == hold.parent_id)
				&& [hold.left_id, hold.right_id].iter().all(|id| envelope
					.initialization
					.get(&key(*id))
					.is_some_and(|i| i
						.source
						.as_ref()
						.is_some_and(|s| s.parent_id == hold.parent_id))),
			"invalid alarm source hold"
		);
	}
	Ok(())
}

pub(crate) fn key(id: [u8; 16]) -> vbare::Str {
	uuid::Uuid::from_bytes(id).to_string().into()
}


