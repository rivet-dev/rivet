# SlateDB leadership patch

## Why this patch exists

**The patch makes SlateDB writer and embedded-compactor initialization respect the order of our external lease grants.** SlateDB already fences stale database handles. Its default initialization allocates a new storage epoch for each opener, independently of which process currently owns our lease.

The failure case is a delayed open:

1. A acquires lease token `10`, then pauses before its SlateDB open claims an epoch.
2. A's lease expires. B acquires token `11` and successfully opens the database.
3. A resumes its old open attempt. With ordinary SlateDB initialization, A can claim a newer storage epoch and fence B, even though B holds the valid lease.
4. With `with_leadership_epoch(10)`, A must use exactly `10`. Once B has installed `11`, A's initialization fails with `Fenced`; conflict retries cannot choose a higher epoch.

This also applies when an expired owner starts another open using its old grant. Our coordinator must issue strictly increasing tokens for each resource, and every opener must follow this contract. The token is fixed for the entire open attempt.

**An already-open A that stalls while B takes over was already covered by native SlateDB fencing.** Once B completes the WAL fence, A's existing handle cannot continue durable writes. Concurrent opens already have storage fencing too; the patch additionally connects their ordering to lease authority. The delayed-open scenario above can displace a legitimate leader without demonstrating a failure of SlateDB's native storage safety.

### How native SlateDB fencing still works

The manifest contains `writer_epoch`. The new writer claims its epoch and creates an empty object in the next WAL position using create-if-absent semantics. For example, if B creates the fence at WAL position `101`, A's attempt to write its next batch there fails because that object exists. SlateDB treats the collision as `Fenced`; the old writer cannot skip the failed position and continue. If A wins position `101` first, B verifies its manifest epoch, tries position `102`, and recovers A's successful write during replay. **A manifest epoch change alone does not complete WAL fencing.** See the pinned upstream [WAL storage code](https://github.com/slatedb/slatedb/blob/c1e36fc07fb9b759d31484dda18c1c1aad8a2853/slatedb/src/wal/slatedb/store.rs) and [initialization protocol](https://github.com/slatedb/slatedb/blob/c1e36fc07fb9b759d31484dda18c1c1aad8a2853/slatedb/src/wal/slatedb/writer_init.rs).

### Why the embedded compactor also needs the token

SlateDB has a separate `compactor_epoch` in its manifest and compaction metadata. A delayed embedded-compactor initialization could otherwise claim a newer epoch after the successor has opened. The patch supplies the same lease token to the writer and embedded compactor, and preserves the existing propagation into the `.compactions` metadata.

Native compactor fencing protects publication: old work may finish uploading immutable SSTs, but a fenced coordinator cannot commit those results into the live manifest. Metadata updates use conditional writes and epoch validation. The patch preserves native checkpoint, publication, and garbage-collection ordering; it does not require every old upload to stop instantly. See the pinned upstream [compaction state protocol](https://github.com/slatedb/slatedb/blob/c1e36fc07fb9b759d31484dda18c1c1aad8a2853/slatedb/src/compactor_state_protocols.rs).

An independently started compactor that allocates its own epoch can violate this token contract and fence the embedded compactor. Such mixed operation is unsupported; independently managed compaction would need an explicit ownership design.

### Why initialization cleanup is included

Fencing can make an open fail after it has created internal state or started background work. The accompanying cleanup changes release that partial instance before the retained build future returns an error. The supervisor still owns timed-out or cancelled open/close operations until cleanup finishes; dropping a future is not proof that its tasks or remote requests have stopped.

### Remaining limits

- A token orders takeover; it does not make SlateDB check lease expiry on every read or write. Lease deadlines and admission remain the supervisor's responsibility, including revalidating a grant after a slow open.
- Open and close can remain pending on object-store I/O. The patch does not establish a failover-time bound or fix upstream compactor contention: [issue #1970](https://github.com/slatedb/slatedb/issues/1970) reports a 20+ minute open under cross-region contention and latency. That report is not a timing estimate for our single-AZ deployment.
- The small real-SlateDB tests below cover delayed writer and compactor initialization, WAL races, and close/error cleanup. They do not qualify takeover during large active compactions with concurrent GC and delayed S3 requests; that remains deployment validation work.

## Provenance

`vendor/slatedb/out` is generated from the complete published `slatedb` 0.17.0 crate by `vendor/slatedb/regenerate.py`, which never touches Cargo's registry. The archive SHA-256 is `543da8a0c19e1a6af286a3c314a1c936af598603f3af98549514fffffb837334`. Its `.cargo_vcs_info.json` identifies upstream commit [`c1e36fc07fb9b759d31484dda18c1c1aad8a2853`](https://github.com/slatedb/slatedb/tree/c1e36fc07fb9b759d31484dda18c1c1aad8a2853), repository subdirectory `slatedb`.

The dependency remains Apache-2.0. The published archive omitted the license file, so `vendor/slatedb/out/LICENSE` is the unmodified [upstream license at that exact commit](https://github.com/slatedb/slatedb/blob/c1e36fc07fb9b759d31484dda18c1c1aad8a2853/LICENSE). Package metadata and original source files remain present. Local modifications are marked in their source headers. This change is `vendor/slatedb/patches/0001-leadership-epoch.patch`; the series is listed in `vendor/slatedb/regenerate.py`.

Upstream already provides the lower-level fixed-epoch primitive: [issue #1095](https://github.com/slatedb/slatedb/issues/1095) proposed synchronizing compactor state with externally selected epochs, and [PR #1152](https://github.com/slatedb/slatedb/pull/1152) added the implementation. This patch exposes that existing primitive through the database builder and its embedded compactor; it does not modify `slatedb-txn-obj`.

Those upstream changes concern internal compactor coordination. They do not establish an upstream-supported external-leadership writer API or a deployment using this exact integration. The public builder option and external lease contract described here are local changes that need their own upstream review.

The primitive itself is the checksum-pinned `slatedb-txn-obj` 0.17.0 dependency in `Cargo.lock`, rather than another local patch. Its [source at the same upstream commit](https://github.com/slatedb/slatedb/blob/c1e36fc07fb9b759d31484dda18c1c1aad8a2853/slatedb-txn-obj/src/lib.rs#L355) was inspected separately: `init_with_epoch` rejects a supplied epoch less than or equal to the stored epoch, writes the unchanged supplied epoch through a version-conditioned transactional update, and refreshes/retries conflicts with that same supplied epoch. It never increments the supplied value. The native tests below exercise this dependency through the actual database builder.

## Changes and contract

- Add `DbBuilder::with_leadership_epoch(NonZeroU64)`. It selects the exact supplied writer epoch and the same epoch for the embedded compactor, including a caller-supplied embedded `CompactorBuilder`. The existing compactions-file fencing already uses the compactor manifest epoch and is preserved.
- Equal or older tokens fail with `ErrorKind::Closed(CloseReason::Fenced)`. CAS retries keep the supplied token. An unsuccessful open can consume its token. A retry or replacement needs a fresh grant; imported databases need a token floor above all persisted writer, compactor-manifest, and compactions epochs.
- Every writer/compactor accessing a managed database must follow the same external token contract. Independent automatic-epoch compactor tools and replacing only an embedded compactor under the same token are unsupported. Omitting the option preserves native automatic allocation.
- Preserve native WAL initialization, create-only WAL fencing, replay, compaction algorithms, data formats, and successful close behavior. Installing the manifest epoch is not completion of WAL fencing: an old WAL append can still win during that interval and must be recovered by the new opener.
- Clean up initialization errors after `DbInner` creation. Before task startup, discard pending handler definitions to break the WAL writer/executor reference cycle. After startup, record the original open error, cancel every task before joining, close without flushing partial replay state, and await the monitor. Return the original error; secondary cleanup errors are logged.
- The caller must retain each whole build/close future, including error cleanup, across cancellation or timeout. Arbitrarily dropping a native build future is not made safe by this patch. Stuck object-store I/O may keep cleanup pending; the adapter owns quarantine and capacity limits. A late successful build must be closed if its grant was revoked.
- Four registry-scoped failpoints expose actual initialization boundaries for integration tests: `leadership-before-writer-fence`, `leadership-after-manifest`, `leadership-before-compactor`, and `leadership-before-replay`. With no configured action, they do nothing. Existing native flush hooks provide close barriers.

## Verification

Run from the repository root:

```sh
cargo check --manifest-path vendor/slatedb/out/Cargo.toml --no-default-features --lib
cargo test --manifest-path vendor/slatedb/out/Cargo.toml --no-default-features --test leadership_lifecycle -- --test-threads=1
cargo test --manifest-path vendor/slatedb/out/Cargo.toml --no-default-features --lib fence:: -- --test-threads=1
```

Results: the library check passed; all **8 new integration tests** and all **8 existing native fencing tests** passed. New tests keep the default embedded compactor enabled and cover:

1. Rejection of stale/equal tokens while the current database remains writable.
2. An old manifest opener delayed until a successor has opened and durably written.
3. An old embedded-compactor opener delayed until a successor has opened and durably written.
4. A durable old WAL append after the successor's manifest epoch change but before its native WAL fence, followed by successful recovery.
5. A close paused during L0 flush while a successor opens and recovers acknowledged data.
6. Initialization failures before and after monitor startup, repeated three times at each boundary; weak store references prove that cleanup released the opening instance.
7. A timed-out but retained replay/open task, a newer successful opener, late completion/cleanup, and release of the delayed instance's distinct store wrapper.
8. Ordinary builder automatic epochs and durable reopen recovery.

The tests use real SlateDB state machines and `object_store::memory::InMemory`, deterministic callback barriers, and tiny key/value data. They do not exercise S3 transport, cloud credentials, or unbounded contention. A cleanup/fenced result is not proof that graceful final flush succeeded; the adapter must classify that outcome separately.

## Packaging and eventual removal

The vendored copy makes the patch reproducible and allows tests against real modified SlateDB without editing the Cargo registry. Vendoring is a packaging choice: a Git dependency pinned to a fork commit could carry the same changes. The leadership design depends on the patch's behavior, not on the source being vendored.

Replace the patch with an upstream version when it provides the equivalent fixed-epoch writer/embedded-compactor contract and initialization cleanup, and the lifecycle regression tests pass against that version.

## Reapply

Run `just slatedb-regenerate` from the repository root. It applies every patch in `vendor/slatedb/patches` in order with zero fuzz to the checksummed 0.17.0 crate and replaces `vendor/slatedb/out`. `just slatedb-check` regenerates into a temporary directory and fails if the result differs from the committed tree.
