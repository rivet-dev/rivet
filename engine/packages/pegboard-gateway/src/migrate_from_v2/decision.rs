//! Pure routing decisions for a cluster that is taking over a V2 namespace. The callers read
//! this cluster's actor state and ask V2; these functions only choose where a request goes.

/// What this cluster holds for an actor id, as routing sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum V3Actor {
	/// Any phase other than `MigratingFromV2`, `Destroying` and `Destroyed`, with its data
	/// record written. Served here.
	Active,
	/// The V2 migrator holds it. Nothing here starts or waits for it.
	Held,
	/// A create settled that its key belongs to V2, so V2 serves every request for it.
	Delegated,
	/// No state, no data record yet, or destroyed.
	None,
}

/// Where a request goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
	/// Serve it here as usual. A keyed getOrCreate creates the actor here if it is missing.
	V3,
	/// Proxy the request unchanged to the V2 gateway.
	V2,
	/// Refuse with the retryable `actor.migrating_from_v2`.
	RetryLater,
	/// Answer `actor.not_found`.
	NotFound,
}

/// A request by actor id. A held actor is still V2's until its cutover, so the request goes to
/// V2, which answers `actor.not_found` once the cutover lands.
pub fn by_id(actor: V3Actor) -> Route {
	match actor {
		V3Actor::Active => Route::V3,
		V3Actor::Held | V3Actor::Delegated | V3Actor::None => Route::V2,
	}
}

/// V2 answered `actor.not_found`, and this cluster's state was read again.
///
/// An actor that became active here in the meantime was just activated. The request already
/// sent its body to V2, so it cannot be replayed here; it is refused as retryable and the
/// client's retry lands here.
pub fn after_v2_not_found(actor: V3Actor) -> Route {
	match actor {
		V3Actor::Active | V3Actor::Held => Route::RetryLater,
		V3Actor::Delegated | V3Actor::None => Route::NotFound,
	}
}

/// A keyless create: `draw` is uniform in `[0, 1)`.
pub fn keyless_create(draw: f64, create_ratio: f64) -> Route {
	if draw < create_ratio {
		Route::V3
	} else {
		Route::V2
	}
}
