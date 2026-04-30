use anyhow::{Context, Result, bail};
use universaldb::utils::IsolationLevel::Serializable;

use crate::{
	compactor::SqliteColdCompactPayload,
	pump::{
		keys,
		types::{PinStatus, decode_pinned_bookmark_record, encode_pinned_bookmark_record},
	},
};

use super::{
	phase_a::{
		ColdCompactState, ColdPhaseAPlan, decode_cold_compact_state, encode_cold_compact_state,
	},
	phase_b::{ColdPhaseBOutput, ColdUploadedPin},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColdPhaseCOutput {
	pub cold_drained_txid: u64,
	pub ready_pins: usize,
}

pub(crate) async fn run(
	db: &universaldb::Database,
	plan: &ColdPhaseAPlan,
	phase_b_output: &ColdPhaseBOutput,
	cancel_token: tokio_util::sync::CancellationToken,
	now_ms: i64,
) -> Result<ColdPhaseCOutput> {
	ensure_not_cancelled(&cancel_token)?;

	let branch_id = plan.branch_id;
	let expected_cold_drained_txid = plan.state_before.cold_drained_txid;
	let new_cold_drained_txid = plan.materialized_txid;
	let uploaded_pins = phase_b_output.uploaded_pins.clone();

	db.run(move |tx| {
		let cancel_token = cancel_token.clone();
		let uploaded_pins = uploaded_pins.clone();

		async move {
			ensure_not_cancelled(&cancel_token)?;

			let current_state = read_cold_state(&tx, branch_id).await?;
			if current_state.cold_drained_txid != expected_cold_drained_txid {
				bail!(
					"sqlite cold phase C cold_drained_txid fence changed from {} to {}",
					expected_cold_drained_txid,
					current_state.cold_drained_txid
				);
			}

			let next_state = ColdCompactState {
				cold_drained_txid: new_cold_drained_txid,
				in_flight_uuid: None,
			};
			tx.informal().set(
				&keys::branch_meta_cold_compact_key(branch_id),
				&encode_cold_compact_state(next_state)?,
			);
			tx.informal().set(
				&keys::branch_manifest_cold_drained_txid_key(branch_id),
				&new_cold_drained_txid.to_be_bytes(),
			);

			let mut ready_pins = 0;
			for pin in uploaded_pins {
				if mark_pin_ready(&tx, pin, now_ms).await? {
					ready_pins += 1;
				}
			}

			Ok(ColdPhaseCOutput {
				cold_drained_txid: new_cold_drained_txid,
				ready_pins,
			})
		}
	})
	.await
}

async fn read_cold_state(
	tx: &universaldb::Transaction,
	branch_id: crate::pump::types::ActorBranchId,
) -> Result<ColdCompactState> {
	let Some(bytes) = tx
		.informal()
		.get(&keys::branch_meta_cold_compact_key(branch_id), Serializable)
		.await?
	else {
		return Ok(ColdCompactState {
			cold_drained_txid: 0,
			in_flight_uuid: None,
		});
	};

	decode_cold_compact_state(&bytes)
}

async fn mark_pin_ready(
	tx: &universaldb::Transaction,
	pin: ColdUploadedPin,
	now_ms: i64,
) -> Result<bool> {
	let pinned_key = keys::bookmark_pinned_key(&pin.actor_id, pin.bookmark.as_str());
	let Some(bytes) = tx.informal().get(&pinned_key, Serializable).await? else {
		return Ok(false);
	};
	let mut record = decode_pinned_bookmark_record(&bytes)
		.context("decode sqlite pinned bookmark record during cold phase C")?;

	if record.actor_branch_id != pin.actor_branch_id
		|| record.versionstamp != pin.versionstamp
		|| record.bookmark != pin.bookmark
	{
		return Ok(false);
	}

	record.status = PinStatus::Ready;
	record.pin_object_key = Some(pin.object_key);
	record.updated_at_ms = now_ms;
	tx.informal().set(
		&pinned_key,
		&encode_pinned_bookmark_record(record)
			.context("encode sqlite pinned bookmark record during cold phase C")?,
	);

	Ok(true)
}

pub(crate) async fn mark_payload_pins_failed(
	db: &universaldb::Database,
	payload: &SqliteColdCompactPayload,
	now_ms: i64,
) -> Result<usize> {
	let SqliteColdCompactPayload::CreatePinnedBookmark {
		actor_id,
		actor_branch_id,
		bookmark,
		versionstamp,
	} = payload
	else {
		return Ok(0);
	};
	let actor_id = actor_id.clone();
	let actor_branch_id = *actor_branch_id;
	let bookmark = bookmark.clone();
	let versionstamp = *versionstamp;

	db.run(move |tx| {
		let actor_id = actor_id.clone();
		let bookmark = bookmark.clone();

		async move {
			let pinned_key = keys::bookmark_pinned_key(&actor_id, bookmark.as_str());
			let Some(bytes) = tx.informal().get(&pinned_key, Serializable).await? else {
				return Ok(0);
			};
			let mut record = decode_pinned_bookmark_record(&bytes)
				.context("decode sqlite pinned bookmark record during cold failure handling")?;

			if record.actor_branch_id != actor_branch_id
				|| record.versionstamp != versionstamp
				|| record.bookmark != bookmark
				|| record.status != PinStatus::Pending
			{
				return Ok(0);
			}

			record.status = PinStatus::Failed;
			record.updated_at_ms = now_ms;
			tx.informal().set(
				&pinned_key,
				&encode_pinned_bookmark_record(record)
					.context("encode sqlite pinned bookmark record during cold failure handling")?,
			);

			Ok(1)
		}
	})
	.await
}

fn ensure_not_cancelled(cancel_token: &tokio_util::sync::CancellationToken) -> Result<()> {
	if cancel_token.is_cancelled() {
		bail!("sqlite cold compaction cancelled");
	}

	Ok(())
}
