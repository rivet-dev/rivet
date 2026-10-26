macro_rules! define_byte_keys {
	($(($value:literal, $const_name:ident, $str_name:literal)),* $(,)?) => {
		$(
			pub const $const_name: usize = $value;
		)*

		#[deny(unreachable_patterns)]
		pub fn key_from_str(key: &str) -> Option<usize> {
			match key {
				$(
					$str_name => Some($const_name),
				)*
				_ => None,
			}
		}

		#[deny(unreachable_patterns)]
		pub fn str_from_key(key: usize) -> Option<&'static str> {
			match key {
				$(
					$const_name => Some($str_name),
				)*
				_ => None,
			}
		}
	};
}

macro_rules! define_text_keys {
	($(($name:ident, $text:literal)),* $(,)?) => {
		$(pub const $name: &str = $text;)*

		#[deny(unreachable_patterns)]
		pub fn key_from_str(text: &str) -> Option<&'static str> {
			match text {
				$($text => Some($name),)*
				_ => None,
			}
		}
	};
}

// Each address space owns its own fixed components. Add entries only when a key is used.
// Tag segments are always singular nouns (`node`, not `nodes`), even when the segment prefixes a
// collection.
pub mod s3 {
	define_text_keys! {
		(RIVET, "rivet"),
		(DEPOT, "depot"),
		(CLUSTER, "cluster"),
		(COORDINATOR, "coordinator"),
		(CONFIG, "config"),
		(AUTH, "auth"),
		(JWT, "jwt"),
		(KEY_RING, "key-ring"),
		(NODE, "node"),
		(STORAGE, "storage"),
		(DB, "db"),
		(MANIFEST, "manifest"),
		(TABLET, "tablets"),
		(VOLUME, "volumes"),
		(AUTO_PROVISION, "auto-provision"),
	}
}

pub mod ups {
	define_byte_keys! {
		(1, RIVET, "rivet"),
		(2, DEBUG, "debug"),
		(3, TRACING, "tracing"),
		(6, DEPOT, "depot"),
		(7, VOLUME_LOG, "volume-log"),
		(8, VOLUME_COMMAND, "volume-command"),
		(9, COORDINATOR, "coordinator"),
		(10, KV_COMMAND, "kv-command"),
		(11, ACTOR, "actor"),
		(12, ENVOY, "envoy"),
		(13, READY, "ready"),
		(14, STOP_READINESS, "stop-readiness"),
		(15, SUPERSEDE, "supersede"),
		(16, START_NOTICE, "start-notice"),
		(17, DATABASE_PREFETCH, "database-prefetch"),
		(18, JWT_ROTATION, "jwt-rotation"),
	}
}

pub mod slatedb {
	define_byte_keys! {
		(1, VOLUME, "volumes"),
		(2, RECORD, "record"),
		(3, BLOCK, "blocks"),
		(4, DELETED, "deleted"),
		(8, FS_META, "fs-meta"),
		(9, FS_DATA, "fs-data"),
		(10, FS_INODE, "fs-inode"),
		(11, FS_DENTRY, "fs-dentry"),
		(12, FS_LINK, "fs-link"),
		(13, FS_XATTR, "fs-xattr"),
		(14, FS_KV, "fs-kv"),
		(15, FS_ORPHAN, "fs-orphan"),
		(16, FS_CHECKPOINT, "fs-checkpoint"),
		(17, FS_FORKHOLD, "fs-forkhold"),
		(18, FS_PENDING, "fs-pending"),
		(19, FS_FORK_CHECK, "fs-fork-check"),
		(20, FS_MATERIALIZE, "fs-materialize"),
		(21, FS_SWEEP, "fs-sweep"),
		(22, FS_INCARNATION, "fs-incarnation"),
		(23, FS_FORK_PREPARE, "fs-fork-prepare"),
		(24, FS_FORK_REFERENCE, "fs-fork-reference"),
		(25, FS_FORK_RELEASE, "fs-fork-release"),
		(26, FS_FORK_FAILURE, "fs-fork-failure"),
		(27, FS_FORMAT, "fs-format"),
		(28, FS_DELETE, "fs-delete"),
		(29, FS_MAINTAINER_LOG, "fs-maintainer-log"),
		(30, ALARM_TIME, "alarm-time"),
		(32, ALARM_BOOTSTRAP, "alarm-bootstrap"),
		(33, ALARM_CURRENT, "alarm-current"),
		(34, FS_PENDING_IMPORT, "fs-pending-import"),
		(35, FS_RECORD, "fs-record"),
		(36, FS_SPLIT_SNAPSHOT, "fs-split-snapshot"),
		(37, FS_REPAIR_AUDIT, "fs-repair-audit"),
	}
}

pub mod partition {
	// Deleted tags leave gaps; surviving tags keep their numbers so existing keys keep their bytes.
	define_byte_keys! {
		(3, EPOXY_KEY, "epoxy-key"),
		(4, EPOXY_RECORD, "epoxy-record"),
		(5, EPOXY_CONFIG, "epoxy-config"),
		(7, EPOXY_COORDINATOR, "epoxy-coordinator"),
		(8, EPOXY_REPLICA, "epoxy-replica"),
		(10, OPERATOR_MUTATION, "operator-mutation"),
		(11, ACTOR, "actor"),
		(14, DATA, "data"),
		(16, ENVOY, "envoy"),
		(19, KV, "kv"),
		(22, ACTOR_INDEX, "actor-index"),
		(23, ENVOY_INDEX, "envoy-index"),
		(24, ACTOR_KEY, "actor-key"),
		(25, ACTOR_DATABASE, "actor-database"),
		(26, MACHINE_STATE, "machine-state"),
		(27, MACHINE_DATA, "machine-data"),
		(28, ENVOY_POOL_INDEX, "envoy-pool-index"),
		(29, ENVOY_POOL, "envoy-pool"),
		(31, PLACED, "placed"),
		(32, ACTOR_STATE, "actor-state"),
		(33, MAINTAINER_LOG, "maintainer-log"),
		(34, ACTIVE_ENVOY_INDEX, "active-envoy-index"),
		(35, MACHINE_TIMER, "machine-timer"),
		(36, MACHINE_TIMER_BY_ID, "machine-timer-by-id"),
		(37, ENVOY_STATE, "envoy-state"),
		(38, DESTROYED_ACTOR, "destroyed-actor"),
		(39, NAMESPACE_CATALOG, "namespace-catalog"),
		(40, NAMESPACE, "namespace"),
		(41, POOL_CONNECTION, "pool-connection"),
		(42, POOL_HEARTBEAT, "pool-heartbeat"),
	}
}

pub mod depot_command {
	define_byte_keys! {
		(1, KV_GET, "kv-get"),
		(2, KV_APPLY, "kv-apply"),
		(3, EPOXY_CONTROL, "epoxy-control"),
		(4, EPOXY_REPLICA_COMMAND, "epoxy-replica-command"),
		(5, OPERATOR_COMPARE_AND_SET, "operator-compare-and-set"),
		(6, PEGBOARD_INDEX_INSERT_ACTOR, "pegboard.index.insert-actor"),
		(7, PEGBOARD_ENVOY_UPSERT, "pegboard.envoy.upsert"),
		(29, PEGBOARD_ENVOY_READ, "pegboard.envoy.read"),
		(8, PEGBOARD_INDEX_LIST_ACTORS, "pegboard.index.list-actors"),
		(9, PEGBOARD_INDEX_LIST_ENVOYS, "pegboard.index.list-envoys"),
		(10, PEGBOARD_ACTOR_CREATE, "pegboard.actor.create"),
		(11, PEGBOARD_ACTOR_READ, "pegboard.actor.read"),
		(12, PEGBOARD_ACTOR_EVENT, "pegboard.actor.event"),
		(15, PEGBOARD_INDEX_DELETE_ACTOR, "pegboard.index.delete-actor"),
		(17, PEGBOARD_INDEX_DELETE_ENVOY, "pegboard.index.delete-envoy"),
		(18, EPOXY_REPLICA_READ, "epoxy-replica-read"),
		(19, PEGBOARD_INDEX_READ_ENVOY, "pegboard.index.read-envoy"),
		(20, PEGBOARD_INDEX_PUT_ENVOY_POOL, "pegboard.index.put-envoy-pool"),
		(21, PEGBOARD_INDEX_LIST_ENVOY_POOLS, "pegboard.index.list-envoy-pools"),
		(23, PEGBOARD_ACTOR_ALARM, "pegboard.actor.alarm"),
		(25, PEGBOARD_INDEX_INSERT_ENVOY, "pegboard.index.insert-envoy"),
		(26, PEGBOARD_INDEX_UPSERT_ACTIVE_ENVOY, "pegboard.index.upsert-active-envoy"),
		(27, PEGBOARD_INDEX_DELETE_ACTIVE_ENVOY, "pegboard.index.delete-active-envoy"),
		(28, PEGBOARD_INDEX_LIST_ACTIVE_ENVOYS, "pegboard.index.list-active-envoys"),
		(30, PEGBOARD_INDEX_LIST_ACTIVE_ENVOY_STATES, "pegboard.index.list-active-envoy-states"),
		(31, PEGBOARD_INDEX_READ_DESTROYED_ACTOR, "pegboard.index.read-destroyed-actor"),
		(32, PEGBOARD_NAMESPACE_CREATE, "pegboard.namespace.create"),
		(33, PEGBOARD_NAMESPACE_READ, "pegboard.namespace.read"),
		(34, PEGBOARD_NAMESPACE_LIST, "pegboard.namespace.list"),
		(35, PEGBOARD_POOL, "pegboard.pool"),
		(36, PEGBOARD_POOL_LIST, "pegboard.pool-list"),
	}
}
