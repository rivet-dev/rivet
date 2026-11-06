//! Logical Epoxy keys owned by pegboard. Epoxy stores each one in its own key volume.

// The reservation of a keyed actor's name and key within its namespace. It carries the key's
// placement, so Epoxy places its key volume with the key's actor.
rivet_keys::volume_kv_key! {
	pub ActorKeyKey(namespace: String, name: String, key: String) placed_by(reservation_placement) => [ACTOR_KEY, {namespace}, {name}, {key}];
	value: vbare_selected(rivet_data::versioned::EpoxyActorByKeyEntry);
}

fn reservation_placement(key: &ActorKeyKey) -> rivet_keys::keys::codec::Placement {
	crate::keys::partition::key_placement(&key.namespace, &key.name, &key.key)
}
