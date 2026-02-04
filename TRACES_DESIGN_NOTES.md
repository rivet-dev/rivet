1. use span[] (make sure to link to docs on this in code)
2. use otlp 1 (link to docs on thi sin code)
3. use raw binary instead of hex/base64 for max compactness
4. we don't need to store resources, since this is written to local disk for a single resource (eg an actor). we'll attach resource data when we export to otlp systems later. make sure this is documented
5. use keyvalue. have the key be in the strings lookup map. have the value be encoded using cbor-x
6. i think we need something more complicated where we store data in each span of all active spans at the beginning of this chunk and which bucket/span it started in. then we can look up that bucket/span manually. do you have any recommendations on how we coudl improve this? how does this affect our read/write system?
7. yes
8. yes, explicit clamped proerty
9. we have a heavy write load and these spans can last months. is this still what you would recommend? give me a few recommendations.

did you get what i said about storing a lookup map for all strings?

# Traces design notes / questions

Primary references (OTLP/JSON schema and structure):

https://opentelemetry.io/docs/specs/otlp/
https://opentelemetry.io/docs/specs/otel/protocol/file-exporter/
https://github.com/open-telemetry/opentelemetry-proto
https://protodoc.io/open-telemetry/opentelemetry-proto/opentelemetry.proto.collector.trace.v1
https://protodoc.io/Helicone/helicone/opentelemetry.proto.trace.v1
https://opentelemetry.io/docs/specs/otel/common/
https://opentelemetry.io/docs/concepts/resources/

---

1) OTLP/JSON “flavors”: what they are
- OTLP/JSON ExportTraceServiceRequest is the canonical OTLP trace payload. It’s the protobuf ExportTraceServiceRequest encoded as JSON (proto3 JSON mapping + OTLP-specific rules). The structure is resourceSpans → scopeSpans → spans.
- A “Span[] only” subset would be a custom format (not standard OTLP), which means any off‑the‑shelf collector won’t accept it. OTLP JSON examples show spans always nested under resource and scope.

Recommendation: use the standard OTLP/JSON envelope for interoperability, even if we store compact internal records and reconstruct on read.

---

2) OTLP versions and available fields
- OTLP trace payloads are defined by the proto schemas in the opentelemetry-proto repo (trace + collector/trace). That’s the authoritative field list.
- High‑level structure: ExportTraceServiceRequest.resourceSpans[] → each has resource + scopeSpans[] → each has scope + spans[].
- Span fields include IDs, timestamps, name/kind, attributes, events, links, status, dropped counts, flags, etc. (see trace.proto via protodoc link).

Recommendation: target “current OTLP v1” (stable) and treat the proto as source‑of‑truth. The OTLP spec is stable for trace signals.

---

3) ID encoding: hex vs base64
- OTLP/JSON explicitly requires hex strings for traceId/spanId (not base64).

Pros:
- Spec‑compliant; matches OTel APIs (hex is the canonical external form).

Cons:
- Larger than binary (hex is 2× size).

Recommendation: use hex strings in JSON output; store internally as bytes for compactness.

---

4) Resource vs scope (instrumentation scope)
- Resource describes the entity producing telemetry (service, host, deployment, etc.).
- Instrumentation scope describes the library that produced the spans (name/version/attributes).
- In OTLP, spans are grouped by resource, then by scope.

Practical difference: resource = “who/where,” scope = “which instrumentation library,” and both are preserved in OTLP JSON.

---

5) “JSON-ish types” vs OTLP AnyValue
OTLP attributes are a list of KeyValue, where the value is AnyValue (a tagged union: string/int/bool/double/bytes/array/map).

So you have two internal options:
- JSON-ish: store arbitrary JSON objects/arrays directly and convert to AnyValue at read time.
- OTLP-style: store AnyValue/KeyValue structures internally and serialize directly.

Given preference for a compact internal schema + string table, a good fit is:
- Internal: compact AnyValue-like union + string table
- External: OTLP/JSON reconstructed from that

---

6) Span lifetime vs time buckets
OTel doesn’t define buckets; spans can start and end at any time. Typically, spans are exported after they end. So you can safely:
- keep open spans in memory,
- write ended spans to disk,
- read hybrid (memory + disk) for queries.

If you need to persist long‑running spans without mutating disk entries, use append‑only records:
- SpanStart, SpanEvent, SpanEnd records (or “SpanDelta”)
- reconstruct on read into OTLP Span

This avoids rewriting chunks.

Question: do you want the append‑only delta model, or is “open spans stay only in memory until end” acceptable (with the risk of losing open spans on crash)?

---

7) emitEvent without span?
In OTLP trace data, events are part of a span (Span.Event). There is no standalone trace event in OTLP JSON.

Options:
- require an active span, or
- create an implicit span (e.g., span name = event name), or
- treat it as a log signal (not part of trace data).

Question: should emitEvent error without an active span, or should it auto‑create a short-lived span?

---

8) Read semantics (default picks)
- Filter by: span startTimeUnixNano
- Range: [start, end) (inclusive start, exclusive end) to avoid double‑count when paginating
- Sort: by startTimeUnixNano, tie-break by traceId/spanId
- Limit: clamp to MAX_LIMIT to avoid runaway allocations; return the actual count
- Mid‑chunk: it’s fine to stop after reaching limit mid‑chunk (we can stream decode and stop early)

Question: do you want an explicit “limit was clamped” signal in the API, or just silently clamp?

---

9) Defaults (proposal)
- Bucket size: 1h
- Chunk target: 512 KiB (keeps well under 1 MiB)
- Flush age: 5–10s (pick 5s for low‑traffic latency)
- Size estimation: encode records into a per‑chunk buffer as they are added; size = buffer length (accurate and simple)

Pending inputs to finalize:
1) append‑only deltas vs only write on end
2) emitEvent behavior without span
3) limit clamp behavior
