# Epoxy

Epoxy is a geo-distributed, strongly consistent KV store. Keys are immutable by default, with opt-in mutable overwrite semantics for workloads that need them. The consensus protocol is single-decree Fast Paxos (Lamport, 2006), run independently per key. Fresh keys commit in 1 RTT by skipping Phase 1 (Prepare) and going straight to Phase 2 (Accept). Keys with prior in-flight state fall back to classic two-phase Paxos (Prepare + Accept) at 2 RTT.

### Why immutable keys

Immutability is not a limitation. It is the design choice that makes everything else cheap:

- **No read quorums.** A committed value can never change, so any replica that has it can serve reads locally. Reads never touch the network in steady state.
- **Aggressive caching.** Replicas can cache committed values from other datacenters indefinitely. Cache invalidation is trivial because there is nothing to invalidate.
- **Idempotent replication.** Commit messages and changelog catch-up can deliver the same key multiple times safely. The second write is always a no-op.
- **No conflict resolution.** There is no merge logic or last-writer-wins. Either your value won or someone else got there first.
- **Unordered changelogs.** Each replica's changelog can have entries in a different order. This is safe because the final state is the same set of committed values regardless of application order.

Mutable keys trade some of these properties for overwrite capability. Their optimistic cache must be invalidated on commit, changelog catch-up becomes version-aware, and duplicate committed values are only idempotent when the version matches.

### Why single-decree per key

Each key only needs agreement on one value, so each key gets its own independent single-decree Paxos instance. This gives:

- **No shared log.** Keys are independent. There is no global ordering to maintain across unrelated keys.
- **No leader election.** Any replica can propose for any key at any time. There is no distinguished leader to elect or fail over.
- **Lazy crash recovery.** A stalled proposal only affects one key. There is no failure detector or background scan. Recovery happens only when another proposer tries to write the same key.

### Terminology

Epoxy now uses standard Fast Paxos terminology. Earlier versions of this codebase inherited the name `PreAccept` from its EPaxos ancestry, but that phase was always standard Paxos Phase 2a.

| Epoxy          | Fast Paxos     | Purpose                                         |
|----------------|----------------|-------------------------------------------------|
| Prepare        | Phase 1a       | Leader asks replicas to promise a ballot         |
| Prepare Ok     | Phase 1b       | Replicas promise and report any accepted value   |
| Accept         | Phase 2a       | Leader asks replicas to accept a value at ballot |
| Accept Ok      | Phase 2b       | Replicas confirm they accepted                   |
| Commit         | Learn/Decide   | Leader tells replicas the value is final         |

Rivet uses Epoxy for two workflows:

- `pegboard::workflows::actor::actor_keys::Propose` reserves an actor key with a single-key set-if-absent write.
- `pegboard::ops::get_reservation_for_key` resolves that reservation with an optimistic read.

Those workloads do not need cross-key ordering or shared global proposal metadata, so Epoxy keeps consensus state per key. For workloads that do need cross-key ordering in high-latency geo-distributed environments, EPaxos (Moraru et al., 2013) is a useful reference.

## Model

Each key is handled independently by single-decree Paxos.

- A proposal leader first runs ballot selection by reading `kv/{key}/value` and `kv/{key}/ballot` atomically.
- Fresh keys skip Prepare and go straight to `Accept`, which gives the common 1 RTT write path.
- Keys with prior in-flight state run `Prepare` first, adopt the highest accepted value if one exists, then continue with `Accept`.
- Successful proposals commit locally in one UDB transaction and then broadcast `Commit` to every replica.
- Immutable values keep the 1 RTT fast path and indefinite optimistic caching. Mutable values reuse the same per-key Paxos flow, but each overwrite runs a new versioned round for that key.

### Why there is only one Phase 2 message

In standard Paxos, Phase 2 (`Accept`) asks replicas to accept a value at a given ballot. Epoxy uses that standard message directly: it writes the ballot and accepted value to `kv/{key}/ballot` and `kv/{key}/accepted` in one transaction, and `Commit` is the Learn/Decide notification that tells replicas the value is final.

### Crash recovery

If a proposer crashes after `Accept` succeeds but before `Commit` completes, the key is left with a non-zero ballot and an accepted value but no committed value.

Recovery is lazy. There is no failure detector, timeout, or background scan that notices the stranded state. Recovery only happens when another proposer tries to write the same key. That proposer's ballot selection sees the non-zero ballot, enters the `Prepare` path, and collects the accepted value from the replicas that stored it. If nobody ever tries to write that key again, the stranded state sits there indefinitely, which is harmless.

This works because Epoxy only serves actor key reservation. If a key matters, a caller will eventually try to reserve it and trigger recovery as a side effect. Standard Paxos safety requires re-proposing the highest accepted value, which is what the recovering leader does. This ensures the earlier majority-accepted value is not lost.

## Sequence Diagrams

### Fast path: new key (1 RTT)

The common case for actor key reservation. No prior state exists for the key. Replica A is the proposing leader. All three replicas participate in the quorum (the leader sends to itself through the same handler).

```text
Replica A (leader)               Replica B               Replica C
  |                                 |                       |
  | [read local: no value, no ballot]                       |
  | [generate ballot (1, A)]        |                       |
  |                                 |                       |
  |--- Accept(val, 1,A) ------->|                       |
  |--- Accept(val, 1,A) -------------------------------->|
  |                                 |                       |
  |<------------- Ok --------------|                       |
  |<------------- Ok ----------------------------------------|
  |                                 |                       |
  | [fast quorum reached (3/3)]     |                       |
  | [commit local tx]               |                       |
  |                                 |                       |
  |--- Commit(key, val) ----------->|                       |
  |--- Commit(key, val) ---------------------------------->|
  |                                 |                       |
  v  done: Committed               v                       v
```

### Already committed key (0 RTT immutable write, local read)

A read or duplicate immutable write hits a key that was already committed. Ballot selection short-circuits without any network traffic. Mutable keys do not stop here. They use the committed value's version to start the next overwrite round.

```text
Replica A (leader)
  |
  | [read local: value exists]
  | [return AlreadyCommitted(v)]
  |
  v  done
```

### Slow path: prior in-flight state (2 RTT)

Replica C starts a proposal but crashes before it can commit. Later, Replica A tries to write the same key for an unrelated reservation. A's ballot selection finds C's stranded ballot and triggers recovery. Even though A has a lower replica ID than C, it supersedes C's stalled proposal by incrementing the counter to `(2, A)`, which is higher than `(1, C)` because the counter is compared first.

```text
Replica A                        Replica B               Replica C
  |                                 |                       |
  |                                 |   [generate ballot (1, C)]
  |                                 |                       |
  |<--- Accept(val, 1,C) --------|<-- Accept(val, 1,C) --
  |                                 |                       |
  |-------------- Ok -------------->|------------- Ok ----->|
  |                                 |                       |
  |                                 |     [fast quorum (3/3)]
  |                                 |     [crash before Commit]
  |                                 |                       x
  |                                 |                       x
  | --- later, A tries to write this key ---                x
  |                                 |                       x
  | [read local: no value, ballot (1, C)]                   x
  | [prior state found, must Prepare]                       x
  | [generate ballot (2, A)]        |                       x
  | [(2, A) > (1, C) because 2 > 1] |                       x
  |                                 |                       x
  |--- Prepare(key, 2,A) --------->|                       x
  |                                 |                       x
  |<--- Ok(accepted=val@1,C) ------|                       x
  |                                 |                       x
  | [slow quorum reached (2/3)]     |                       x
  | [re-propose C's accepted value] |                       x
  |                                 |                       x
  |--- Accept(val, 2,A) ------->|                       x
  |                                 |                       x
  |<------------- Ok --------------|                       x
  |                                 |                       x
  | [fast quorum reached (2/2)]     |                       x
  | [commit local tx]               |                       x
  |                                 |                       x
  |--- Commit(key, val) ----------->|                       x
  |                                 |                       x
  v  done: Committed               v                       x
```

### Concurrent proposers on a fresh key (contention)

Replica A and Replica C both try to write the same fresh key simultaneously. Both see no prior state and skip Prepare. With `n=3`, the fast quorum is 3 (all replicas), so at most one proposer can reach it. The loser falls back to the slow path on retry.

```text
Replica A (leader)               Replica B               Replica C (leader)
  |                                 |                       |
  | [no value, no ballot]           |       [no value, no ballot]
  | [generate ballot (1, A)]        |   [generate ballot (1, C)]
  |                                 |                       |
  |--- Accept(val_a, 1,A) ----->|                       |
  |                                 |<-- Accept(val_c, 1,C) --
  |                                 |                       |
  |                  [B accepts (1,A)]                      |
  |                  [then sees (1,C) > (1,A)]              |
  |                                 |                       |
  |<------- HigherBallot ----------|                       |
  |                                 |----------- Ok ------->|
  |                                 |                       |
  | [only 1/3, no fast quorum]      |   [3/3, fast quorum reached]
  | [ConsensusFailed]               |                       |
  |                                 |           [commit local tx]
  |                                 |<--- Commit(val_c) ----|
  |<---------- Commit(val_c) ------|                       |
  |                                 |                       |
  | [retry: read local]             |                       |
  | [sees committed val_c]          |                       |
  | [AlreadyCommitted or            |                       |
  |  ExpectedValueDoesNotMatch]     |                       |
  |                                 |                       |
  v                                 v                       v
```

The replica-ID component of the ballot is the tiebreaker. When two leaders use the same counter, the higher replica ID wins Accept at any replica that sees both requests. Since fast quorum requires all replicas for `n=3`, the lower-ID leader cannot reach quorum.

### Learner catch-up (reconfiguration)

A new datacenter joins the cluster. The coordinator (which runs on Replica A) adds the new replica as `Joining`, broadcasts the updated config so live commits reach it, then starts catch-up. The joining replica pages the changelog from one active source while also applying live commits that arrive concurrently.

```text
Replica A (coordinator)          Replica B (source)      Replica C (joining)
  |                                 |                       |
  | [detect new DC (C)]             |                       |
  | [health check OK]               |                       |
  | [add C as Joining to config]    |                       |
  |                                 |                       |
  |--- UpdateConfig(+C joining) -->|                       |
  |--- UpdateConfig(+C joining) -------------------------------->|
  |                                 |                       |
  | [config broadcast done]         |                       |
  | [live commits now reach C]      |                       |
  |                                 |                       |
  |--- BeginLearning ------------------------------------------>|
  |                                 |                       |
  |                                 |       [choose B as source]
  |                                 |<-- ChangelogRead(cursor=0, N)
  |                                 |--- [{k1,v1},{k2,v2}] --->|
  |                                 |           [apply k1, k2] |
  |                                 |                       |
  |                                 |--- Commit(k3,v3) --->|
  |                                 |               [apply k3] |
  |                                 |                       |
  |                                 |<-- ChangelogRead(cursor=page1.last)
  |                                 |--- [{k3,v3}] -------->|
  |                                 |          [k3 exists, no-op]
  |                                 |                       |
  |                                 |<-- ChangelogRead(cursor=page2.last)
  |                                 |--- empty page -------->|
  |                                 |        [catch-up complete]
  |                                 |                       |
  |<----------- UpdateReplicaStatus(Active) ----------------|
  |                                 |                       |
  | [mark C Active, bump epoch]     |                       |
  |--- UpdateConfig(C active) ---->|                       |
  |--- UpdateConfig(C active) --------------------------------->|
  | [trigger changelog GC]          |                       |
  |                                 |                       |
  v                                 v                       v
```

The key safety property is idempotency. A committed value that arrives via both a live `Commit` and the changelog page is applied only once because the second write is a no-op when `kv/{key}/value` already exists.

## Key Layout

All new consensus state lives under each replica's v2 UDB subspace. There is no shared global
log or shared ballot counter.

```text
/rivet/epoxy_v2/
    replica/{replica_id}/
        config = ClusterConfig

        kv/{key}/value = CommittedValue { value, version, mutable }
        kv/{key}/ballot = Ballot
        kv/{key}/accepted = KvAcceptedValue { value, ballot, version, mutable }
        kv/{key}/cache = CommittedValue { value, version, mutable }

        changelog/{versionstamp} = ChangelogEntry { key, value, version, mutable }
```

Legacy committed values remain read-only under `/rivet/epoxy/replica/{replica_id}/kv/{key}`.
Reads still check `committed_value`, then `value`, in that legacy subspace so old actors remain
visible after the hard cutover.

### `config`

The current cluster configuration for this replica. This includes the coordinator replica id, the config epoch, and every replica's status and peer URLs.

### `kv/{key}/value`

The committed value for `key`.

- Immutable keys write `CommittedValue { value, version = 1, mutable = false }` and never change again.
- Mutable keys overwrite this record with a higher `version` on every successful commit.
- Legacy raw values are still readable and are treated as `version = 0, mutable = false`.

### `kv/{key}/ballot`

The highest ballot this replica has seen for `key`. A replica will not accept proposals with a lower ballot. Ballots are `(counter, replica_id)` tuples compared lexicographically (counter first, then replica_id as tiebreak). Each ballot is scoped to one logical user key. There is no shared ballot across unrelated keys.

The ballot has two components for different reasons. The counter provides liveness: any replica can supersede any other replica's stalled proposal by incrementing the counter, which is necessary for crash recovery. Without it, a crashed high-ID replica would permanently block a key because no lower-ID replica could generate a higher ballot. The replica ID provides deterministic tiebreaking when two proposers use the same counter, which happens when multiple replicas try to write a fresh key simultaneously.

Ballot selection reads `value` and `ballot` together to decide whether the leader can use the fast path or must run `Prepare`.

### `kv/{key}/accepted`

The latest accepted but not yet committed proposal for `key`.

```text
{
    value: Vec<u8>,
    ballot: Ballot,
    version: u64,
    mutable: bool,
}
```

- `Accept` writes this record together with `ballot`.
- `Prepare` returns it so a recovery leader can re-propose the highest accepted value.
- `Commit` and changelog catch-up clear it once the value becomes committed.

### `kv/{key}/cache`

An optimistic cache of a committed value observed from another replica. Only used by `kv_get_optimistic`.

- Immutable keys can stay cached indefinitely.
- Mutable keys store the version in cache and invalidate older cache entries on commit.
- `SkipCache` reads bypass this key entirely and do not populate it.

### `changelog/{versionstamp}`

An append-only per-replica changelog entry:

```text
{
    key: Vec<u8>,
    value: Vec<u8>,
    version: u64,
    mutable: bool,
}
```

Entries are written with an FDB versionstamp key so appends do not contend with each other. Each replica maintains its own changelog because versionstamps are local ordering tokens, not globally comparable across replicas. Learners page this changelog during catch-up. The coordinator uses learner cursors to garbage-collect old entries.

The cursor returned by `ChangelogRead` is only meaningful for the replica that produced it. Catch-up must stay on a single source replica for the whole session.

## Proposal Flow

The compiled proposal path accepts the old `Proposal` wrapper for compatibility, but the supported operation is intentionally narrow: exactly one command, exactly one key, and a concrete value. Immutable writes use set-if-absent semantics. Mutable writes opt into overwrite semantics at the operation input.

### 1. Ballot Selection

The leader reads the local state for the target key in one serializable UDB transaction:

1. Read `kv/{key}/value`
2. Read `kv/{key}/ballot`

Outcomes:

- If `value` exists and the key is immutable, stop and return `AlreadyCommitted` with the committed value. The caller compares this against the requested value later.
- If `value` exists and the key is mutable and the caller requested a mutable write, continue with `version = committed.version + 1`. If the ballot was cleared by the previous mutable commit, the proposer can use the fast path again.
- If `ballot` is missing or zero, generate a fresh ballot `(1, replica_id)` and continue to Accept.
- If `ballot` already exists, run Prepare first.

This preserves the 1 RTT fast path for untouched keys while still recovering safely from prior in-flight proposals.

### 2. Prepare

Prepare is Paxos Phase 1 for a single key. The leader uses it whenever ballot selection finds prior ballot state.

#### Request

```text
PrepareRequest {
    key,
    ballot,
    mutable,
    version,
}
```

The leader chooses `ballot = (max_counter + 1, replica_id)`, where `max_counter` is the highest counter it has seen so far for that key.

#### Replica behavior

Each replica reads `kv/{key}/value`, `kv/{key}/ballot`, and `kv/{key}/accepted` in one transaction.

- If `value` exists, reply `AlreadyCommitted`.
- If `value` exists but the request targets a higher mutable version, continue normally.
- If `request.ballot <= current_ballot`, reply `HigherBallot`.
- Otherwise, write `kv/{key}/ballot = request.ballot` and reply `Ok` with:
  - `highest_ballot`
  - `accepted_value`, including its version and mutable flag, if any
  - `accepted_ballot`, if any

#### Leader behavior

The leader sends Prepare to a slow quorum, including itself through the standard message path.

- If any replica returns `AlreadyCommitted`, stop and return the committed value.
- If too many replicas reject with higher ballots to reach the quorum, bump the ballot and retry Prepare with exponential backoff and jitter (10ms initial, 2x, 1s cap, max 10 retries).
- If quorum promises succeed, choose the value for Phase 2:
  - If any replica reported an accepted value, re-propose the value with the highest accepted ballot.
  - Otherwise, keep the client's requested value.

Re-proposing the highest accepted value is the safety rule that prevents an earlier majority-accepted value from being lost.

### 3. Accept

Accept is Epoxy's Phase 2 message.

#### Request

```text
AcceptRequest {
    key,
    value,
    ballot,
    mutable,
    version,
}
```

#### Replica behavior

Each replica reads `kv/{key}/value` and `kv/{key}/ballot` in one transaction.

- If `value` exists and the request is immutable, or the request's mutable version is stale, reply `AlreadyCommitted`.
- If `current_ballot > request.ballot`, reply `HigherBallot`.
- Otherwise:
  - write `kv/{key}/ballot = request.ballot`
  - write `kv/{key}/accepted = { value, ballot, version, mutable }`
  - reply `Ok`

#### Leader behavior

The leader sends Accept to the target quorum for the current path.

- If any replica returns `AlreadyCommitted`, stop and return that value.
- If some replicas return `HigherBallot`, keep waiting for other responses.
- If the target quorum replies `Ok`, continue to commit.
- If the target quorum becomes unreachable, return `ConsensusFailed`.

The common case for a fresh actor key is ballot selection plus one Accept round, which is the 1 RTT write path.

### 4. Commit

The leader commits locally before broadcasting the replicated commit.

#### Local commit transaction

The leader runs one serializable UDB transaction that:

1. Reads `kv/{key}/ballot`
2. Rejects if a higher ballot has appeared and this replica no longer has a matching accepted
   record for the chosen value
3. Reads `kv/{key}/value`
4. Returns `AlreadyCommitted` if the key is already committed
5. Writes `kv/{key}/value = { value, version, mutable }`
6. Deletes `kv/{key}/accepted`
7. Deletes `kv/{key}/ballot` and `kv/{key}/cache` for mutable commits
8. Appends `{ key, value, version, mutable }` to `changelog/{versionstamp}`

If this transaction fails because the ballot was preempted before this replica accepted the chosen
value, the proposal returns `ConsensusFailed`.

#### Commit broadcast

After the local write succeeds, the leader broadcasts:

```text
CommitRequest {
    key,
    value,
    ballot,
    mutable,
    version,
}
```

Replica behavior:

- If `kv/{key}/value` already exists, return `AlreadyCommitted`.
- Mutable commits only overwrite when `request.version` is higher than the committed version.
- If `commit.ballot < current_ballot`, return `StaleCommit`.
- Otherwise:
  - write `kv/{key}/value`
  - clear `kv/{key}/accepted`
  - clear `kv/{key}/ballot` and `kv/{key}/cache` for mutable keys
  - append the entry to the local changelog
  - return `Ok`

Commit is intentionally idempotent. That is required for retry handling and learner catch-up.

### Caller result mapping

Immutable writes keep the old set-if-absent result mapping.

- If an immutable committed value matches the requested value, the proposal returns `Committed`.
- If an immutable committed value differs, the proposal returns `ExpectedValueDoesNotMatch`.
- Mutable writes return `Committed` when their overwrite commits successfully.
- If quorum cannot be reached without a higher-ballot retry, the proposal returns `ConsensusFailed` and the caller retries.

## Quorums

Epoxy uses Fast Paxos quorum sizes:

- Slow quorum: `floor(n / 2) + 1`
- Fast quorum: `n - floor((slow_q - 1) / 2)`
- Safety invariant: `2 * fast_q + slow_q > 2 * n`

This keeps slow quorums at a strict majority and ensures any two fast quorums plus one slow quorum still intersect safely. In small clusters this means:

| n | slow_q | fast_q |
|---|--------|--------|
| 3 | 2      | 3      |
| 4 | 3      | 3      |
| 5 | 3      | 4      |
| 7 | 4      | 6      |

Only `Active` replicas participate in proposal quorums. Sender-excluded fanout quorum helpers subtract one from these sizes for `Fast`, `Slow`, and `All`, while `Any` still targets a single response. Commit fanout still goes to joining learners so catch-up can stay current while they bootstrap.

## Reads

Reads support two cache behaviors.

- `Optimistic` checks the local optimistic cache on miss and writes back the first remote committed value it finds.
- `SkipCache` bypasses the local optimistic cache entirely and does not populate it. It still checks the local committed value first.

The local read order is:

1. v2 `kv/{key}/value`
2. Legacy `kv/{key}/committed_value`
3. Legacy `kv/{key}/value`
4. v2 `kv/{key}/cache` when using `Optimistic`
5. Fan out to other datacenters and cache the first committed value found when using `Optimistic`

This keeps the steady-state read path local.

### Why reads use a separate cache key

On an optimistic cache miss, the fanout result is written to `kv/{key}/cache` instead of `kv/{key}/value`. This keeps reads isolated from the Paxos consensus path. Writing directly to `kv/{key}/value` would require a full Commit transaction: checking the ballot, clearing `kv/{key}/accepted`, and appending a changelog entry. Skipping any of those steps would leave the replica in an inconsistent state.

The separate cache key is simpler because it does not touch consensus state at all. The tradeoff is one extra key per remotely-read value, but it avoids turning reads into write transactions.

#### TODO: eliminate the cache key

The cache key could be removed by running a proper Commit transaction on fanout hit. The value is already committed on another replica and values are immutable, so this is safe. The benefits would be:

- One fewer key per remotely-read value
- Reads would populate `kv/{key}/value` directly, so subsequent reads skip the cache lookup
- The changelog would be complete, so learners catching up from this replica would not miss the entry

The cost is that a cache-miss read becomes a write transaction (ballot check + value write + accepted clear + changelog append) instead of a single cache key write. This is a latency and contention tradeoff: the current cache approach adds ~0 RTT overhead on the read path, while a Commit transaction would add the cost of a serializable UDB transaction. For now, the separate cache key is the simpler approach because it keeps reads completely outside the well-defined Paxos flow.

## Reconfiguration

Each datacenter runs a replica workflow, and the leader datacenter also runs a coordinator workflow.

### Coordinator flow

When the coordinator detects new datacenters in topology:

1. Read the current topology and compare it with the stored cluster config.
2. Health check each new replica until it is reachable or the topology changes again.
3. Add the new replicas to cluster config with status `Joining`.
4. Register each joining replica in coordinator learner state for changelog GC tracking.
5. Broadcast the joining config to every replica before catch-up starts.
6. Send `BeginLearning` to each joining replica.

Step 5 is important. Commit fanout uses the current config's full replica list, so broadcasting the joining config first is what lets live commits reach the learner while it is downloading history.

### Learner flow

On `BeginLearning`, the learner:

1. Stores the provided cluster config locally.
2. Chooses one active source replica from that config.
3. If there is no active source replica yet, skips catch-up and reports `Active`.
4. Otherwise pages that source replica's changelog until it reaches the end.

### Changelog catch-up loop

For each page:

1. Send `ChangelogRead(afterVersionstamp, count)` to the chosen source replica.
2. For every returned `{ key, value, version, mutable }` entry, apply it locally:
   - write `kv/{key}/value` if the key is absent
   - keep immutable duplicates only when the value matches exactly
   - overwrite mutable values only when the incoming version is higher
   - clear `kv/{key}/accepted`
   - clear `kv/{key}/ballot` and `kv/{key}/cache` for mutable entries
   - append the entry to the learner's own changelog
3. Advance `afterVersionstamp` to the page's `lastVersionstamp`.
4. Repeat until the source returns an empty page.

This path is idempotent, so live `Commit` messages that arrive while the learner is still paging history are safe. If the same committed value arrives twice, the second apply is a no-op.

### Promotion to Active

After catch-up finishes, the learner sends `CoordinatorUpdateReplicaStatus(status=Active)`.

The coordinator then:

1. Marks the replica `Active` in cluster config.
2. Removes it from learner GC tracking.
3. Increments the config epoch.
4. Broadcasts the updated config to every replica.
5. Triggers changelog GC.

The config epoch versions cluster membership changes. It is not part of the per-key Paxos ballot.

### Changelog GC

The coordinator tracks learners by source replica and remembers the oldest catch-up cursor still in use.

- If learners are reading from a source replica, that oldest cursor becomes the GC watermark for that source.
- If no learners are reading from a source replica, GC can truncate through the replica's latest currently visible changelog entry.
- If a learner has not established a cursor yet, GC must not truncate that source replica's changelog.

Each replica's changelog is garbage-collected independently because versionstamp cursors are replica-local.

## Transport

Peer-to-peer traffic uses raw BARE payloads over fixed v2 HTTP endpoints:

- `POST /v2/epoxy/message` for replica RPCs such as `Prepare`, `Accept`, `Commit`, `UpdateConfig`, and status updates
- `POST /v2/epoxy/changelog-read` for paginated changelog reads during learner catch-up

The route layer still validates request kinds so changelog reads do not silently fall back onto
the generic message endpoint.

## Legacy Fallback

The legacy key layout stored committed values under `/rivet/epoxy/replica/{replica_id}` while the
current layout uses `/rivet/epoxy_v2/replica/{replica_id}`. There is no background migration. All
new writes go to v2 and old committed values are discovered through local dual-read fallback.

### Dual-read fallback

Reads check both layouts in order: v2 `kv/{key}/value`, legacy `kv/{key}/committed_value`,
legacy `kv/{key}/value`, then v2 `kv/{key}/cache`. `ops/kv/read_value.rs` handles that lookup,
and `replica/ballot.rs` uses the same legacy subspace checks so already-committed legacy keys
still short-circuit before any new ballot is written.

### Legacy state that is not preserved

Only committed values are read from the legacy subspace. Legacy ballots, accepted values,
changelogs, and config are ignored after the cutover. If an old replica had an in-flight accepted
value without a commit, that proposal is lost and a later proposer will either find a committed
value in the legacy subspace or retry the key from scratch.
