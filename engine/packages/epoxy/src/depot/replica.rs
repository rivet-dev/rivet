use crate::commands::ReplicaCommand;
use anyhow::{Result, bail, ensure};
use depot_client::Client;
use epoxy_protocol::protocol;
use std::sync::Arc;

/// The durable state for one regional replica lives in one Depot partition record.
#[derive(Clone)]
pub struct Replica {
	pub(crate) client: Client,
	replica_id: protocol::ReplicaId,
	membership: Arc<protocol::ClusterConfig>,
}

impl Replica {
	pub fn new(client: Client, replica_id: protocol::ReplicaId) -> Self {
		Self {
			client,
			replica_id,
			membership: Arc::new(local_membership(replica_id)),
		}
	}

	/// The replica's membership: this datacenter's replica alone, active and coordinating. It is
	/// fixed for the process, so no request reads Depot for it and nothing refreshes it.
	pub(crate) async fn membership(&self) -> Result<Option<protocol::ClusterConfig>> {
		Ok(Some((*self.membership).clone()))
	}

	pub fn replica_id(&self) -> protocol::ReplicaId {
		self.replica_id
	}

	#[tracing::instrument(skip_all, fields(replica_id = self.replica_id))]
	pub async fn handle(&self, request: protocol::Request) -> Result<protocol::Response> {
		ensure!(
			request.to_replica_id == self.replica_id,
			"Epoxy request addressed to a different replica"
		);
		let kind = match request.kind {
			protocol::RequestKind::PrepareRequest(req) => {
				protocol::ResponseKind::PrepareResponse(self.prepare(&req).await?)
			}
			protocol::RequestKind::AcceptRequest(req) => {
				protocol::ResponseKind::AcceptResponse(self.accept(&req).await?)
			}
			protocol::RequestKind::CommitRequest(req) => {
				protocol::ResponseKind::CommitResponse(self.commit(&req).await?)
			}
			protocol::RequestKind::KvReadStateRequest(req) => {
				protocol::ResponseKind::KvReadStateResponse(self.read_state(&req).await?)
			}
			protocol::RequestKind::KvGetRequest(req) => {
				protocol::ResponseKind::KvGetResponse(protocol::KvGetResponse {
					value: self.get(&req.key).await?,
				})
			}
			protocol::RequestKind::HealthCheckRequest => {
				protocol::ResponseKind::HealthCheckResponse
			}
			protocol::RequestKind::PreAcceptRequest(_) => bail!("PreAccept is local to a proposer"),
			protocol::RequestKind::UpdateConfigRequest(_)
			| protocol::RequestKind::BeginLearningRequest(_)
			| protocol::RequestKind::CoordinatorUpdateReplicaStatusRequest(_) => {
				bail!("Epoxy membership is fixed to the local datacenter")
			}
			protocol::RequestKind::ChangelogReadRequest(_) => {
				bail!("Epoxy changelog and replica catch-up are deferred")
			}
		};
		Ok(protocol::Response { kind })
	}

	#[tracing::instrument(skip_all, fields(replica_id = self.replica_id))]
	pub async fn get(&self, key: &[u8]) -> Result<Option<protocol::CommittedValue>> {
		// A read of a closed key volume uses a maintainer read session, which may lag the
		// committed log. A missing volume holds no value, and a found value is allowed to be
		// stale. An empty answer from an existing volume may hide a just-committed value, so it
		// is confirmed through the writer before it reports no holder.
		let volume = super::keys::partition::key_volume(self.replica_id, key);
		let request = protocol::ReplicaCommand {
			replica_id: self.replica_id,
			key: key.to_vec().into(),
			operation: protocol::ReplicaOperation::GetReplica,
		};
		match self
			.client
			.command::<crate::commands::ReplicaRead>(&volume, request)
			.await
		{
			Ok(protocol::ReplicaCommandResponse::ReplicaValue(response))
				if response.value.is_some() =>
			{
				return Ok(response.value);
			}
			Ok(protocol::ReplicaCommandResponse::ReplicaValue(_)) => {}
			Ok(_) => bail!("unexpected replica response"),
			Err(error) if crate::commands::is_missing(&error) => return Ok(None),
			Err(error) => return Err(error),
		}
		match self
			.command(key, protocol::ReplicaOperation::GetReplica)
			.await?
		{
			protocol::ReplicaCommandResponse::ReplicaValue(response) => Ok(response.value),
			_ => bail!("unexpected replica response"),
		}
	}

	/// Diagnostic state does not take a read promise or require an active voting membership.
	pub(crate) async fn inspect(&self, key: &[u8]) -> Result<protocol::KvReadStateResponse> {
		match self
			.command(key, protocol::ReplicaOperation::InspectReplica)
			.await?
		{
			protocol::ReplicaCommandResponse::ReplicaInspection(response) => Ok(response.state),
			_ => bail!("unexpected replica response"),
		}
	}

	#[tracing::instrument(skip_all, fields(replica_id = self.replica_id))]
	pub async fn prepare(
		&self,
		req: &protocol::PrepareRequest,
	) -> Result<protocol::PrepareResponse> {
		match self
			.command(
				&req.key,
				protocol::ReplicaOperation::PrepareRequest(req.clone()),
			)
			.await?
		{
			protocol::ReplicaCommandResponse::PrepareResponse(response) => Ok(response),
			_ => bail!("unexpected replica response"),
		}
	}

	#[tracing::instrument(skip_all, fields(replica_id = self.replica_id))]
	pub async fn accept(&self, req: &protocol::AcceptRequest) -> Result<protocol::AcceptResponse> {
		match self
			.command(
				&req.key,
				protocol::ReplicaOperation::AcceptRequest(req.clone()),
			)
			.await?
		{
			protocol::ReplicaCommandResponse::AcceptResponse(response) => Ok(response),
			_ => bail!("unexpected replica response"),
		}
	}

	#[tracing::instrument(skip_all, fields(replica_id = self.replica_id))]
	pub async fn commit(&self, req: &protocol::CommitRequest) -> Result<protocol::CommitResponse> {
		match self
			.command(
				&req.key,
				protocol::ReplicaOperation::CommitRequest(req.clone()),
			)
			.await?
		{
			protocol::ReplicaCommandResponse::CommitResponse(response) => Ok(response),
			_ => bail!("unexpected replica response"),
		}
	}

	#[tracing::instrument(skip_all, fields(replica_id = self.replica_id))]
	pub async fn read_state(
		&self,
		req: &protocol::KvReadStateRequest,
	) -> Result<protocol::KvReadStateResponse> {
		self.check_read_epoch(Some(&self.membership), req.epoch)?;
		let response = self
			.command(
				&req.key,
				protocol::ReplicaOperation::KvReadStateRequest(req.clone()),
			)
			.await?;
		self.check_read_epoch(Some(&self.membership), req.epoch)?;
		match response {
			protocol::ReplicaCommandResponse::KvReadStateResponse(response) => Ok(response),
			_ => bail!("unexpected replica response"),
		}
	}

	fn check_read_epoch(&self, config: Option<&protocol::ClusterConfig>, epoch: u64) -> Result<()> {
		if let Some(config) = config {
			ensure!(config.epoch == epoch, "Epoxy read configuration changed");
			ensure!(
				config
					.replicas
					.iter()
					.any(|r| r.replica_id == self.replica_id
						&& r.status == protocol::ReplicaStatus::Active),
				"Epoxy read replica is inactive"
			);
		} else {
			ensure!(epoch == 0, "Epoxy membership has not been initialized");
		}
		Ok(())
	}

	async fn command(
		&self,
		key: &[u8],
		operation: protocol::ReplicaOperation,
	) -> Result<protocol::ReplicaCommandResponse> {
		use protocol::{ReplicaCommandResponse as Response, ReplicaOperation as Operation};
		let empty_state = || protocol::KvReadStateResponse {
			committed: None,
			accepted: None,
			promised: None,
		};
		let missing = match &operation {
			Operation::GetReplica => Some(Response::ReplicaValue(protocol::ReplicaValue {
				value: None,
			})),
			Operation::InspectReplica => {
				Some(Response::ReplicaInspection(protocol::ReplicaInspection {
					state: empty_state(),
				}))
			}
			Operation::KvReadStateRequest(request) => match request.ballot {
				None => Some(Response::KvReadStateResponse(empty_state())),
				Some(_) => None,
			},
			Operation::PrepareRequest(_)
			| Operation::AcceptRequest(_)
			| Operation::CommitRequest(_) => None,
		};
		let volume = super::keys::partition::key_volume(self.replica_id, key);
		let request = protocol::ReplicaCommand {
			replica_id: self.replica_id,
			key: key.to_vec().into(),
			operation,
		};
		if let Some(missing) = missing {
			crate::commands::read::<ReplicaCommand>(&self.client, &volume, request, missing).await
		} else {
			crate::commands::run::<ReplicaCommand>(&self.client, &volume, request).await
		}
	}
}



/// The membership of a single-datacenter deployment: `replica_id` alone, active, as its own
/// coordinator, at the first configuration epoch. Its peer addresses are empty because no
/// request ever leaves this datacenter.
pub fn local_membership(replica_id: protocol::ReplicaId) -> protocol::ClusterConfig {
	protocol::ClusterConfig {
		coordinator_replica_id: replica_id,
		epoch: LOCAL_EPOCH,
		replicas: vec![protocol::ReplicaConfig {
			replica_id,
			status: protocol::ReplicaStatus::Active,
			api_peer_url: String::new().into(),
			guard_url: String::new().into(),
		}],
	}
}

/// The configuration epoch of the fixed single-datacenter membership.
pub const LOCAL_EPOCH: u64 = 1;
