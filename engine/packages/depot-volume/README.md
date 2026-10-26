# Depot volume client

`volume_client_machine` owns write ordering, quorum evidence and admission bounds.
The deterministic core retains entry descriptions and payload handles. `Payloads` owns
bytes outside the core and materializes requests through an injected
`VolumeClientTransport`; a peer replies only after its shard writes the request to disk.

Each replica has an independent bounded append window. A slow replica does not
hold up a quorum. Pending writes reserve count and memory byte capacity before
receiving a sequence. Writes waiting behind a full replica window are coalesced
within the encoded request limit. After 100 ms without an append carrying new quorum progress, an explicit
AdvanceCommitted request tells replicas what the client knows is committed.

The test cluster uses three shard state machines and filesystem executors, each
with its own temporary directory. The test driver honors due-alarm ordering and
can hold individual sends without changing the production core.

Opening asks the maintainer to create missing replicas at the cold position,
claims an in-sync quorum. Recovery chooses a source by client epoch before sequence, validates
each replay page, and matches the other replicas using conditional truncation and
copies with original entry epochs. The client serves only after its
own claim entry reaches an in-sync quorum. Close drains accepted writes and
publishes their committed position before releasing through the maintainer.

Unmatched replicas receive one conditional repair operation at a time, with at
most two repairs active per client. Probes step back exponentially and bisect the
first matching bracket. Failed repairs back off without stalling the healthy
quorum. A repairing replica is promoted only after copying to the client's end
using MarkInSync with the repaired incarnation and exact tail.

Writes and block trims carry a stable UUID request identity and a 128-bit xxh3 content
digest. The last 64 mutation identities are rebuilt during replay. Retry with the
returned `WriteToken`: matching content joins the original sequence, conflicting
content fails permanently, and an absent identity has an indeterminate outcome.
A sequence number alone never identifies a write.

Synchronous callers time out after three seconds with their token while the entry
continues replicating. Thirty seconds without quorum progress fences the handle.
`WriteError::disposition()` distinguishes an admission rejection, an unknown
outcome and a permanent error. The commit barrier captures the current end; confirmation waits
for that end and then obtains fresh committed-announcement replies from two distinct peers.

The crash suite kills child processes at every append delivery boundary and at
segment open, write, file sync and directory sync. It also replaces a counted
replica after claiming and verifies that losing two in-sync histories fails
recovery. These tests cover process crashes on real filesystems; they do not model
power loss or torn hardware writes.
