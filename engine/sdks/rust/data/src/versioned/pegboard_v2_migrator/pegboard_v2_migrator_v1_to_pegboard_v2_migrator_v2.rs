//! Upgrades from version 1 to version 2 of the V2 migrator family. Version 1 entries were all
//! placed at the legacy unprefixed placement, since no namespace had a volume group when 3.0.2 and
//! 3.0.3 wrote them, so they upgrade with no group. Every type that holds no entry is the same at
//! both versions and converts through its generated `From`.
use crate::generated::{pegboard_v2_migrator_v1 as v1, pegboard_v2_migrator_v2 as v2};
use anyhow::Result;

pub(super) fn in_flight(x: v1::MigrateFromV2InFlight) -> Result<v2::MigrateFromV2InFlight> {
	Ok(v2::MigrateFromV2InFlight {
		migration_id: x.migration_id,
		v3_actor_id: x.v3_actor_id,
		candidate: x.candidate.into(),
		started_at: x.started_at,
		phase: x.phase.into(),
		group: None,
		namespace: None,
	})
}

pub(super) fn state_write_request(
	x: v1::MigrateFromV2StateWriteRequest,
) -> Result<v2::MigrateFromV2StateWriteRequest> {
	Ok(v2::MigrateFromV2StateWriteRequest {
		coordinator_generation: x.coordinator_generation,
		pass: x.pass.map(Into::into),
		puts: x
			.puts
			.into_iter()
			.map(in_flight)
			.collect::<Result<Vec<_>>>()?,
		deletes: x.deletes,
	})
}

pub(super) fn state_load_response(
	x: v1::MigrateFromV2StateLoadResponse,
) -> Result<v2::MigrateFromV2StateLoadResponse> {
	Ok(match x {
		v1::MigrateFromV2StateLoadResponse::MigrateFromV2StateLoaded(loaded) => {
			v2::MigrateFromV2StateLoadResponse::MigrateFromV2StateLoaded(
				v2::MigrateFromV2StateLoaded {
					pass: loaded.pass.map(Into::into),
					in_flight: loaded
						.in_flight
						.into_iter()
						.map(in_flight)
						.collect::<Result<Vec<_>>>()?,
				},
			)
		}
		v1::MigrateFromV2StateLoadResponse::MigrateFromV2StateFenced(fenced) => {
			v2::MigrateFromV2StateLoadResponse::MigrateFromV2StateFenced(fenced.into())
		}
	})
}
