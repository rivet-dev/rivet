# FoundationDB Native Backup

## Sources

- [Backup, Restore, and Replication for Disaster Recovery — apple.github.io](https://apple.github.io/foundationdb/backups.html) — primary user-facing reference for `fdbbackup` / `fdbrestore` CLI surface.
- [`design/backup.md` (apple/foundationdb)](https://github.com/apple/foundationdb/blob/main/design/backup.md) — high-level architecture and TaskBucket / proxy-mutation-log mechanism.
- [`design/backup-dataFormat.md` (apple/foundationdb)](https://github.com/apple/foundationdb/blob/main/design/backup-dataFormat.md) — exact range-file / log-file binary layout and naming scheme.
- [`design/backup_v2_partitioned_logs.md` (apple/foundationdb)](https://github.com/apple/foundationdb/blob/main/design/backup_v2_partitioned_logs.md) — backup workers, partitioned logs, hot-path-overhead reduction in v2.
- [`documentation/sphinx/source/backups.rst` (apple/foundationdb)](https://github.com/apple/foundationdb/blob/main/documentation/sphinx/source/backups.rst) — full CLI reference with subcommand semantics.
- [Disk snapshot backup — apple.github.io](https://apple.github.io/foundationdb/disk-snapshot-backup.html) — counterpoint to logical backup, useful for tradeoff discussion.
- [Backup & restore performance tuning — forums](https://forums.foundationdb.org/t/backup-restore-performance-tuning/2078) — production throughput numbers.
- [Trying to understand the backup mechanism better — forums](https://forums.foundationdb.org/t/trying-to-understand-the-backup-mechanism-better/588) — Steve Atherton's explanation of proxy-side mutation logging and TaskBucket.
- [About backup mechanism — forums](https://forums.foundationdb.org/t/about-backup-mechanism/1477) — concurrent backups on different key ranges, multi-tenancy caveats.
- [Lots of questions about backup and restore — forums](https://forums.foundationdb.org/t/lots-of-questions-about-backup-and-restore/2888) — v2 backup status, DR pause/resume.
- [Backing up FoundationDB — Tigris blog](https://www.tigrisdata.com/blog/backing-up-foundationdb/) — real-world S3 layout (5,407 files / 446.7 GiB), retention, IAM gotchas.
- [Snowflake on FoundationDB — Snowflake blog](https://www.snowflake.com/en/blog/how-foundationdb-powers-snowflake-metadata-forward/) — large-scale FDB deployment context.
- [`fdbbackup` expire on blobstore returning error — forums](https://forums.foundationdb.org/t/fdbbackup-expire-with-blobstore-returning-error-not-deleting-from-s3/1131) — expire mechanics and S3 IAM failure modes.

## Architecture overview

FDB native backup is a **logical, key-value-level** backup driven by stateless agent processes that coordinate through the database itself.

- **`fdbbackup`**: control-plane CLI for backup tasks. Subcommands: `start`, `modify`, `abort`, `discontinue`, `wait`, `status`, `list`, `tags`, `describe`, `expire`, `delete`, `pause`, `resume`, `cleanup` ([backups.rst](https://github.com/apple/foundationdb/blob/main/documentation/sphinx/source/backups.rst)).
- **`fdbrestore`**: control-plane CLI for restore tasks. Subcommands: `start`, `abort`, `wait`, `status` ([backups.rst](https://github.com/apple/foundationdb/blob/main/documentation/sphinx/source/backups.rst)).
- **`backup_agent`**: long-running daemon that does the actual work. "By default, the FoundationDB packages are configured to start a single backup_agent process on each FoundationDB server. Each backup agent will be responsible for a range of keys, which they will either store locally or stream to the object store" ([apple.github.io/backups](https://apple.github.io/foundationdb/backups.html)).
- **`fdbdr` / `dr_agent`**: equivalent CLI/daemon pair for DB-to-DB disaster-recovery streaming (parallel design, different destination).
- **Backup workers (v2 only)**: a stateless FDB role recruited per log-router tag that pulls mutations directly from the transaction logs and uploads partitioned mutation logs to blob storage ([backup_v2_partitioned_logs.md](https://github.com/apple/foundationdb/blob/main/design/backup_v2_partitioned_logs.md)).

End-to-end flow (continuous backup):

1. `fdbbackup start -z ...` writes a configuration key under `\xff` that describes the backup tag, key range, and destination URL. Steve Atherton: "The initial backup Task, as part of its completion transaction, sets a special key in `\xff` which tells proxies to start logging mutations for a specific key range to another place in `\xff`" ([forums #588](https://forums.foundationdb.org/t/trying-to-understand-the-backup-mechanism-better/588)).
2. In v1, **commit proxies** observe the config and copy each committed mutation that intersects the backup key range into a backup mutation log keyspace. In v2, this responsibility moves to backup workers reading from transaction logs directly, removing the proxy double-write.
3. `backup_agent` processes pull tasks out of a TaskBucket queue ("Backups Agents use TaskBucket to divide the work of a backup into many small Tasks and execute them transactionally" — [forums #588](https://forums.foundationdb.org/t/trying-to-understand-the-backup-mechanism-better/588)). Tasks include "scan key range R at version V into a range file" and "drain backup mutation log into log files."
4. Range scans produce **range files** (inconsistent KV snapshot of one slice at one version). Mutation drains produce **log files** (versioned mutations). Both are written directly to S3 / blobstore / local FS.
5. Periodically, snapshot tasks are restarted at a new version so the backup contains a sequence of inconsistent snapshots plus a continuous mutation stream.

The cluster does not need to "freeze" or hold a long read version. The backup is reconciled from "an *inconsistent* copy of the data with a log of database changes that took place during the creation of that inconsistent copy" ([apple.github.io/backups](https://apple.github.io/foundationdb/backups.html)).

## Granularity

- **Whole-cluster vs subset.** Subset is supported. `fdbbackup start -k '<BEGIN> <END>'` (repeatable) restricts the backup to one or more lexicographic key ranges. Without `-k`, the full user keyspace is backed up. System keyspace (`\xff`) is excluded.
- **Tag/scope model.** "A 'tag' is a named slot in which a backup task executes. Backups on different named tags make progress and are controlled independently" ([apple.github.io/backups](https://apple.github.io/foundationdb/backups.html)). The default tag is `default`.
- **Concurrent backups.** Multiple tags can run at the same time on overlapping or disjoint ranges. Confirmed on the forum: "you can have many backups running, each targeting set of key ranges of your choosing" ([forums #1477](https://forums.foundationdb.org/t/about-backup-mechanism/1477)).
- **Tenant isolation.** FDB has no native per-tenant ACL on the backup surface. Tags are flat strings; the destination URL is per-tag. Each tag effectively becomes its own backup container in the blobstore. Multi-tenancy is delivered by application-level prefix conventions (Record Layer style), not by the backup system itself.

## Storage model

- **File format.** Two file types defined in [backup-dataFormat.md](https://github.com/apple/foundationdb/blob/main/design/backup-dataFormat.md):
  - **Range files** describe key-value pairs in a range at the version when the backup process took the snapshot of that range. Block layout: `Header startKey k1 v1 k2 v2 ... Padding`. "All blocks except for the final block will have one last value which will not be used."
  - **Log files** describe mutations from version `v1` to `v2`. Block layout: `Header, [Param1, Param2]... padding` where `Param1 = hashValue|commitVersion|part` and `Param2` contains mutations encoded as `type|kLen|vLen|Key|Value`.
  - Integers are stored big-endian on disk and converted at read time.
- **Object layout.**
  - Range files: `snapshots/snapshot,beginVersion,beginVersion,blockSize`.
  - Log files (v1): `logs/<x>/<y>/log,beginVersion,endVersion,randomUID,blockSize` where `<x>/<y>` is a 2-level version prefix designed to bucket roughly `10^smallestBucket` versions per leaf path so directory listings stay tractable.
  - Log files (v2 partitioned): `log,startVersion,endVersion,UID,N-of-M,blockSize` where `M` is the partition count (typically equals log-router-tag count) and `N` is the partition index.
  - A backup is a folder under `<base_url>/<name>` with `snapshots/`, `logs/`, plus metadata/properties files used by `expire` / `describe` / `list`.
  - Tigris production example: a single backup container held **5,407 objects / 446.7 GiB** ([Tigris blog](https://www.tigrisdata.com/blog/backing-up-foundationdb/)).
- **Snapshot vs log files.** Snapshots are the complete data at one version, fragmented across many range files. Logs are the mutation stream between snapshots. Restore uses one snapshot plus the log range covering snapshot-version → target-version.
- **Compression / encryption.** No first-class object-level compression in the documented format. TLS to the blobstore is supported (`secure_connection=1`). At-rest encryption is the responsibility of the bucket / storage provider.

## Point-in-time recovery

- **Granularity.** Per-version (FDB versions tick at ~10^6/s, so PITR is sub-second). Restore can also be expressed by wall-clock timestamp via `--timestamp`, which converts to a version using the original cluster's version-to-time history (so the **original cluster file is required** for timestamp-based restore).
- **Mechanism.**
  1. Pick a target version `vt` within the backup's restorability window.
  2. Choose the latest snapshot whose `endVersion <= vt` (call it `vs`).
  3. Apply all range files for that snapshot (writes raw KV pairs).
  4. Replay log files covering `[vs, vt]` in version order. Atherton: "restore will write all of a backup's key range snapshots to the database and apply the backup's mutation log to update them to the target version" ([forums #2078](https://forums.foundationdb.org/t/backup-restore-performance-tuning/2078)).
  5. The "inconsistent snapshot + log replay" model means the snapshot doesn't have to be a consistent cut — replay heals it.
- **Version model.** FDB has a single global monotonically increasing commit version; backup files are tagged with these versions. Versionstamps appear inside committed mutations but are not the granularity unit of the backup itself. `describe` reports the snapshot version range and the restorable version range.

## Operational model

- **Start/stop.**
  - Start: `fdbbackup start -d <URL> -t <tag> -z -s <interval> -k '<begin> <end>'` (the `-z` flag makes it continuous; without it the agent stops after one restorable backup is achieved).
  - Stop: `discontinue` (clean stop, becomes a finite/restorable backup) or `abort` (immediate stop, "the destination is NOT deleted automatically, and it may or may not be restorable depending on when the abort is done" — [backups.rst](https://github.com/apple/foundationdb/blob/main/documentation/sphinx/source/backups.rst)).
  - `pause` / `resume` halt all backup_agent work cluster-wide without aborting tasks.
- **Restore procedure.**
  - The user must clear or pre-stage the target keyspace; restore "will not clear the target key ranges, for safety reasons" ([apple.github.io/backups](https://apple.github.io/foundationdb/backups.html)).
  - `fdbrestore start -r <URL> -k <range> -v <version>` (or `--timestamp`) launches a restore tag that schedules tasks across backup_agents.
  - Optional `--remove-prefix` / `--add-prefix` to relocate the restored data under a different prefix, which is the standard mechanism for restoring "into" a different namespace on the same cluster.
- **Failure modes.**
  - Agent crash: another agent picks up the task next transaction (TaskBucket atomically reassigns), so progress is durable as of the last task commit.
  - Cluster recovery: backup workers (v2) checkpoint progress under `\xff\x02/backupStarted` etc., so a recovered cluster spawns new workers that resume from the last saved version. Some duplicate version ranges may be re-uploaded; restore dedupes them.
  - Pre-6.0.18: a TCP-reuse bug could silently drop log uploads, breaking log continuity ([forums #1131](https://forums.foundationdb.org/t/fdbbackup-expire-with-blobstore-returning-error-not-deleting-from-s3/1131)).
  - S3 permission failures (HTTP 403) are common with IAM-role auth on Kubernetes; dedicated IAM users with bucket-scoped policies work around it ([Tigris blog](https://www.tigrisdata.com/blog/backing-up-foundationdb/)).
  - Bucket lifecycle policies must be **disabled** ("No automatic deletion/archival policies allowed. Only the `fdbbackup` utility understands which key ranges remain necessary") ([Tigris blog](https://www.tigrisdata.com/blog/backing-up-foundationdb/)).

## Performance and storage cost

- **Hot-path overhead (per mutation).**
  - **v1**: every committed mutation in a backed-up range is written **twice** — once into the user keyspace, once into the backup mutation log keyspace. This roughly doubles TLog write bandwidth for the backed-up range and adds significant CPU on commit proxies.
  - **v2 partitioned logs**: backup workers read from transaction logs directly, which "removes the requirement to generate backup mutations at the CommitProxy, thus reduce TLog write bandwidth usage by half and significantly improve CommitProxy CPU usage" ([backup_v2_partitioned_logs.md](https://github.com/apple/foundationdb/blob/main/design/backup_v2_partitioned_logs.md)).
- **Throughput.**
  - Production: 3.1 GB/min backup (1.5 TB in 8 h), 0.93 GB/min restore (1.5 TB in 27 h, single-replica) ([forums #2078](https://forums.foundationdb.org/t/backup-restore-performance-tuning/2078)).
  - Restore historically capped at "around 100MB/s on a well-tuned cluster" because mutation-log application went through a single commit proxy. v6.3+ parallel restore workers raise this ceiling.
  - Recommendation: "One backup agent per 6 fdb storage processes" ([forums #2078](https://forums.foundationdb.org/t/backup-restore-performance-tuning/2078)).
- **Storage cost ratio.** Continuous backup keeps the full mutation history since the oldest unexpired snapshot. Repeated rewrites of the same key produce log entries that never coalesce until expiration moves the cutoff forward. Tigris noted that "updating values without size changes (timestamps, integers) permanently increases backup size despite leaving the database itself unaffected" ([Tigris blog](https://www.tigrisdata.com/blog/backing-up-foundationdb/)).
- **Backup file lifecycle.** `fdbbackup expire --restorable-after-version <V>` (or `--restorable-after-timestamp`) keeps the backup restorable to any version ≥ `V` and reclaims everything older. `expire` deletes range/log files first, then metadata; `delete` removes the entire backup. There is no automatic compaction — old snapshots remain whole until expired past.

## Comparison to application-layer backup

- **When users prefer native:** transactional consistency across the entire cluster, no per-application code, supports cross-cluster DR via `fdbdr`, mature CLI tooling, supports key-range scoping and PITR out of the box.
- **When users prefer custom:** need per-tenant retention policies, want to participate in application-defined snapshot semantics (e.g. "snapshot at the boundary of an actor lifecycle event"), need per-tenant restore without coordinating a cluster-wide tag, want logical export to a different schema/format, want to encrypt with a per-tenant key, or want compaction of repeated overwrites.
- **Hard limitations of native backup for fine-grained workloads:**
  - Tags are coarse: each tag is a control-plane object that lives in `\xff` and is enumerated by `fdbbackup tags` / `list`. Standing up tens of thousands of tags (one per actor) is **not** the documented use case and would create a metadata explosion in `\xff` plus a `s3://.../<tag>/` per actor.
  - No per-tag retention defaults; you must run `expire` per tag.
  - Restore reads from a single backup container at a time. Restoring N actors means N restore jobs.
  - Restore-into-running-cluster requires the user to clear/move the destination range. There is no "restore actor X to version V while X is live" primitive.
  - Versions are global FDB versions, not per-actor logical versions, so PITR semantics are tied to cluster wall-time, not per-actor activity.
  - Bucket lifecycle policies cannot be used; FDB owns the retention model.

## Direct relevance to our SQLite-on-FDB-with-PITR design

- **Use FDB-native backup directly per actor: rejected.** One tag per actor would put O(actor_count) configuration entries in `\xff` and one S3 prefix per actor in the backup bucket. The tag system is designed for cluster-scale jobs, not millions of independent backup streams. Per-actor restore latency would also pay a TaskBucket scheduling round-trip per actor.
- **Idea worth transplanting: "inconsistent snapshot + mutation log" reconcile model.** FDB explicitly does **not** require a consistent cut for the snapshot — the log replay heals it. We can do the same: emit per-actor SQLite page-image dumps lazily while continuously appending the WAL/mutation stream, and rely on log replay during restore to reach a consistent state.
- **Idea worth transplanting: range-file naming `snapshot,beginVersion,beginVersion,blockSize` and log-file naming `log,beginVersion,endVersion,UID,N-of-M,blockSize`.** Versions in the filename make listing-driven restore trivially correct without an index file, and the `N-of-M` partition naming lets multiple writers race without coordination as long as M is fixed.
- **Idea worth transplanting: 2-level version-prefix bucketing (`x/y/log,...`)** so a single S3 prefix never holds an unbounded number of objects. With our actor-id partitioning we'd similarly want `<actor_id>/<version-bucket>/<file>`.
- **Idea NOT worth transplanting: TaskBucket-style global agent pool draining a `\xff` queue.** Our actors already own their own writers; we don't need a separate stateless agent fleet contending for FDB tasks. Per-actor compaction by the actor's writer (or by a dedicated compactor lease per actor) is a better fit and avoids the agent-count tuning problem.
- **Idea NOT worth transplanting: "user must clear destination before restore."** Per-actor PITR/forking should be transactional and online — fork an actor at version V into a new actor id without disturbing the running source. FDB-native restore is offline-into-cleared-range and would be a regression.
- **What we'd still have to build even if we used FDB-native backup:** per-actor retention policy, per-actor restore RPC, online fork (FDB restore is destination-clear), per-actor encryption keys, log-file compaction across overwrites, integration with our SQLite VFS so the restored bytes become a usable database without an external "load-from-KV" step.

## Direct quotes / code references

- "The FoundationDB backup software is distributed in operation, comprised of multiple backup agents which cooperate to perform a backup or restore faster than a single machine can send or receive data and to continue the backup process seamlessly even when some backup agents fail." ([apple.github.io/backups](https://apple.github.io/foundationdb/backups.html))
- "A full backup consists of an *inconsistent* copy of the data with a log of database changes that took place during the creation of that inconsistent copy. … combined to reconstruct a consistent, point-in-time snapshot." ([apple.github.io/backups](https://apple.github.io/foundationdb/backups.html))
- "A 'tag' is a named slot in which a backup task executes. Backups on different named tags make progress and are controlled independently." ([apple.github.io/backups](https://apple.github.io/foundationdb/backups.html))
- "blobstore://[<api_key>][:<secret>[:<security_token>]]@<hostname>[:<port>]/<name>?bucket=<bucket_name>" ([backups.rst](https://github.com/apple/foundationdb/blob/main/documentation/sphinx/source/backups.rst))
- Range file naming: `snapshots/snapshot,beginVersion,beginVersion,blockSize`. Log file naming: `logs/<x>/<y>/log,beginVersion,endVersion,randomUID,blockSize` ([backup-dataFormat.md](https://github.com/apple/foundationdb/blob/main/design/backup-dataFormat.md)).
- v2 partition naming: `log,[startVersion],[endVersion],[UID],[N-of-M],[blockSize]` ([backup_v2_partitioned_logs.md](https://github.com/apple/foundationdb/blob/main/design/backup_v2_partitioned_logs.md)).
- "Removes the requirement to generate backup mutations at the CommitProxy, thus reduce TLog write bandwidth usage by half and significantly improve CommitProxy CPU usage." ([backup_v2_partitioned_logs.md](https://github.com/apple/foundationdb/blob/main/design/backup_v2_partitioned_logs.md))
- "The initial backup Task, as part of its completion transaction, sets a special key in `\xff` which tells proxies to start logging mutations for a specific key range to another place in `\xff`." (Steve Atherton, [forums #588](https://forums.foundationdb.org/t/trying-to-understand-the-backup-mechanism-better/588))
- Restore "normally maxes out at the rate at which a single fdb proxy can apply the backup's mutation log, which is around 100MB/s on a well-tuned cluster." (Steve Atherton, [forums #2078](https://forums.foundationdb.org/t/backup-restore-performance-tuning/2078))
- "Each one will be responsible for a range of keys, which they will either store locally … or stream to the object store." ([Tigris blog](https://www.tigrisdata.com/blog/backing-up-foundationdb/))

## Open questions

- Exact `expire` cost on S3 for very large backup containers — does it `LIST` per prefix, or maintain its own object index? `expire` doc claims metadata-driven, but real-world thread suggests pathological cases.
- Maximum practical tag count per cluster. The design does not state a limit; production deployments seem to use single-digit tags. Anyone running >100 tags?
- Whether v2 partitioned logs are the on-by-default in current FDB releases (we assume yes for 7.x but should confirm against `engine/packages/sqlite-storage/` deployment FDB version).
- Real per-mutation byte cost in v2 log files (header + part + mutation framing) — we'd want to compare to our own WAL-frame chunking.
- Whether `--remove-prefix` / `--add-prefix` restore can target a *live* range without requiring a prior `clear`. Docs imply not, but we should test before ruling it out as a building block for actor forking.
