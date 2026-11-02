# API peer volume reads

API peer serves read-only volume snapshots from the tablet maintainer while a
volume client keeps writing. These are internal routes with no authentication,
token, namespace authorization, or public API forwarding. They use the shared
Depot manifest route and worker identity without acquiring writer ownership.

The Engine mounts this crate's router on the existing `api_peer.host` /
`api_peer.port` listener alongside UPS and Epoxy. Read bounds are configured
through `api_peer.volumes`; there is no separate filesystem HTTP listener or
service. The client is resolved on each request so requests work after Depot
startup installs volume access.

| GET route | Response |
| --- | --- |
| `/volumes/{partition}/{name}/stat/{path}` | JSON inode number and attributes |
| `/volumes/{partition}/{name}/dirs/{path}?cursor=...` | JSON entries and optional next cursor |
| `/volumes/{partition}/{name}/files/{path}` | File bytes; one `Range: bytes=...` returns 206 |
| `/volumes/{partition}/{name}/kv/{path}?key=...` | Raw value bytes, or 404 for an absent key |

Partition and volume names use lowercase hex. `-` names an empty partition or
an unnamed volume; `~` names an explicitly empty volume name. Paths and KV keys
use percent encoding for arbitrary bytes. JSON directory names use lowercase
hex, so non-UTF-8 names round-trip. Omitting the path reads the root. For example,
`/volumes/74656e616e74/-/files/database` reads `database` in the unnamed volume
whose partition bytes spell `tenant`.

Each request keeps one snapshot and its original deadline. Files stream in
4 KiB pieces with backpressure; missing chunks are sparse zeros. Inline files
avoid chunk requests. Open-ended and suffix ranges are supported; unsatisfiable
or multiple ranges return 416 with `Content-Range: bytes */{size}`. A stale or
unsupported `If-Range` validator returns the complete file instead.

A directory cursor retains the original incarnation, session, directory, sequence
and last name. A different HTTP node can continue it. The maintainer enforces the
original session expiry; a later page never renews it. An expired or retired
session returns 409 with `{"error":"TooOld"}`; restart the listing without a
cursor. Completed reads close their sessions. Disconnected requests are bounded
by the original maintainer TTL, and concurrent HTTP work has a separate limit.

Transient routing and apply failures retry within the original request deadline.
Before response headers, an ordinary GET may open a fresh snapshot after a
maintainer change. A directory continuation or an already streaming file never
silently changes its snapshot. A stream that loses its session terminates; retry
the GET. ETags identify the file's incarnation, sequence and inode.

Verification lives in `depot-volume/tests/ups/filesystem_http.rs`, over the real
UPS transport, three disk replicas and SlateDB. It covers concurrent writer
visibility, binary KV, symlinks, chunk and inline ranges, unloaded reads,
snapshot-stable pagination across HTTP nodes and maintainer re-grant.

The regional binary filesystem endpoint is also owned by API peer, in
`src/filesystem.rs`. Its region, deadline, size, and topology lease checks protect
routing and storage authority; they do not authenticate callers.
