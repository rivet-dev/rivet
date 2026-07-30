use anyhow::Result;
use vbare::OwnedVersionedData;

use super::ToEnvoy;
use crate::generated::{v6, v7};

#[test]
fn v6_request_abort_deserializes_with_unknown_reason() -> Result<()> {
	let payload = serde_bare::to_vec(&v6::ToEnvoy::ToEnvoyTunnelMessage(
		v6::ToEnvoyTunnelMessage {
			message_id: v6::MessageId {
				gateway_id: [1; 4],
				request_id: [7; 4],
				message_index: 1,
			},
			message_kind: v6::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort,
		},
	))?;

	let decoded = ToEnvoy::deserialize(&payload, 6)?;
	let v7::ToEnvoy::ToEnvoyTunnelMessage(msg) = decoded else {
		panic!("expected tunnel message");
	};
	let v7::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(abort) = msg.message_kind else {
		panic!("expected request abort");
	};

	assert_eq!(abort.reason.kind, v7::HttpStreamAbortReasonKind::Unknown);
	assert!(abort.reason.detail.is_none());
	Ok(())
}

#[test]
fn v7_request_abort_serializes_to_v6_void_abort() -> Result<()> {
	let encoded = ToEnvoy::wrap_latest(v7::ToEnvoy::ToEnvoyTunnelMessage(
		v7::ToEnvoyTunnelMessage {
			message_id: v7::MessageId {
				gateway_id: [1; 4],
				request_id: [7; 4],
				message_index: 1,
			},
			message_kind: v7::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(
				v7::ToEnvoyRequestAbort {
					reason: v7::HttpStreamAbortReason {
						kind: v7::HttpStreamAbortReasonKind::ClientDisconnect,
						detail: Some("client closed connection".into()),
					},
				},
			),
		},
	))
	.serialize(6)?;

	let decoded: v6::ToEnvoy = serde_bare::from_slice(&encoded)?;
	let v6::ToEnvoy::ToEnvoyTunnelMessage(msg) = decoded else {
		panic!("expected tunnel message");
	};
	assert!(matches!(
		msg.message_kind,
		v6::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
	));
	Ok(())
}
