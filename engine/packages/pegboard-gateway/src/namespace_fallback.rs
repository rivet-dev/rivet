//! Requests the gateway resolves in `default` because they name no namespace and their
//! credential resolves none, which is every credential but a namespace-scoped JWT on a direct
//! actor id; see `routes::request_namespace`. Such a request does not find an actor of another
//! namespace. That is accepted, so the gateway counts every one and logs a warning at most once
//! per [`WARN_INTERVAL_MS`] to point operators at the clients that send them.

use rivet_guard_core::auth;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::metrics::GATEWAY_NAMESPACELESS_REQUESTS_TOTAL;

/// The shortest time between two fallback warnings of one process.
const WARN_INTERVAL_MS: u64 = 60_000;

/// The entry a gateway request arrived on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RouteKind {
	/// A `/gateway/{selector}` path, which names the actor in its selector.
	#[default]
	GatewayPath,
	/// Any other path, with the actor in the `x-rivet-actor` header or a WebSocket's
	/// `rivet_actor.` subprotocol.
	ActorHeaders,
}

impl RouteKind {
	fn label(self) -> &'static str {
		match self {
			RouteKind::GatewayPath => "gateway_path",
			RouteKind::ActorHeaders => "actor_headers",
		}
	}
}

/// The process's record of namespace fallbacks, shared by every gateway request.
#[derive(Debug, Default)]
pub(crate) struct NamespaceFallbacks {
	/// Boot time of the last warning plus one, or 0 before the first.
	last_warned_boot_ms: AtomicU64,
}

impl NamespaceFallbacks {
	/// Counts a request that resolved in `default` because it names no namespace and
	/// `credential` resolves none, and logs a warning unless this process logged one within
	/// [`WARN_INTERVAL_MS`] of `now_boot_ms`. The warning never names the actor or the token.
	pub(crate) fn record(
		&self,
		now_boot_ms: u64,
		credential: &auth::Credential,
		direct: bool,
		route: RouteKind,
	) {
		let credential = credential_kind(credential);
		GATEWAY_NAMESPACELESS_REQUESTS_TOTAL
			.with_label_values(&[credential])
			.inc();

		if !self.claim_warning(now_boot_ms) {
			return;
		}
		tracing::warn!(
			credential,
			direct,
			route = route.label(),
			"a request named no namespace and resolved in default, where an actor of another namespace is not found; send x-rivet-namespace or rvt-namespace (RivetKit 3.0.x clients do not, so upgrade them)"
		);
	}

	/// Whether a warning at `now_boot_ms` is due. Claims it when it is, so of concurrent
	/// requests that find one due, only one warns.
	fn claim_warning(&self, now_boot_ms: u64) -> bool {
		let stored = self.last_warned_boot_ms.load(Ordering::Relaxed);
		let last_warned_boot_ms = stored.checked_sub(1);
		if !warning_due(last_warned_boot_ms, now_boot_ms, WARN_INTERVAL_MS) {
			return false;
		}
		self.last_warned_boot_ms
			.compare_exchange(
				stored,
				now_boot_ms.saturating_add(1),
				Ordering::Relaxed,
				Ordering::Relaxed,
			)
			.is_ok()
	}
}

/// Whether a warning at `now_boot_ms` is due, given the boot time of the last warning, if any,
/// and the shortest time `interval_ms` between two warnings.
fn warning_due(last_warned_boot_ms: Option<u64>, now_boot_ms: u64, interval_ms: u64) -> bool {
	match last_warned_boot_ms {
		None => true,
		Some(last_warned_boot_ms) => now_boot_ms.saturating_sub(last_warned_boot_ms) >= interval_ms,
	}
}

/// The bounded metric and log label of a credential's kind.
fn credential_kind(credential: &auth::Credential) -> &'static str {
	match credential {
		auth::Credential::Admin => "admin",
		auth::Credential::Unauthenticated => "unauthenticated",
		auth::Credential::Jwt(_) => "jwt",
	}
}
