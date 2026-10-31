use rivet_types::{
	envoy_pools::{EnvoyPool, EnvoyPoolKind},
	keys::namespace::envoy_pool::EnvoyPoolVariant,
};

pub fn envoy_pool_variant(envoy_pool: &EnvoyPool) -> EnvoyPoolVariant {
	match envoy_pool.kind {
		EnvoyPoolKind::Normal { .. } => EnvoyPoolVariant::Normal,
		EnvoyPoolKind::Serverless { .. } => EnvoyPoolVariant::Serverless,
	}
}
