//! Version envelopes for the pool family: a pool's record and an active envoy's state in the
//! pool's volume, and the admin, batched heartbeat and envoy pick commands on that volume.

use crate::generated::pegboard_pool_v1 as wire;

macro_rules! pool_v1 {
	($($name:ident),* $(,)?) => {
		$(
			pub enum $name {
				V1(wire::$name),
			}

			impl_versioned_v1!($name, PEGBOARD_POOL, wire::$name);
		)*
	};
}

pool_v1!(
	PoolRecord,
	PoolAdminRequest,
	PoolAdminResponse,
	PoolEnvoyState,
	PoolHeartbeatsRequest,
	PoolHeartbeatsResponse,
	PoolPickEnvoyRequest,
	PoolPickEnvoyResponse,
	PoolVersionStamp,
);
