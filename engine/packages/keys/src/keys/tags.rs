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
		(19, V2_MIGRATOR, "v2-migrator"),
		(20, REPLICA_READINESS, "replica-readiness"),
		// 40: reserved for the filesystem API.
		// 41: reserved for volume limits.
		(43, NAMESPACE, "namespace"),
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
		// 60: reserved for volume usage rows.
		// 61-62: reserved for claim rows and claim usage rows.
		(63, FS_FORKPIN, "fs-forkpin"),
		(64, FS_KV_RANGE_WHITEOUT, "fs-kv-range-whiteout"),
		(65, FS_DELETED_INCARNATION, "fs-deleted-incarnation"),
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
		(35, MACHINE_TIMER_LEGACY, "machine-timer-legacy"),
		(36, MACHINE_TIMER_BY_ID_LEGACY, "machine-timer-by-id-legacy"),
		(37, ENVOY_STATE, "envoy-state"),
		(38, DESTROYED_ACTOR, "destroyed-actor"),
		(39, NAMESPACE_CATALOG, "namespace-catalog"),
		(40, NAMESPACE, "namespace"),
		(41, POOL_CONNECTION, "pool-connection"),
		(42, POOL_HEARTBEAT, "pool-heartbeat"),
		(43, MACHINE_TIMER, "machine-timer"),
		(44, MACHINE_TIMER_BY_ID, "machine-timer-by-id"),
		(45, V2_MIGRATOR, "v2-migrator"),
		(46, V2_MIGRATOR_PASS, "v2-migrator-pass"),
		(47, V2_MIGRATOR_IN_FLIGHT, "v2-migrator-in-flight"),
		(48, V2_MIGRATOR_FENCE, "v2-migrator-fence"),
		(49, ENVOY_VERSION, "envoy-version"),
		(50, ENVOY_NAME_VERSION, "envoy-name-version"),
		(51, ENVOY_POOL_VERSION, "envoy-pool-version"),
		(52, ACL_TOKEN, "acl-token"),
		(53, ACL_RULE, "acl-rule"),
		(54, ACL_POLICY, "acl-policy"),
		(55, ACL_ROLE, "acl-role"),
		// 100-105: reserved for the filesystem API.
		(106, CLAIM_LOG, "claim-log"),
		// Retired: the namespace group key's tag before that key became `[NAMESPACE, name]`. It
		// was never released; the value stays reserved so no other family reuses it.
		(107, RETIRED_GROUP, "retired-group"),
		(108, GROUPED, "grouped"),
		(109, GROUP_RECORD, "group-record"),
		(110, FORK_CHILD, "fork-child"),
		(111, EVENT, "event"),
		(112, EVENT_SEQ, "event-seq"),
		(113, EVENT_ROW, "event-row"),
		(114, GROUP_VOLUME, "group-volume"),
		(115, FORK_OPERATION, "fork-operation"),
		(116, NAMESPACE_VOLUME, "namespace-volume"),
		(117, NAMESPACE_STATE, "namespace-state"),
		(118, NAMESPACE_INDEX, "namespace-index"),
		// Legacy: an actor name a legacy fallback read found, on a namespace volume. Removed in
		// 3.1 with the backfill.
		(119, LEGACY_BACKFILL_NAME, "legacy-backfill-name"),
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
		(32, LEGACY_PEGBOARD_NAMESPACE_CREATE, "pegboard.namespace.create"),
		(33, LEGACY_PEGBOARD_NAMESPACE_READ, "pegboard.namespace.read"),
		(34, LEGACY_PEGBOARD_NAMESPACE_LIST, "pegboard.namespace.list"),
		(35, PEGBOARD_POOL, "pegboard.pool"),
		(36, PEGBOARD_POOL_LIST, "pegboard.pool-list"),
		(37, PEGBOARD_ACTOR_MIGRATE_FROM_V2_BEGIN, "pegboard.actor.migrate-from-v2-begin"),
		(38, PEGBOARD_V2_MIGRATOR_STATE_WRITE, "pegboard.v2-migrator.state-write"),
		(39, PEGBOARD_V2_MIGRATOR_STATE_LOAD, "pegboard.v2-migrator.state-load"),
		(40, PEGBOARD_V2_MIGRATOR_STATE_READ, "pegboard.v2-migrator.state-read"),
		// 100-106: reserved for the filesystem API.
		(107, GROUP_READ, "depot.group.read"),
		(108, EVENTS_READ, "depot.events.read"),
		(109, GROUP_CREATE, "depot.group.create"),
		(110, GROUP_FORK_DRAW, "depot.group.fork-draw"),
		(111, GROUP_CHILD_CREATE, "depot.group.child-create"),
		(112, GROUP_ARCHIVE, "depot.group.archive"),
		(113, GROUP_CHILD_ARCHIVED, "depot.group.child-archived"),
		(114, EVENT_POST, "depot.events.post"),
		(115, PEGBOARD_NAMESPACE_ARCHIVE, "pegboard.namespace.archive"),
		(116, PEGBOARD_NAMESPACE_STEP, "pegboard.namespace.step"),
		(117, PEGBOARD_NAMESPACE_ALARM, "pegboard.namespace.alarm"),
		(118, PEGBOARD_NAMESPACE_STATE_CREATE, "pegboard.namespace.state-create"),
		(119, PEGBOARD_NAMESPACE_STATE_READ, "pegboard.namespace.state-read"),
		(120, PEGBOARD_NAMESPACE_INDEX_PUT, "pegboard.namespace.index-put"),
		(121, PEGBOARD_NAMESPACE_INDEX_LIST, "pegboard.namespace.index-list"),
		(122, LEGACY_PEGBOARD_NAMESPACE_MOVE, "pegboard.namespace.legacy-move"),
		(123, LEGACY_PEGBOARD_ACTOR_ALARM_READ, "pegboard.actor.legacy-alarm-read"),
		// Legacy: records an actor name a legacy fallback read found, on a namespace volume.
		// Removed in 3.1 with the backfill.
		(124, LEGACY_PEGBOARD_BACKFILL_NAME_RECORD, "pegboard.namespace.legacy-backfill-name-record"),
		// Legacy: pages the actor names recorded on a namespace volume for the backfill. Removed in
		// 3.1 with the backfill.
		(125, LEGACY_PEGBOARD_BACKFILL_NAME_LIST, "pegboard.namespace.legacy-backfill-name-list"),
	}
}
