use anyhow::Result;
use futures_util::TryStreamExt;
use gas::prelude::*;
use rivet_key_data::generated::pegboard_runner_address_v1::Data as AddressKeyData;
use rivet_types::runners::Runner;
use udb_util::{FormalChunkedKey, SERIALIZABLE, SNAPSHOT, TxnExt};
use universaldb::{self as udb, options::StreamingMode};

use crate::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub runner_ids: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub runners: Vec<Runner>,
}

#[operation]
pub async fn pegboard_runner_get(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let dc_name = ctx.config().dc_name()?.to_string();

	let runners = ctx
		.udb()?
		.run(|tx, _mc| {
			let dc_name = dc_name.to_string();
			async move {
				let mut runners = Vec::new();

				for runner_id in input.runner_ids.clone() {
					if let Some(runner) = get_inner(&dc_name, &tx, runner_id).await? {
						runners.push(runner);
					}
				}

				std::result::Result::<_, udb::FdbBindingError>::Ok(runners)
			}
		})
		.await?;

	Ok(Output { runners })
}

pub(crate) async fn get_inner(
	dc_name: &str,
	tx: &udb::Transaction,
	runner_id: Id,
) -> std::result::Result<Option<Runner>, udb::FdbBindingError> {
	let txs = tx.subspace(keys::subspace());

	// TODO: Make this part of the below try join to reduce round trip count
	// Check if runner exists by looking for workflow ID
	if !txs
		.exists(&keys::runner::WorkflowIdKey::new(runner_id), SERIALIZABLE)
		.await?
	{
		return std::result::Result::Ok(None);
	}

	let namespace_id_key = keys::runner::NamespaceIdKey::new(runner_id);
	let name_key = keys::runner::NameKey::new(runner_id);
	let key_key = keys::runner::KeyKey::new(runner_id);
	let version_key = keys::runner::VersionKey::new(runner_id);
	let total_slots_key = keys::runner::TotalSlotsKey::new(runner_id);
	let remaining_slots_key = keys::runner::RemainingSlotsKey::new(runner_id);
	let create_ts_key = keys::runner::CreateTsKey::new(runner_id);
	let connected_ts_key = keys::runner::ConnectedTsKey::new(runner_id);
	let drain_ts_key = keys::runner::DrainTsKey::new(runner_id);
	let stop_ts_key = keys::runner::StopTsKey::new(runner_id);
	let last_ping_ts_key = keys::runner::LastPingTsKey::new(runner_id);
	let last_rtt_key = keys::runner::LastRttKey::new(runner_id);
	let metadata_key = keys::runner::MetadataKey::new(runner_id);
	let metadata_subspace = txs.subspace(&metadata_key);

	let (
		namespace_id,
		name,
		key,
		version,
		total_slots,
		remaining_slots,
		create_ts,
		connected_ts,
		drain_ts,
		stop_ts,
		last_ping_ts,
		last_rtt,
		(addresses_http, addresses_tcp, addresses_udp),
		metadata_chunks,
	) = tokio::try_join!(
		// NOTE: These are not SERIALIZABLE because this op is meant for basic information (i.e. data for the
		// API)
		txs.read(&namespace_id_key, SNAPSHOT),
		txs.read(&name_key, SNAPSHOT),
		txs.read(&key_key, SNAPSHOT),
		txs.read(&version_key, SNAPSHOT),
		txs.read(&total_slots_key, SNAPSHOT),
		txs.read(&remaining_slots_key, SNAPSHOT),
		txs.read(&create_ts_key, SNAPSHOT),
		txs.read_opt(&connected_ts_key, SNAPSHOT),
		txs.read_opt(&drain_ts_key, SNAPSHOT),
		txs.read_opt(&stop_ts_key, SNAPSHOT),
		txs.read_opt(&last_ping_ts_key, SNAPSHOT),
		txs.read_opt(&last_rtt_key, SNAPSHOT),
		async {
			// Get addresses by scanning all address keys for this runner
			let mut addresses_http = util::serde::HashableMap::new();
			let mut addresses_tcp = util::serde::HashableMap::new();
			let mut addresses_udp = util::serde::HashableMap::new();

			let address_subspace = txs.subspace(&keys::runner::AddressKey::subspace(runner_id));

			let mut stream = txs.get_ranges_keyvalues(
				udb::RangeOption {
					mode: StreamingMode::Iterator,
					..(&address_subspace).into()
				},
				SNAPSHOT,
			);

			while let Some(entry) = stream.try_next().await? {
				let (address_key, address_data) =
					txs.read_entry::<keys::runner::AddressKey>(&entry)?;

				match address_data {
					AddressKeyData::Http(addr) => {
						addresses_http.insert(address_key.name.clone(), addr);
					}
					AddressKeyData::Tcp(addr) => {
						addresses_tcp.insert(address_key.name.clone(), addr);
					}
					AddressKeyData::Udp(addr) => {
						addresses_udp.insert(address_key.name.clone(), addr);
					}
				}
			}

			Ok((addresses_http, addresses_tcp, addresses_udp))
		},
		async {
			txs.get_ranges_keyvalues(
				udb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&metadata_subspace).into()
				},
				SNAPSHOT,
			)
			.try_collect::<Vec<_>>()
			.await
			.map_err(Into::into)
		},
	)?;

	let metadata = if metadata_chunks.is_empty() {
		None
	} else {
		Some(
			metadata_key
				.combine(metadata_chunks)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
				.metadata,
		)
	};

	std::result::Result::Ok(Some(Runner {
		runner_id,
		namespace_id,
		datacenter: dc_name.to_string(),
		name,
		key,
		version,
		total_slots,
		remaining_slots,
		addresses_http: addresses_http.into(),
		addresses_tcp: addresses_tcp.into(),
		addresses_udp: addresses_udp.into(),
		create_ts,
		last_connected_ts: connected_ts,
		drain_ts,
		stop_ts,
		last_ping_ts: last_ping_ts.unwrap_or_default(),
		last_rtt: last_rtt.unwrap_or_default(),
		metadata,
	}))
}
