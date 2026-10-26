use std::time::Duration;

use futures_util::TryStreamExt;
use gas::prelude::*;
use indoc::formatdoc;
use universaldb::prelude::*;

mod keys;

const PROTOCOL_VERSION_ENTRY_EXPIRED_THRESHOLD_MS: i64 = 30000;
const PROTOCOL_VERSION_CHECK_INTERVAL: Duration = Duration::from_secs(15);

#[tracing::instrument(skip_all)]
pub async fn start(
	config: rivet_config::Config,
	pools: rivet_engine_context::EngineContext,
) -> Result<()> {
	let mut check_interval = tokio::time::interval(PROTOCOL_VERSION_CHECK_INTERVAL);
	check_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

	let mut last_protocols = config.protocols();

	loop {
		check_interval.tick().await;

		let protocols = pools
			.udb()?
			.txn("engine_check_protocol_versions", |tx| async move {
				update_protocol_versions(&tx).await
			})
			.await?;

		if protocols != *last_protocols {
			for protocol in &protocols {
				if protocol.version() < protocol.compiled_version() {
					tracing::info!(
						kind = %protocol.kind,
						version = protocol.version(),
						compiled_version = protocol.compiled_version(),
						"protocol pinned below compiled version by an older engine"
					);
				} else {
					tracing::info!(
						kind = %protocol.kind,
						version = protocol.version(),
						"protocol running at compiled version"
					);
				}
			}
		}

		config.set_protocols(protocols);
		last_protocols = config.protocols();
	}
}

/// Verifies that no rollback has occurred (if allowing rollback is disabled) and that protocol versions are
/// allowed.
pub async fn check_engine_admission(
	config: &rivet_config::Config,
	pools: &rivet_engine_context::EngineContext,
) -> Result<()> {
	let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
		.context("failed to parse cargo pkg version as semver")?;

	tracing::debug!(%current_version, "checking engine admission");

	let protocols = pools
		.udb()?
		.txn("engine_check_admission", |tx|  {
			let config = config.clone();
			let current_version = current_version.clone();
			async move {
				if let Some(existing_version) = tx.read_opt(&keys::EngineVersionKey::new(), Serializable).await? {
					if !config.runtime.allow_version_rollback() {
						if current_version < existing_version {
							bail!(
								"{}",
								formatdoc!(
									"
									Rivet Engine has been rolled back to a previous version:
									- Last Used Version: {existing_version}
									- Current Version:   {current_version}
									Cannot proceed without potential data corruption.
									
									(If you know what you're doing, this error can be disabled in the Rivet config via `allow_version_rollback: true`)
									"
								)
							);
						}
					}
				}

				// NOTE: We serializably read this key and write it in the same txn making all engine
				// connections serial. This is by design
				tx.write(&keys::EngineVersionKey::new(), current_version)?;

				// NOTE: These writes rely on the conflict guarantees of the above `EngineVersionKey`
				update_protocol_versions(&tx).await
			}
		})
		.await?;

	config.set_protocols(protocols);

	Ok(())
}

async fn update_protocol_versions(
	tx: &universaldb::Transaction,
) -> Result<rivet_config::RuntimeProtocols> {
	// this datacenter. Include every reader's capability before enabling cross-region upgrades.
	let now = util::timestamp::now();
	let mut protocols = rivet_build_meta::compiled_runtime_protocols();

	for protocol in &mut protocols {
		let protocol_version_subspace = universaldb::utils::Subspace::all()
			.subspace(&keys::ProtocolVersionKey::subspace(protocol.kind));
		// Read versions in increasing order to get the most out of date version first
		let mut stream = tx.get_ranges_keyvalues(
			universaldb::RangeOption {
				mode: StreamingMode::Small,
				..(&protocol_version_subspace).into()
			},
			Serializable,
		);

		while let Some(entry) = stream.try_next().await? {
			let (existing_key, update_ts) = tx.read_entry::<keys::ProtocolVersionKey>(&entry)?;

			// Check expired and delete
			if update_ts < now - PROTOCOL_VERSION_ENTRY_EXPIRED_THRESHOLD_MS {
				tracing::debug!(
					kind = %protocol.kind,
					version = existing_key.version,
					last_update_ts = update_ts,
					age_ms = now - update_ts,
					"deleting expired protocol version entry"
				);

				tx.delete(&existing_key);
				continue;
			}

			if protocol.compiled_version() < existing_key.version {
				bail!(
					"{}",
					formatdoc!(
						"
						Rivet Engine has been rolled back to a previous version:
						- Last Used `{0}` Protocol Version: {1}
						- Current `{0}` Protocol Version:   {2}
						Cannot proceed without potential data corruption.
						",
						protocol.kind,
						existing_key.version,
						protocol.compiled_version(),
					)
				);
			}

			protocol.set_override_version(existing_key.version);
			break;
		}

		// NOTE: Must write after reading to prevent RYOW
		tx.atomic_op(
			&keys::ProtocolVersionKey::new(protocol.kind, protocol.compiled_version()),
			&now.to_le_bytes(),
			MutationType::Max,
		);
	}

	Ok(protocols)
}


