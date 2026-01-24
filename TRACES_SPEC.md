# @rivetkit/traces — storage + API spec (Span[] storage + OTLP v1 export)

## Goals
- Provide a **generic** tracing library as a new RivetKit package (`@rivetkit/traces`).
- **Store spans compactly** as an internal Span[]-style record stream (append-only deltas + snapshots).
- **Export/read in OTLP v1 JSON** on demand.
- Use **VBARE** for binary encoding of chunk payloads.
- Persist to a **simple KV interface** (get/set/delete/list/batch), same spirit as workflow engine driver.
- **Single writer + single reader** assumed.
- **Do not store Resource** in persistence (single-resource local disk); attach Resource only at export time.
- **No caching** across reads; read combines in-memory (not flushed) and on-disk data as a hybrid.

## Non-goals
- Full OTLP collector implementation.
- Multi-writer concurrency.
- Storage of Resource (explicitly excluded).

## References (embed in code comments / docs)
- OTLP spec (v1): https://opentelemetry.io/docs/specs/otlp/
- OTel trace data model: https://opentelemetry.io/docs/specs/otel/trace/api/
- OTLP proto definitions: https://github.com/open-telemetry/opentelemetry-proto

## Storage model summary
- **Internal storage:** Span[]-style records (append-only), encoded via VBARE.
- **Export format:** OTLP v1 JSON **ExportTraceServiceRequest** at read/export time.
- **IDs:** raw bytes in storage; convert to **hex strings** when exporting OTLP JSON.
- **Attributes:** KeyValue list; key is string-table ID; value is CBOR bytes.
- **Long-running spans:** each chunk includes an `activeSpans` snapshot with start/snapshot pointers.

## Timestamps
- Each chunk stores a **base Unix timestamp** in nanoseconds (`baseUnixNs`).
- All record timestamps are stored as **nanoseconds relative to `baseUnixNs`**.
- Export to OTLP JSON by computing `absoluteNs = baseUnixNs + offsetNs`.
- Read query uses absolute Unix time in milliseconds, `[start_ms, end_ms)` (inclusive start, exclusive end).

### Time source and base computation
We want precise **relative** timing without clock drift between chunks. Use a **single anchor** when the library loads:

- On library init:
  - `anchorUnixMs = Date.now()`
  - `anchorMonoMs = performance.now()` (Node: `performance.now()`; do **not** re-anchor per chunk)
- For each record time (when `timeUnixMs` is not provided):
  - `unixMs = anchorUnixMs + (performance.now() - anchorMonoMs)`
  - `absoluteUnixNs = unixMs * 1_000_000`

Then compute chunk-relative offsets:
- `bucketStartSec = floor(absoluteUnixNs / 1_000_000_000 / bucketSizeSec) * bucketSizeSec`
- `baseUnixNs = bucketStartSec * 1_000_000_000`
- `timeOffsetNs = absoluteUnixNs - baseUnixNs`

**Implementation note:** use `bigint` for `absoluteUnixNs`, `baseUnixNs`, and `timeOffsetNs` to avoid overflow (ns timestamps exceed 2^53). Convert to `number` only for safe millisecond values.

### Snapshot start time
Snapshots store `startTimeUnixNs` as an **absolute** Unix timestamp to avoid negative offsets when a span started before the current chunk base.

## KV key layout (fdb-tuple)
Namespace is already isolated, but we still use a **constant prefix** to leave room for future data types.

```
KEY_PREFIX = {
	DATA: 1,
}
```

### Data chunks
- **Key:** `(KEY_PREFIX.DATA, bucket_start_sec, chunk_id)`
  - `bucket_start_sec` is Unix seconds at bucket boundary (default 1h buckets).
  - `chunk_id` is an incrementing index within a bucket (monotonic, single-writer).

### Start key / snapshot key
- `start_key` = `(KEY_PREFIX.DATA, bucket_start_sec, chunk_id, record_index)`
- `record_index` = 0-based index of the record within the chunk payload.
- Used to hydrate span base state without scanning older buckets.
  - These are **pointers** stored inside chunk payloads (see `activeSpans`), not separate KV entries.

## Records & chunk format (VBARE schema)

### Schema overview
We store chunks that contain a string table and an ordered list of records.

- **Chunk**
  - `baseUnixNs: u64` (chunk base timestamp)
  - `strings: list<str>` (string table; records reference by string ID)
  - `records: list<Record>`
  - `activeSpans: list<ActiveSpanRef>` (snapshot of spans active at chunk flush time)

- **Record**
  - `timeOffsetNs: u64` (record timestamp relative to chunk base)
  - `body: RecordBody` (union)

- **RecordBody** (append-only)
  - `SpanStart`
  - `SpanEvent`
  - `SpanUpdate`
  - `SpanEnd`
  - `SpanSnapshot`

### VBare schema (v1.bare)
```bare
# @rivetkit/traces schema v1

# CBOR-encoded value (cbor-x)
type Cbor data

# Raw IDs (opaque bytes)
type TraceId data   # 16 bytes expected

type SpanId data    # 8 bytes expected

# String table index

type StringId u32

# KeyValue with string table key + CBOR value

type KeyValue struct {
	key: StringId
	value: Cbor
}

# List of key-values

type Attributes list<KeyValue>

# Span status

type SpanStatusCode enum {
	UNSET
	OK
	ERROR
}


type SpanStatus struct {
	code: SpanStatusCode
	message: optional<str>
}

# Event

type SpanEvent struct {
	spanId: SpanId
	name: StringId
	attributes: Attributes
	droppedAttributesCount: u32
}

# SpanEvent note
# - `traceId` is resolved from the span state (start/snapshot), so it is not stored per event.

# Link

type SpanLink struct {
	traceId: TraceId
	spanId: SpanId
	traceState: optional<str>
	attributes: Attributes
	droppedAttributesCount: u32
}

# Start

type SpanStart struct {
	traceId: TraceId
	spanId: SpanId
	parentSpanId: optional<SpanId>
	name: StringId
	kind: u32             # matches OTLP SpanKind enum
	traceState: optional<str>
	flags: u32            # traceFlags
	attributes: Attributes
	droppedAttributesCount: u32
	links: list<SpanLink>
	droppedLinksCount: u32
}

# Update (attributes and/or status changes)

type SpanUpdate struct {
	spanId: SpanId
	attributes: Attributes
	droppedAttributesCount: u32
	status: optional<SpanStatus>
}

# SpanUpdate notes
# - Emitted by `updateSpan`, `setAttributes`, or `setStatus`.
# - `attributes` contains only changed/added keys (delta), not the full attribute set.

# End

type SpanEnd struct {
	spanId: SpanId
	status: optional<SpanStatus>
}

# Snapshot (periodic base state for long spans)

type SpanSnapshot struct {
	traceId: TraceId
	spanId: SpanId
	parentSpanId: optional<SpanId>
	name: StringId
	kind: u32
	startTimeUnixNs: u64    # absolute span start time (nanoseconds since Unix epoch)
	traceState: optional<str>
	flags: u32
	attributes: Attributes
	droppedAttributesCount: u32
	links: list<SpanLink>
	droppedLinksCount: u32
	status: optional<SpanStatus>
}

# Record union

type RecordBody union {
	SpanStart |
	SpanEvent |
	SpanUpdate |
	SpanEnd |
	SpanSnapshot
}

# Record container

type Record struct {
	timeOffsetNs: u64
	body: RecordBody
}

# Record key pointer

type SpanRecordKey struct {
	prefix: u32
	bucketStartSec: u64
	chunkId: u32
	recordIndex: u32
}

# Active span reference (stored in every chunk)

type ActiveSpanRef struct {
	spanId: SpanId
	startKey: SpanRecordKey
	latestSnapshotKey: optional<SpanRecordKey>
}

# Chunk container

type Chunk struct {
	baseUnixNs: u64
	strings: list<str>
	records: list<Record>
	activeSpans: list<ActiveSpanRef>
}
```

## Attribute encoding (KeyValue + CBOR)
- `KeyValue.key` is a `StringId` into the chunk’s string table.
- `KeyValue.value` is CBOR-encoded using `cbor-x`.
- CBOR values map to OTLP AnyValue on export:
  - CBOR string -> AnyValue.stringValue
  - CBOR integer -> AnyValue.intValue
  - CBOR float -> AnyValue.doubleValue
  - CBOR boolean -> AnyValue.boolValue
  - CBOR bytes -> AnyValue.bytesValue (base64 in JSON)
  - CBOR array -> AnyValue.arrayValue (each element recursively)
  - CBOR map -> AnyValue.kvlistValue (keys must be strings)
- If CBOR map keys are not strings, drop with `droppedAttributesCount++`.

## API surface (TypeScript)

### Primary API
```ts
export interface TracesDriver {
	get(key: Uint8Array): Promise<Uint8Array | null>;
	set(key: Uint8Array, value: Uint8Array): Promise<void>;
	delete(key: Uint8Array): Promise<void>;
	deletePrefix(prefix: Uint8Array): Promise<void>;
	list(prefix: Uint8Array): Promise<{ key: Uint8Array; value: Uint8Array }[]>;
	listRange(
		start: Uint8Array,
		end: Uint8Array,
		options?: { reverse?: boolean; limit?: number },
	): Promise<{ key: Uint8Array; value: Uint8Array }[]>;
	batch(writes: { key: Uint8Array; value: Uint8Array }[]): Promise<void>;
}

export interface SpanHandle {
	spanId: Uint8Array;  // 8 bytes
	traceId: Uint8Array; // 16 bytes
	isActive(): boolean;
}

export interface StartSpanOptions {
	parent?: SpanHandle; // optional explicit parent (must be active)
	attributes?: Record<string, unknown>;
	links?: Array<{
		traceId: Uint8Array; // 16 bytes
		spanId: Uint8Array;  // 8 bytes
		traceState?: string;
		attributes?: Record<string, unknown>;
	}>;
	kind?: number; // OTLP SpanKind
	traceState?: string;
	flags?: number;
}

export interface UpdateSpanOptions {
	attributes?: Record<string, unknown>;
	status?: { code: "UNSET" | "OK" | "ERROR"; message?: string };
}

export interface EndSpanOptions {
	status?: { code: "UNSET" | "OK" | "ERROR"; message?: string };
}

export interface EventOptions {
	attributes?: Record<string, unknown>;
	timeUnixMs?: number; // defaults to now (absolute unix ms)
}

export interface ReadRangeOptions {
	startMs: number;
	endMs: number;
	limit: number; // max spans returned
}

export interface ReadRangeResult {
	otlp: OtlpExportTraceServiceRequestJson; // OTLP v1 JSON ExportTraceServiceRequest
	clamped: boolean;                       // true if limit was clamped or results truncated
}

export interface Traces {
	startSpan(name: string, options?: StartSpanOptions): SpanHandle;
	updateSpan(handle: SpanHandle, options: UpdateSpanOptions): void;
	setAttributes(handle: SpanHandle, attributes: Record<string, unknown>): void;
	setStatus(handle: SpanHandle, status: { code: "UNSET" | "OK" | "ERROR"; message?: string }): void;
	endSpan(handle: SpanHandle, options?: EndSpanOptions): void;
	emitEvent(handle: SpanHandle, name: string, options?: EventOptions): void;
	withSpan<T>(handle: SpanHandle, fn: () => T): T;
	getCurrentSpan(): SpanHandle | null;
	flush(): Promise<boolean>;
	readRange(options: ReadRangeOptions): Promise<ReadRangeResult>;
}
```

### Notes
- **No parentSpanId / traceId overrides**. These are generated internally.
- Trace context propagation uses `withSpan` / `getCurrentSpan`. `startSpan` uses the current span as parent (or the explicit `parent` handle) and otherwise creates a new trace root.
- Continuing a remote/external trace is **not supported** in v1 (no traceId injection). This can be added later with an explicit API.
- `emitEvent` **requires an active span**.
- `SpanHandle.isActive()` returns `false` once the span is ended or dropped due to `maxActiveSpans`.
- `readRange` returns OTLP v1 JSON **ExportTraceServiceRequest**, with Resource added by exporter.

### Context propagation
- Use `AsyncLocalStorage` (Node) to store the current span handle.
- `withSpan(handle, fn)` sets the current span for the duration of `fn` (including async continuations).
- `getCurrentSpan()` returns the current span handle or `null`.

## Write path

### In-memory state
- `activeSpans: Map<spanId, SpanState>`
- `currentChunk: { bucketStartSec, chunkId, baseUnixNs, strings, records, sizeBytes, createdAtMonoMs }`
- `activeSpanRefs: Map<spanId, ActiveSpanRef>` (start/snapshot pointers for active spans)
- `maxActiveSpans: number` (hard cap; deepest spans are dropped first)
- `timeAnchor: { unixMs, monoMs }` captured at library init

### Span state (in-memory only)
- `traceId`, `parentSpanId`, `depth`
- `startTimeUnixNs` (absolute, from start record)
- `attributes`, `status`, `links` (current state for snapshots)
- `bytesSinceSnapshot`, `lastSnapshotMonoMs`

### Bucket selection
- `bucketStartSec = floor(absoluteUnixNs / 1_000_000_000 / bucketSizeSec) * bucketSizeSec`

### Flush policy
- Flush chunk if:
  - `sizeBytes >= targetChunkBytes` (default 512 KiB, max < 1 MiB)
  - OR `performance.now() - createdAtMonoMs >= maxChunkAgeMs` (default 5s)
 - Manual `flush()` **does not** write empty chunks; it returns `false` when there are no records.

### Active span cap (depth-based)
- `maxActiveSpans` is a hard cap to prevent unbounded memory growth.
- When the cap is exceeded, **drop spans with the greatest depth** first (keep shallower spans).
- Depth is the count of parent links within the active span set (root = 0).
- Tie-breaker: drop the most recently started spans first.
- Dropped spans stop emitting events/updates/end records (a start record may already exist).

### Pseudocode (write)
```ts
const timeAnchor = {
	unixMs: Date.now(),
	monoMs: performance.now(),
};

function nowUnixMs(): number {
	return timeAnchor.unixMs + (performance.now() - timeAnchor.monoMs);
}

function nowUnixNs(): bigint {
	const unixMs = nowUnixMs();
	const msInt = Math.floor(unixMs);
	const msFrac = unixMs - msInt;
	return BigInt(msInt) * 1_000_000n + BigInt(Math.floor(msFrac * 1_000_000));
}

function computeBucketStartSec(absoluteUnixNs: bigint): number {
	const sec = absoluteUnixNs / 1_000_000_000n;
	const bucket = sec / BigInt(bucketSizeSec);
	return Number(bucket * BigInt(bucketSizeSec));
}

function appendRecord(body: RecordBody, timeUnixMs?: number): { recordIndex: number; encodedBytes: number } {
	const absoluteUnixNs = timeUnixMs != null
		? BigInt(Math.floor(timeUnixMs)) * 1_000_000n
		: nowUnixNs();
	const recordBucketStartSec = computeBucketStartSec(absoluteUnixNs);
	if (recordBucketStartSec !== currentChunk.bucketStartSec) {
		flushChunk();
		resetChunkState(recordBucketStartSec);
	}
	const timeOffsetNs = absoluteUnixNs - currentChunk.baseUnixNs;
	const record = { timeOffsetNs, body };
	const encodedRecord = encodeRecord(record);
	if (currentChunk.sizeBytes + encodedRecord.length > targetChunkBytes) {
		flushChunk();
		resetChunkState(recordBucketStartSec);
	}
	currentChunk.records.push(record);
	currentChunk.sizeBytes += encodedRecord.length;
	return { recordIndex: currentChunk.records.length - 1, encodedBytes: encodedRecord.length };
}

function flushChunk(): boolean {
	if (currentChunk.records.length === 0) return false;
	const chunk = {
		baseUnixNs: currentChunk.baseUnixNs,
		strings: currentChunk.strings,
		records: currentChunk.records,
		activeSpans: [...activeSpanRefs.values()],
	};
	const bytes = encodeChunk(chunk);
	kv.set(buildChunkKey(currentChunk.bucketStartSec, currentChunk.chunkId), bytes);
	return true;
}

function resetChunkState(bucketStartSec: number): void {
	currentChunk.bucketStartSec = bucketStartSec;
	currentChunk.baseUnixNs = BigInt(bucketStartSec) * 1_000_000_000n;
	currentChunk.chunkId = nextChunkId(bucketStartSec);
	currentChunk.createdAtMonoMs = performance.now();
	currentChunk.records = [];
	currentChunk.sizeBytes = 0;
	currentChunk.strings = [];
}

function enforceMaxActiveSpans(): void {
	if (activeSpans.size <= maxActiveSpans) return;
	const candidates = [...activeSpans.values()].sort((a, b) => {
		if (a.depth !== b.depth) return b.depth - a.depth; // deeper first
		return b.startTimeMs - a.startTimeMs; // newest first
	});
	for (const span of candidates) {
		dropSpan(span.spanId);
		if (activeSpans.size <= maxActiveSpans) break;
	}
}

function dropSpan(spanId): void {
	activeSpans.delete(spanId);
	activeSpanRefs.delete(spanId);
}

function startSpan(name, options): SpanHandle {
	const spanId = randomSpanId();
	const parent = options?.parent ?? getCurrentSpan();
	const traceId = parent ? parent.traceId : randomTraceId();
	const parentSpanId = parent ? parent.spanId : null;
	const depth = computeSpanDepth(parentSpanId);
	const body = buildSpanStartRecord(spanId, traceId, name, { ...options, parentSpanId });

	const { recordIndex, encodedBytes } = appendRecord(body);
	const startKey = { prefix: KEY_PREFIX.DATA, bucketStartSec: currentChunk.bucketStartSec, chunkId: currentChunk.chunkId, recordIndex };
	activeSpanRefs.set(spanId, { spanId, startKey, latestSnapshotKey: null });
	activeSpans.set(spanId, initSpanState({
		spanId,
		traceId,
		depth,
		startUnixNs: currentChunk.baseUnixNs + currentChunk.records[recordIndex].timeOffsetNs,
		bytesSinceSnapshot: encodedBytes,
		lastSnapshotMonoMs: performance.now(),
		...,
	}));

	enforceMaxActiveSpans();
	return { spanId, traceId };
}

function emitEvent(handle, name, options) {
	assertActive(handle);
	const body = buildSpanEventRecord(handle, name, options);
	const { encodedBytes } = appendRecord(body, options?.timeUnixMs);
	const state = activeSpans.get(handle.spanId);
	if (state) state.bytesSinceSnapshot += encodedBytes;
	maybeSnapshot(handle.spanId);
}

function updateSpan(handle, options) {
	assertActive(handle);
	const body = buildSpanUpdateRecord(handle, options);
	const { encodedBytes } = appendRecord(body);
	const state = activeSpans.get(handle.spanId);
	if (state) state.bytesSinceSnapshot += encodedBytes;
	maybeSnapshot(handle.spanId);
}

function setAttributes(handle, attributes) {
	return updateSpan(handle, { attributes });
}

function setStatus(handle, status) {
	return updateSpan(handle, { status });
}

function endSpan(handle, options) {
	assertActive(handle);
	const body = buildSpanEndRecord(handle, options);
	appendRecord(body);
	activeSpans.delete(handle.spanId);
	activeSpanRefs.delete(handle.spanId);
}

function maybeSnapshot(spanId) {
	const state = activeSpans.get(spanId);
	if (!state) return;
	if (state.bytesSinceSnapshot >= snapshotBytesThreshold ||
		performance.now() - state.lastSnapshotMonoMs >= snapshotIntervalMs) {
		const snapshot = buildSpanSnapshotRecord(state);
		const { recordIndex } = appendRecord(snapshot);
		const snapshotKey = { prefix: KEY_PREFIX.DATA, bucketStartSec: currentChunk.bucketStartSec, chunkId: currentChunk.chunkId, recordIndex };
		const ref = activeSpanRefs.get(spanId);
		if (ref) ref.latestSnapshotKey = snapshotKey;
		state.bytesSinceSnapshot = 0;
		state.lastSnapshotMonoMs = performance.now();
	}
}
```

## Read path

### Merge strategy
- Read on-disk chunks for `[startMs, endMs)`.
- Read in-memory records (not flushed) whose times fall in range.
- Merge by `recordAbsNs = chunk.baseUnixNs + record.timeOffsetNs` (stable), then reconstruct spans.
- **No caching** across calls.
- `limit` applies to the **number of spans** returned. When the limit is reached, we continue scanning to complete already-selected spans but ignore new span IDs.

### Span hydration
- Find the **previous chunk** immediately before `startMs` (via `listRange(..., reverse: true, limit: 1)`).
- Use that chunk’s `activeSpans` snapshot to hydrate spans that started before the range.
- If no previous chunk exists, only spans that **start within the range** can be reconstructed.
- For each spanId seen in range:
  - If span base not present, look up in `activeSpans` snapshot:
    - If `latestSnapshotKey` exists, read that record as base; else use `startKey`.
  - Apply deltas in the requested range.

### listRange mapping
- `listRangeChunks(startMs, endMs)` is a thin wrapper over `driver.listRange`:
  - `startKey = tuple([KEY_PREFIX.DATA, bucketStartSec(startMs), 0])`
  - `endKey = tuple([KEY_PREFIX.DATA, bucketStartSec(endMs) + bucketSizeSec, 0])`
  - `driver.listRange(startKey, endKey, { reverse: false })`

### Pseudocode (read)
```ts
async function readRange({ startMs, endMs, limit }): Promise<ReadRangeResult> {
	const maxLimit = MAX_LIMIT;
	const limitWasClamped = limit > maxLimit;
	limit = Math.min(limit, maxLimit);

	const previousChunk = await findPreviousChunk(startMs);
	const activeSpanRefs = previousChunk?.activeSpans ?? [];

	const records = [];
	for (const chunk of await listRangeChunks(startMs, endMs)) {
		const decoded = decodeChunk(chunk);
		for (const record of decoded.records) {
			records.push({
				record,
				absNs: decoded.baseUnixNs + record.timeOffsetNs,
			});
		}
	}
	const memRecords = filterInMemoryRecords(startMs, endMs);
	mergeByTime(records, memRecords);

	const spans = new Map();
	let reachedSpanLimit = false;
	for (const entry of records) {
		const record = entry.record;
		const spanId = recordSpanId(record);
		if (!spans.has(spanId)) {
			if (spans.size >= limit) {
				reachedSpanLimit = true;
				continue;
			}
			const ref = activeSpanRefs.find((x) => x.spanId === spanId);
			if (ref) {
				const base = await loadBaseRecord(ref);
				spans.set(spanId, hydrateBase(base));
			} else {
				spans.set(spanId, initSpanFromRecord(record));
			}
		}
		applyDelta(spans.get(spanId), record);
	}

	return {
		otlp: toOtlpExportRequest(spans.values()),
		clamped: limitWasClamped || reachedSpanLimit,
	};
}
```

## Export to OTLP JSON
- Convert internal spans to OTLP JSON **ExportTraceServiceRequest**.
- Attach **Resource** externally at export (not stored).
- Convert:
  - IDs: raw bytes -> hex string
  - Time: `absoluteNs = chunk.baseUnixNs + record.timeOffsetNs`
  - Attributes: CBOR AnyValue -> OTLP AnyValue
- SpanEvent/SpanUpdate/SpanEnd only store `spanId`; `traceId` is resolved from span base state during hydration.
- Grouping into `resourceSpans`/`scopeSpans` is done by exporter (not stored).

## Configuration
- `bucketSizeSec` default: 3600
- `targetChunkBytes` default: 524288
- `maxChunkBytes` hard limit: 1048576
- `maxChunkAgeMs` default: 5000
- `snapshotIntervalMs` default: 300000
- `snapshotBytesThreshold` default: 262144
- `MAX_LIMIT` for reads: 10000 (explicitly clamped)
- `maxActiveSpans` default: 10000 (drop deepest spans first)

## Tests (must cover)

### Encoding / schema
- Encode/decode `Chunk` round-trip with string table and record list.
- Encode/decode each Record type.
- `ActiveSpanRef` encode/decode and record key pointer integrity.
 - `baseUnixNs` + `timeOffsetNs` reconstruction yields correct absolute time.

### Key layout
- fdb-tuple ordering for `(bucketStartSec, chunkId)` is lexicographic and time-ordered.
- Data keys include `KEY_PREFIX.DATA`.

### Write path
- Flush triggered by size and by time.
- Chunk size never exceeds `maxChunkBytes`.
- `flush()` with no records does not write an empty chunk.
- StartSpan writes SpanStart and updates `activeSpans` snapshot.
- updateSpan/setAttributes/setStatus emit SpanUpdate deltas.
- EndSpan appends SpanEnd and removes active span.
- emitEvent writes SpanEvent and requires active span.
- Exceeding `maxActiveSpans` drops the deepest spans first.
- Dropped spans no longer produce events/updates/end records.

### Snapshots
- Snapshot records created on interval/threshold.
- Snapshot checks run after `emitEvent` and `updateSpan` (not on start/end).
- `activeSpans` entries updated with latest snapshot key.
- Read uses snapshot instead of original start when present.

### Read path
- Range query merges disk + memory records in correct order.
- Range with spans that started before range uses previous chunk’s `activeSpans` base hydration.
- Range with no data returns empty.
- Limit clamping sets `clamped = true`.
- Hitting the span limit while more spans exist sets `clamped = true` and excludes new spans beyond the limit.
- Reverse list behavior for `findPreviousChunk` works with `listRange(..., reverse: true, limit: 1)`.
- Records with explicit `timeUnixMs` are placed in correct buckets and sorted by absolute time.

### OTLP export
- IDs converted to hex.
- Timestamps converted to ns.
- CBOR AnyValue -> OTLP AnyValue mapping correctness.
- Resource injected externally; not persisted.

## Documentation notes
- Explicitly document that **Resource is not stored** and must be provided at export time.
- Document that this library stores **Span[]-style data** and converts to OTLP v1 JSON ExportTraceServiceRequest on read/export.
- Include OTLP/OTel links in code comments near the exporter and data model definitions.
