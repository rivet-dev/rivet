use bytes::Bytes;

/// Extractor for a prefix from a byte string, used to build and probe
/// prefix-based bloom filters and segmented compaction.
///
/// This trait is specific to `BloomFilterPolicy` — it is not part of the core
/// `FilterPolicy`/`FilterBuilder`/`Filter` traits. Custom filter policies that
/// need prefix-based filtering can implement their own logic directly in
/// `add_entry`/`might_match`.
///
/// The extractor's output is always a *prefix* of its input — if
/// `prefix_len(target)` returns `Some(n)`, then the bytes inside `target`
/// sliced to `..n` are the extracted prefix.
///
/// # `PrefixTarget` variants
///
/// The `target` argument distinguishes two different semantic questions:
///
/// - [`PrefixTarget::Point`] — the input is a complete key (a stored key
///   during SST construction, or the target of a point lookup).
///   `prefix_len` returns the extraction length that was / will be hashed
///   into the filter. **Invariant**: if `prefix_len(Point(k)) = Some(n)`,
///   then for every `k'` with `k'[..n] == k[..n]`, also
///   `prefix_len(Point(k')) = Some(n)`. The extraction depends only on
///   the first `n` bytes.
///
/// - [`PrefixTarget::Prefix`] — the input is a scan prefix. `prefix_len`
///   returns `Some(n)` only if probing with the first `n` bytes is safe
///   for *every* possible extension of the input. **Invariant**: if
///   `prefix_len(Prefix(p)) = Some(n)`, then for every extension `q` of
///   `p`, `prefix_len(Point(q)) = Some(n)` and `q[..n] == p[..n]`. When
///   this cannot be guaranteed (e.g., the extractor inspects later bytes
///   of the key, as a "last-delimiter" extractor would), the extractor
///   must return `None` for the `Prefix` variant.
///
/// For most extractors — fixed-length, first-delimiter, anchored-prefix —
/// the two variants return the same answer. The split matters for
/// extractors whose extraction depends on the full input; those can still
/// be used for the build and point paths while conservatively disabling
/// prefix-scan filtering.
pub trait PrefixExtractor: Send + Sync {
    /// A unique name identifying this extractor's configuration.
    ///
    /// Changing the extractor changes which hashes are stored in the
    /// filter, so existing filters become invalid. `BloomFilterPolicy`
    /// includes this name in the policy name it writes to SST metadata.
    fn name(&self) -> &str;

    /// Returns the length `n` such that the bytes of `target` sliced to
    /// `..n` are the extracted prefix, or `None` if this target has no
    /// extractable prefix under this extractor.
    ///
    /// Called on three paths, distinguished by the variant of `target`:
    /// - **Build time** — policy wraps each stored key in
    ///   `PrefixTarget::Point` and hashes `key[..n]` into the filter when
    ///   this returns `Some(n)`.
    /// - **Point reads** — policy forwards the incoming
    ///   `PrefixTarget::Point` and probes with `hash(key[..n])`.
    /// - **Prefix reads** — policy forwards the incoming
    ///   `PrefixTarget::Prefix` and probes with `hash(prefix[..n])`; a
    ///   `None` result causes the filter to be skipped so no false
    ///   negative can occur.
    ///
    /// # Worked example
    ///
    /// A 3-byte fixed extractor with an SST containing keys `abc_1`,
    /// `abc_2`, `abx_1` stores hashes of `abc` and `abx`. Then:
    /// - `Prefix("ab")` → `None` (2 < 3; filter skipped).
    /// - `Prefix("abc")` → `Some(3)` → probe `hash("abc")`.
    /// - `Prefix("abcd")` → `Some(3)` → probe `hash("abc")` (truncation
    ///   safe by the `Prefix` invariant).
    /// - `Point("abc_1")` → `Some(3)` → probe `hash("abc")`.
    ///
    /// For a *last-delimiter* extractor (extract up to and including the
    /// last `:`), the `Prefix` variant must return `None` always — the
    /// last delimiter's position can change as bytes are appended, so no
    /// scan prefix is safe to probe. The `Point` variant still returns
    /// the position correctly for complete keys.
    ///
    /// # Panics
    ///
    /// The returned length must be ≤ the length of the bytes inside
    /// `target`. Returning a longer value is a contract violation and
    /// will panic when the length is used to slice the target bytes.
    fn prefix_len(&self, target: &PrefixTarget) -> Option<usize>;
}

/// The target of a prefix extraction or filter query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefixTarget {
    /// Used to test whether a specific key might exist in the SST.
    Point(Bytes),
    /// Used to test whether any key with the given prefix might exist in the SST.
    Prefix(Bytes),
}
