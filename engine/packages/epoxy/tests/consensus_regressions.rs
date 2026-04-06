mod common;

use anyhow::{Result, bail};
use common::{
	THREE_REPLICAS, TestCtx,
	utils::{read_accepted_value, read_v2_value, set_if_absent, write_ballot},
};
use epoxy::{
	protocol::{
		self, AcceptRequest, AcceptResponse, CommitRequest, CommitResponse, PrepareRequest,
		PrepareResponse, Request, RequestKind, ResponseKind,
	},
};
use epoxy_protocol::PROTOCOL_VERSION;

static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test(flavor = "multi_thread")]
async fn slow_path_recovery_uses_majority_quorum_after_prepare() {
	let _guard = TEST_LOCK.lock().await;
	let mut test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
	let replica_id = test_ctx.leader_id;

	let key = b"slow-path-majority-quorum";
	write_ballot(
		test_ctx.get_ctx(replica_id),
		replica_id,
		key,
		protocol::Ballot {
			counter: 7,
			replica_id: THREE_REPLICAS[1],
		},
	)
	.await
	.unwrap();

	test_ctx.stop_replica(THREE_REPLICAS[2], false).await.unwrap();

	let ctx = test_ctx.get_ctx(replica_id);
	let result = set_if_absent(ctx, key, b"committed").await.unwrap();
	assert!(matches!(result, epoxy::ops::propose::ProposalResult::Committed));
	assert_eq!(
		read_v2_value(ctx, replica_id, key).await.unwrap(),
		Some(b"committed".to_vec()),
	);

	let _ = test_ctx.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn equal_ballot_accepts_do_not_overwrite_accepted_state() {
	let _guard = TEST_LOCK.lock().await;
	let mut test_ctx = TestCtx::new_replica_only_with(&[1]).await.unwrap();
	let replica_id = test_ctx.leader_id;
	let ctx = test_ctx.get_ctx(replica_id);
	let key = b"equal-ballot-overwrite";
	let ballot = protocol::Ballot {
		counter: 1,
		replica_id,
	};

	let first_response = send_accept(
		test_ctx.api_peer_url(replica_id),
		replica_id,
		replica_id,
		key,
		b"value-1",
		ballot.clone(),
	)
		.await
		.unwrap();
	assert!(matches!(
		first_response,
		AcceptResponse::AcceptResponseOk(_)
	));

	let first_accepted = read_accepted_value(ctx, replica_id, key)
		.await
		.unwrap()
		.expect("accepted value should exist after first accept");
	assert_eq!(first_accepted.value, b"value-1".to_vec());
	assert_eq!(first_accepted.ballot, ballot);

	let second_response = send_accept(
		test_ctx.api_peer_url(replica_id),
		replica_id,
		replica_id,
		key,
		b"value-2",
		ballot.clone(),
	)
		.await
		.unwrap();
	assert!(matches!(
		second_response,
		AcceptResponse::AcceptResponseHigherBallot(_)
	));

	let accepted = read_accepted_value(ctx, replica_id, key)
		.await
		.unwrap()
		.expect("accepted value should still exist after second accept");
	assert_eq!(accepted.value, b"value-1".to_vec());
	assert_eq!(accepted.ballot, ballot);

	let _ = test_ctx.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn commit_succeeds_after_accept_quorum_even_if_local_ballot_was_preempted() {
	let _guard = TEST_LOCK.lock().await;
	let mut test_ctx = TestCtx::new_with(THREE_REPLICAS).await.unwrap();
	let replica_id = test_ctx.leader_id;
	let key = b"chosen-value-can-still-be-learned";
	let ballot = protocol::Ballot {
		counter: 1,
		replica_id,
	};
	let higher_ballot = protocol::Ballot {
		counter: 2,
		replica_id: THREE_REPLICAS[2],
	};

	for &acceptor_id in &THREE_REPLICAS[..2] {
		let accept_response = send_accept(
			test_ctx.api_peer_url(acceptor_id),
			replica_id,
			acceptor_id,
			key,
			b"value-1",
			ballot.clone(),
		)
		.await
		.unwrap();
		assert!(matches!(
			accept_response,
			AcceptResponse::AcceptResponseOk(_)
		));
	}

	let prepare_response = send_prepare(
		test_ctx.api_peer_url(replica_id),
		THREE_REPLICAS[2],
		replica_id,
		key,
		higher_ballot,
	)
	.await
	.unwrap();
	assert!(matches!(
		prepare_response,
		PrepareResponse::PrepareResponseOk(_)
	));

	let commit_response = send_commit(
		test_ctx.api_peer_url(replica_id),
		replica_id,
		key,
		b"value-1",
		ballot,
	)
	.await
	.unwrap();
	assert!(matches!(commit_response, CommitResponse::CommitResponseOk));
	assert_eq!(
		read_v2_value(test_ctx.get_ctx(replica_id), replica_id, key)
			.await
			.unwrap(),
		Some(b"value-1".to_vec()),
	);

	let _ = test_ctx.shutdown().await;
}

async fn send_accept(
	replica_url: String,
	from_replica_id: protocol::ReplicaId,
	to_replica_id: protocol::ReplicaId,
	key: &[u8],
	value: &[u8],
	ballot: protocol::Ballot,
) -> Result<AcceptResponse> {
	let response = send_request(
		replica_url,
		Request {
			from_replica_id,
			to_replica_id,
			kind: RequestKind::AcceptRequest(AcceptRequest {
				key: key.to_vec(),
				value: value.to_vec(),
				ballot,
				mutable: true,
				version: 1,
			}),
		},
	)
	.await?;

	match response.kind {
		ResponseKind::AcceptResponse(response) => Ok(response),
		_ => bail!("unexpected response type for accept request"),
	}
}

async fn send_prepare(
	replica_url: String,
	from_replica_id: protocol::ReplicaId,
	to_replica_id: protocol::ReplicaId,
	key: &[u8],
	ballot: protocol::Ballot,
) -> Result<PrepareResponse> {
	let response = send_request(
		replica_url,
		Request {
			from_replica_id,
			to_replica_id,
			kind: RequestKind::PrepareRequest(PrepareRequest {
				key: key.to_vec(),
				ballot,
				mutable: true,
				version: 1,
			}),
		},
	)
	.await?;

	match response.kind {
		ResponseKind::PrepareResponse(response) => Ok(response),
		_ => bail!("unexpected response type for prepare request"),
	}
}

async fn send_commit(
	replica_url: String,
	from_replica_id: protocol::ReplicaId,
	key: &[u8],
	value: &[u8],
	ballot: protocol::Ballot,
) -> Result<CommitResponse> {
	let response = send_request(
		replica_url,
		Request {
			from_replica_id,
			to_replica_id: from_replica_id,
			kind: RequestKind::CommitRequest(CommitRequest {
				key: key.to_vec(),
				value: value.to_vec(),
				ballot,
				mutable: true,
				version: 1,
			}),
		},
	)
	.await?;

	match response.kind {
		ResponseKind::CommitResponse(response) => Ok(response),
		_ => bail!("unexpected response type for commit request"),
	}
}

async fn send_request(replica_url: String, request: Request) -> Result<protocol::Response> {
	let mut url = replica_url.trim_end_matches('/').to_string();
	url.push_str(&format!("/v{PROTOCOL_VERSION}/epoxy/message"));

	let request = serde_bare::to_vec(&request)?;

	let response = rivet_pools::reqwest::client()
		.await?
		.post(url)
		.body(request)
		.send()
		.await?;
	if !response.status().is_success() {
		bail!("request failed with status {}", response.status());
	}
	let body = response.bytes().await?;
	let response: protocol::Response = serde_bare::from_slice(&body)?;

	Ok(response)
}
