//! Downgrades from version 2 to version 1 of the V2 migrator family. Version 1 has no volume
//! group, so it expresses only an entry at the legacy unprefixed placement and refuses one placed
//! in a group as [`Inexpressible`]: a binary that reads only version 1 would address the actor at
//! the wrong placement. The entry's namespace is dropped rather than refused: version 1 is written
//! while a storage worker that reads only version 1 is listed, and refusing would stop every
//! migrator write for the whole roll. An entry read back without it is treated as belonging to
//! any namespace, which only holds more backfills. Every type that holds no entry is the same at
//! both versions and converts through its generated `From`.
use crate::{
	generated::{pegboard_v2_migrator_v1 as v1, pegboard_v2_migrator_v2 as v2},
	versioned::Inexpressible,
};
use anyhow::Result;

const GROUPED_ENTRIES: &str = "in-flight entries placed in a volume group";

pub(super) fn in_flight(x: v2::MigrateFromV2InFlight) -> Result<v1::MigrateFromV2InFlight> {
	if x.group.is_some() {
		return Err(Inexpressible::lacks(1, GROUPED_ENTRIES));
	}
	Ok(v1::MigrateFromV2InFlight {
		migration_id: x.migration_id,
		v3_actor_id: x.v3_actor_id,
		candidate: x.candidate.into(),
		started_at: x.started_at,
		phase: x.phase.into(),
	})
}

pub(super) fn state_write_request(
	x: v2::MigrateFromV2StateWriteRequest,
) -> Result<v1::MigrateFromV2StateWriteRequest> {
	Ok(v1::MigrateFromV2StateWriteRequest {
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
	x: v2::MigrateFromV2StateLoadResponse,
) -> Result<v1::MigrateFromV2StateLoadResponse> {
	Ok(match x {
		v2::MigrateFromV2StateLoadResponse::MigrateFromV2StateLoaded(loaded) => {
			v1::MigrateFromV2StateLoadResponse::MigrateFromV2StateLoaded(
				v1::MigrateFromV2StateLoaded {
					pass: loaded.pass.map(Into::into),
					in_flight: loaded
						.in_flight
						.into_iter()
						.map(in_flight)
						.collect::<Result<Vec<_>>>()?,
				},
			)
		}
		v2::MigrateFromV2StateLoadResponse::MigrateFromV2StateFenced(fenced) => {
			v1::MigrateFromV2StateLoadResponse::MigrateFromV2StateFenced(fenced.into())
		}
	})
}
