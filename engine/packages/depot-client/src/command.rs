//! Typed commands run on the volume's maintainer within one deadline. A refusal returns to the
//! caller at once as [`Overloaded`] with the storage host's retry hint, and the request's edge
//! decides whether to retry. A command that ran but could not finish returns as [`Unavailable`],
//! and a fence or corruption as a final error. A snapshot read interrupted by transport returns
//! [`ReadInterrupted`] for the edge to retry within its budget. A route
//! change is followed instead, after the command manifest refreshes, with the same operation id:
//! a stale maintainer, or authority that moved to a new grant possibly after the command
//! committed.
use crate::{Client, metrics};
use anyhow::{Result, ensure};
use depot_protocol::command::{Command, Request, Response};
use rivet_data::{
	DEPOT_KV_COMMAND_VERSION,
	generated::depot_kv_command_v1::{
		KvCommandRequest, KvCommandResponse, OverloadReason, UnavailableReason,
	},
};
use std::time::Duration;
use vbare::Versioned;

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"depot",
	"request_deadline_exceeded",
	"The command deadline elapsed; its outcome may be unknown."
)]
pub(crate) struct RequestDeadlineExceeded;

/// The storage host did not admit the command. Nothing ran, so the caller may send it again after
/// `retry_after_ms`.
#[derive(Debug, serde::Serialize, serde::Deserialize, rivet_error::RivetError)]
#[error(
	"depot",
	"overloaded",
	"The storage host refused the command; retry after the hinted delay.",
	"The storage host refused the command ({reason}); retry after {retry_after_ms} ms."
)]
pub struct Overloaded {
	pub reason: String,
	pub retry_after_ms: u64,
}

/// A transport interruption lost the result of a snapshot-only read. The read may have
/// executed, but it has no mutating or external effects, so the edge may retry it within
/// its budget. This classification never applies to an uncertain mutation.
#[derive(Debug, serde::Serialize, serde::Deserialize, rivet_error::RivetError)]
#[error(
	"depot",
	"read_interrupted",
	"The storage read was interrupted; retry after the hinted delay.",
	"The storage read was interrupted; retry after {retry_after_ms} ms."
)]
pub struct ReadInterrupted {
	pub retry_after_ms: u64,
}

/// The storage host admitted the command but could not finish it. Its outcome may be unknown,
/// so a retry with a new operation id could apply it twice.
#[derive(Debug, serde::Serialize, serde::Deserialize, rivet_error::RivetError)]
#[error(
	"depot",
	"unavailable",
	"The storage host could not finish the command; its outcome may be unknown.",
	"The storage host could not finish the command ({reason}); its outcome may be unknown."
)]
pub struct Unavailable {
	pub reason: String,
}

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"depot",
	"fenced",
	"A newer owner holds the volume; the command was not applied by this owner."
)]
pub struct Fenced;

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"depot",
	"corrupt",
	"The volume's stored data failed a consistency check."
)]
pub struct Corrupt;

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"depot",
	"conflict",
	"The command conflicted with a concurrent change to the volume."
)]
pub struct Conflict;

/// Retry hint for a command refused before it was sent. The storage host would otherwise give its
/// own hint.
const TRANSPORT_RETRY_AFTER_MS: u64 = 100;

/// The [`Overloaded`] reason of a command refused because its maintainer's endpoint could not
/// be resolved. Nothing was sent; the edge retries it on a fresh route.
pub const ROUTE_UNRESOLVED_REASON: &str = "endpoint_unresolved";

/// A cached manifest has no maintainer route, so the command has not been sent.
const ROUTING_UNAVAILABLE_REASON: &str = "routing_unavailable";

/// UPS failures that name a stale route, or that may follow a command the maintainer ran.
const ROUTE_TRANSPORT_CODES: &[&str] = &["wrong_worker", "unknown_worker", "consumer_closed"];

#[derive(Debug, rivet_error::RivetError)]
#[error("depot", "command_too_large", "The command exceeds the byte limit.")]
pub(crate) struct CommandTooLarge;

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"depot",
	"unknown_command",
	"The maintainer does not recognize this command."
)]
pub(crate) struct UnknownCommand;

#[derive(Debug, rivet_error::RivetError)]
#[error("depot", "volume_not_found", "The KV database volume does not exist.")]
pub(crate) struct VolumeNotFound;

#[derive(Debug, rivet_error::RivetError)]
#[error("depot", "volume_deleted", "The KV database volume was deleted.")]
pub(crate) struct VolumeDeleted;

/// Per-call options for [`Client::command_with`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandOptions {
	/// Lets the storage host create a missing volume inside its first open, so the command
	/// runs on the new volume within the same attempt.
	pub create_if_missing: bool,
}

impl Client {
	/// Runs an idempotent command on the volume's maintainer; callers never open or claim the
	/// database volume. A fresh refusal returns as [`Overloaded`] without a retry. A command whose
	/// type sets `CREATE_IF_MISSING` creates a missing volume.
	pub async fn command<C: Command>(
		&self,
		volume_key: &depot_types::VolumeKey,
		request: Request<C>,
	) -> Result<Response<C>> {
		self.command_with::<C>(
			volume_key,
			request,
			CommandOptions {
				create_if_missing: C::CREATE_IF_MISSING,
			},
		)
		.await
	}

	/// Like [`Client::command`], with per-call options. One command type can serve both reads
	/// that must not allocate a volume and writes that create it.
	pub async fn command_with<C: Command>(
		&self,
		volume_key: &depot_types::VolumeKey,
		request: Request<C>,
		options: CommandOptions,
	) -> Result<Response<C>> {
		ensure!(
			!C::READ_ONLY
				|| !(options.create_if_missing || C::CREATE_IF_MISSING || C::SPECULATIVE_CREATE),
			"a read-only command cannot create its volume"
		);
		let timeout = self.command_timeout(C::TIMEOUT);
		ensure!(!timeout.is_zero(), "command timeout must be positive");
		let clock = self.env().clock();
		let deadline = rivet_env::deadline_after(clock.as_ref(), timeout)?;
		// A command body carries its own type's version, not the command envelope's.
		let body = C::Request::seal_to_bytes(&request, C::Request::LATEST)?;
		let request = KvCommandRequest {
			volume_key: volume_key.clone(),
			operation_id: *self
				.env()
				.rng("depot.command.operation")
				.uuid_v4()
				.as_bytes(),
			tag: C::TAG.0.try_into()?,
			body,
			create_if_missing: options.create_if_missing,
		};
		let lifecycle = self.volume_handle()?.lifecycle;
		let lifecycle = if let Some(guard) = &self.dispatch_guard {
			lifecycle.with_dispatch_guard(guard.clone())
		} else {
			lifecycle
		};
		// Seal once: the size check and every attempt use these bytes.
		let encoded =
			rivet_data::versioned::DepotKvCommandRequestData::serialize_with_embedded_version(
				&request,
				DEPOT_KV_COMMAND_VERSION,
			)?;
		if let Some(max_bytes) = lifecycle.max_command_bytes
			&& encoded.len() > max_bytes
		{
			return Err(CommandTooLarge.build());
		}
		let command =
			rivet_keys::keys::tags::depot_command::str_from_key(C::TAG.0).unwrap_or("unknown");
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let started = std::time::Instant::now();
		let execute = async {
			// A later refusal cannot prove that an earlier owner did not commit.
			let mut outcome_uncertain = false;
			loop {
				if !self.dispatch_guard.as_ref().is_none_or(|guard| guard()) {
					return Err(crate::ScopeRevoked.build());
				}
				let response = match lifecycle
					.command_at(&request.volume_key, &encoded, deadline)
					.await
				{
					Ok(response) => response,
					Err(error)
						if error
							.downcast_ref::<rivet_error::RivetError>()
							.is_some_and(|error| error.code() == "invalid_message") =>
					{
						return Err(error);
					}
					Err(error)
						if error.downcast_ref::<rivet_error::RivetError>().is_some_and(
							|error| {
								error.group() == "ups"
									&& ROUTE_TRANSPORT_CODES.contains(&error.code())
							},
						) =>
					{
						metrics::COMMAND_REJECTIONS
							.with_label_values(&[command, "route_transport"])
							.inc();
						outcome_uncertain = true;
						lifecycle.refresh_command_manifest().await;
						continue;
					}
					Err(error) => {
						// The attempt ran out of time at the command's deadline: it found no
						// time left to send, or its request timed out. The deadline on the whole
						// command reports the same, but its sleep is ready only once the clock's
						// timer fires, a tick after a reading first shows the deadline reached,
						// so the attempt's error comes first. A request that UPS timed out under
						// its own cap, with command time left, is a transport failure.
						if timed_out(&error) && clock.observe()?.boot_ms >= deadline {
							return Err(RequestDeadlineExceeded.build());
						}
						let structured = error.downcast_ref::<rivet_error::RivetError>();
						if structured.is_some_and(|error| {
							error.group() == "depot" && error.code() == ROUTING_UNAVAILABLE_REASON
						}) {
							metrics::COMMAND_REJECTIONS
								.with_label_values(&[command, ROUTING_UNAVAILABLE_REASON])
								.inc();
							lifecycle.refresh_manifest();
							return Err(refused::<C>(
								ROUTING_UNAVAILABLE_REASON,
								TRANSPORT_RETRY_AFTER_MS,
								outcome_uncertain,
							));
						}
						let ups_code = structured
							.filter(|error| error.group() == "ups")
							.map(|error| error.code());

						// An unresolved maintainer endpoint refuses the command before it is sent,
						// as after the maintainer died. The manifest refreshes now so the edge's
						// retry follows the route to the tablet's next maintainer.
						if ups_code == Some("endpoint_unresolved") {
							metrics::COMMAND_REJECTIONS
								.with_label_values(&[command, ROUTE_UNRESOLVED_REASON])
								.inc();
							// After an uncertain attempt the command cannot be refused for the edge
							// to send as a new one, so it follows the tablet to its next maintainer
							// as the same logical operation, within its own deadline, as it does
							// after a transport miss.
							if outcome_uncertain {
								lifecycle.refresh_command_manifest().await;
								rivet_env::sleep(
									clock.as_ref(),
									Duration::from_millis(TRANSPORT_RETRY_AFTER_MS),
								)
								.await?;
								continue;
							}
							lifecycle.refresh_manifest();
							return Err(refused::<C>(
								ROUTE_UNRESOLVED_REASON,
								TRANSPORT_RETRY_AFTER_MS,
								outcome_uncertain,
							));
						}

						// A full UPS window refuses the command before it is sent, so nothing ran.
						if ups_code == Some("overloaded") {
							metrics::COMMAND_REJECTIONS
								.with_label_values(&[command, "transport_window"])
								.inc();
							return Err(refused::<C>(
								overload_reason_label(OverloadReason::TransportWindow),
								TRANSPORT_RETRY_AFTER_MS,
								outcome_uncertain,
							));
						}
						metrics::COMMAND_REJECTIONS
							.with_label_values(&[command, "transport"])
							.inc();
						tracing::debug!(?error, command, "depot command transport failed");
						if C::READ_ONLY && ups_code == Some("transport") {
							lifecycle.refresh_manifest();
							return Err(ReadInterrupted {
								retry_after_ms: TRANSPORT_RETRY_AFTER_MS,
							}
							.build());
						}
						return Err(Unavailable {
							reason: unavailable_reason_label(UnavailableReason::Transport)
								.to_owned(),
						}
						.build());
					}
				};
				match response {
					KvCommandResponse::CommandResult(response) => {
						return Ok(C::Response::decode(response.body)?);
					}
					// A stale route is not a refusal: the refreshed manifest names the maintainer.
					KvCommandResponse::NotMaintainer => {
						metrics::COMMAND_REJECTIONS
							.with_label_values(&[command, "not_maintainer"])
							.inc();
						lifecycle.refresh_command_manifest().await;
					}
					// The command follows its volume's authority to the new maintainer, after the
					// handoff's hint, as the same logical operation. A fenced host means a newer
					// owner took the volume, so the command follows the route to it the same way.
					KvCommandResponse::AuthorityMoved(_) | KvCommandResponse::Fenced => {
						let (label, wait_ms) = match &response {
							KvCommandResponse::AuthorityMoved(moved) => {
								("authority_moved", moved.retry_after_ms)
							}
							_ => ("fenced", TRANSPORT_RETRY_AFTER_MS),
						};
						metrics::COMMAND_REJECTIONS
							.with_label_values(&[command, label])
							.inc();
						outcome_uncertain = true;
						lifecycle.refresh_command_manifest().await;
						rivet_env::sleep(clock.as_ref(), Duration::from_millis(wait_ms)).await?;
					}
					KvCommandResponse::Overloaded(overloaded) => {
						let reason = overload_reason_label(overloaded.reason);
						metrics::COMMAND_REJECTIONS
							.with_label_values(&[command, reason])
							.inc();
						return Err(refused::<C>(
							reason,
							overloaded.retry_after_ms,
							outcome_uncertain,
						));
					}
					KvCommandResponse::Unavailable(unavailable) => {
						let reason = unavailable_reason_label(unavailable.reason);
						metrics::COMMAND_REJECTIONS
							.with_label_values(&[command, reason])
							.inc();
						return Err(Unavailable {
							reason: reason.to_owned(),
						}
						.build());
					}
					KvCommandResponse::Corrupt => return Err(Corrupt.build()),
					KvCommandResponse::Conflict => return Err(Conflict.build()),
					KvCommandResponse::UnknownCommand => return Err(UnknownCommand.build()),
					KvCommandResponse::CommandTooLarge => return Err(CommandTooLarge.build()),
					KvCommandResponse::VolumeNotFound => return Err(VolumeNotFound.build()),
					KvCommandResponse::VolumeDeleted => return Err(VolumeDeleted.build()),
					KvCommandResponse::ApplicationError(error) => {
						return Err(rivet_error::RivetError {
							kind: rivet_error::RivetErrorKind::Dynamic {
								group: error.group.to_string(),
								code: error.code.to_string(),
								default_message: error.message.to_string(),
							},
							meta: None,
							message: None,
							actor: None,
						}
						.into());
					}
				}
			}
		};
		let result = rivet_env::timeout_at(clock.as_ref(), deadline, execute)
			.await
			.map_err(|_| RequestDeadlineExceeded.build())
			.and_then(|result| result);
		let outcome = match &result {
			Ok(_) => "ok",
			Err(error)
				if error
					.downcast_ref::<rivet_error::RivetError>()
					.is_some_and(|error| error.code() == "request_deadline_exceeded") =>
			{
				"deadline"
			}
			Err(error)
				if error
					.downcast_ref::<rivet_error::RivetError>()
					.is_some_and(|error| {
						error.group() == "depot" && error.code() == "read_interrupted"
					}) =>
			{
				"read_interrupted"
			}
			Err(error) if retry_after_ms(error).is_some() => "overloaded",
			Err(_) => "error",
		};
		metrics::COMMAND_DURATION
			.with_label_values(&[command, outcome])
			.observe(started.elapsed().as_secs_f64());
		result
	}
}

/// Whether an attempt's error says its time ran out: the attempt found no time left to send,
/// or its request timed out. It is the command's deadline only when the clock agrees, because
/// UPS caps a request's wait at its own request timeout.
fn timed_out(error: &anyhow::Error) -> bool {
	error
		.downcast_ref::<rivet_error::RivetError>()
		.is_some_and(|error| {
			(error.group() == "depot" && error.code() == "request_deadline_exceeded")
				|| (error.group() == "ups" && error.code() == "timeout")
		})
}

/// A later refusal does not erase a possibly applied mutation. A pure read may
/// select a new committed snapshot at the edge because no attempt changes state.
fn refused<C: Command>(
	reason: &str,
	retry_after_ms: u64,
	outcome_uncertain: bool,
) -> anyhow::Error {
	if outcome_uncertain && !C::READ_ONLY {
		Unavailable {
			reason: reason.to_owned(),
		}
		.build()
	} else {
		Overloaded {
			reason: reason.to_owned(),
			retry_after_ms,
		}
		.build()
	}
}

/// The retry hint of a command the storage host did not admit, of a volume open or write a
/// maintainer, shard quorum or volume client refused at admission, of a refused checkpoint or
/// fork, or of a snapshot read interrupted by transport. The edge may retry these within its
/// budget. A refused fork is the exception to "nothing ran": it may only be retried by sending
/// its retained body again. Every other error is
/// final, including an [`Unavailable`] whose mutation outcome may be unknown.
pub fn retry_after_ms(error: &anyhow::Error) -> Option<u64> {
	use depot_volume::volume_client_machine::event::RpcError;

	match error.downcast_ref::<crate::volumes::Error>() {
		Some(crate::volumes::Error::Write(error)) => return error.retry_after_ms(),
		Some(crate::volumes::Error::Filesystem(error)) => return error.retry_after_ms(),
		Some(crate::volumes::Error::Lifecycle(RpcError::LifecycleRejected(reject))) => {
			return depot_types::client::retry_after_ms(reject);
		}
		Some(crate::volumes::Error::Lifecycle(RpcError::Overloaded { retry_after_ms })) => {
			return Some(*retry_after_ms);
		}
		Some(crate::volumes::Error::Checkpoint(error)) => return error.retry_after_ms(),
		Some(_) | None => {}
	}
	if let Some(error) = error.downcast_ref::<crate::filesystem::Error>() {
		return error.retry_after_ms();
	}
	if let Some(error) = error.downcast_ref::<crate::checkpoints::Error>() {
		return error.retry_after_ms();
	}
	if let Some(error) = error.downcast_ref::<crate::forks::Error>() {
		return error.retry_after_ms();
	}
	let error = error.downcast_ref::<rivet_error::RivetError>()?;
	if error.group() != "depot" || !matches!(error.code(), "overloaded" | "read_interrupted") {
		return None;
	}
	let meta = error.metadata()?;
	meta.get("retry_after_ms")?.as_u64()
}

/// Whether `error` is an [`Overloaded`] refusal because the cached manifest has no maintainer
/// route or its endpoint could not be resolved. Its retry routes afresh rather than waiting
/// out shed load.
pub fn route_unresolved(error: &anyhow::Error) -> bool {
	let Some(error) = error.downcast_ref::<rivet_error::RivetError>() else {
		return false;
	};
	if error.group() != "depot" || error.code() != "overloaded" {
		return false;
	}
	let reason = error
		.metadata()
		.and_then(|meta| meta.get("reason")?.as_str().map(ToOwned::to_owned));
	matches!(
		reason.as_deref(),
		Some(ROUTE_UNRESOLVED_REASON | ROUTING_UNAVAILABLE_REASON)
	)
}

/// The bounded reason label of an [`OverloadReason`], also the `reason` of [`Overloaded`].
pub fn overload_reason_label(reason: OverloadReason) -> &'static str {
	match reason {
		OverloadReason::CommandCapacity => "command_capacity",
		OverloadReason::CommandBytes => "command_bytes",
		OverloadReason::VolumesBusy => "volumes_busy",
		OverloadReason::VolumeTransition => "volume_transition",
		OverloadReason::FilesystemOverloaded => "filesystem_overloaded",
		OverloadReason::MaintainerInbox => "maintainer_inbox",
		OverloadReason::TransportWindow => "transport_window",
	}
}

/// The bounded reason label of an [`UnavailableReason`], also the `reason` of [`Unavailable`].
pub fn unavailable_reason_label(reason: UnavailableReason) -> &'static str {
	match reason {
		UnavailableReason::TimedOut => "timed_out",
		UnavailableReason::OpenFailed => "open_failed",
		UnavailableReason::StorageFailed => "storage_failed",
		UnavailableReason::Internal => "internal",
		UnavailableReason::Transport => "transport",
	}
}


