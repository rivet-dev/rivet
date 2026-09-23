use anyhow::{Context, Result, bail, ensure};
use gas::prelude::*;
use universaldb::prelude::FormalKey;

use crate::{
	SigningKeyRing, decode_signing_key_ring,
	ops::key_ring,
	storage_keys::{self, SigningKeyRingKey},
};

pub struct Input {
	/// Encoded authoritative ring returned by `key_ring::get_latest`. `None` is valid only for
	/// generation-one bootstrap. The encoded generation is the concurrency token.
	pub expected: Option<Vec<u8>>,
	pub successor: SigningKeyRing,
}

impl std::fmt::Debug for Input {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Input")
			.field("expected", &self.expected.as_ref().map(|_| "<redacted>"))
			.field("successor", &self.successor)
			.finish()
	}
}

#[derive(Debug, Clone)]
pub enum Output {
	Committed(Box<key_ring::Snapshot>),
	Conflict,
}

#[operation]
pub async fn auth_jwt_signing_key_ring_compare_and_set(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Output> {
	ensure!(
		ctx.config().is_leader(),
		"JWT signing-ring writes are leader-only"
	);

	match &input.expected {
		Some(expected) => {
			let previous = decode_signing_key_ring(expected)
				.context("invalid expected JWT signing key ring")?;
			previous.validate_successor(&input.successor)?;
		}
		None => {
			ensure!(
				input.successor.generation == 1,
				"only generation one can bootstrap an absent signing key ring"
			);
			input.successor.validate()?;
		}
	}

	let logical_key = SigningKeyRingKey;
	let key = storage_keys::subspace().pack(&logical_key);
	let new_value = logical_key
		.serialize(input.successor.clone())
		.context("failed to encode JWT signing key ring")?;
	match ctx
		.op(epoxy::ops::propose::Input {
			proposal: epoxy::ops::propose::Proposal {
				commands: vec![epoxy::ops::propose::Command {
					kind: epoxy::ops::propose::CommandKind::CheckAndSetCommand(
						epoxy::ops::propose::CheckAndSetCommand {
							key,
							expect_one_of: vec![input.expected.clone()],
							new_value: Some(new_value.clone()),
						},
					),
				}],
			},
			mutable: true,
			purge_cache: true,
			target_replicas: None,
		})
		.await?
	{
		epoxy::ops::propose::ProposalResult::Committed => {
			Ok(Output::Committed(Box::new(key_ring::Snapshot {
				encoded: new_value,
				ring: input.successor.clone(),
			})))
		}
		epoxy::ops::propose::ProposalResult::ConsensusFailed {
			reason: epoxy::ops::propose::ConsensusFailedReason::ExpectedValueDoesNotMatch { .. },
		} => Ok(Output::Conflict),
		epoxy::ops::propose::ProposalResult::ConsensusFailed { reason } => {
			bail!("JWT signing-ring proposal failed: {reason:?}")
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn encoded_signing_ring_never_appears_in_debug_output() {
		let day = 24 * 60 * 60 * 1_000;
		let ring = crate::bootstrap(
			"https://api.rivet.dev".into(),
			1,
			"rivet-api".into(),
			1,
			1,
			&crate::SigningKeyRecord::generate(0),
			0,
			crate::RotationPolicy {
				rotation_interval_ms: 7 * day,
				publish_lead_ms: 10 * 60 * 1_000,
				max_signing_lifetime_ms: 14 * day,
				max_token_ttl_ms: day,
				clock_skew_ms: 30 * 1_000,
			},
		)
		.unwrap()
		.successor;
		let encoded = b"secret-signing-ring-test-sentinel".to_vec();
		for debug in [
			format!(
				"{:?}",
				Input {
					expected: Some(encoded.clone()),
					successor: ring.clone(),
				}
			),
			format!(
				"{:?}",
				key_ring::Snapshot {
					encoded: encoded.clone(),
					ring
				}
			),
		] {
			assert!(!debug.contains(&format!("{encoded:?}")));
			assert!(!debug.contains("secret-signing-ring-test-sentinel"));
			assert!(debug.contains("<redacted>"));
		}
	}
}
