# 0004: compaction filters can require their full input

**What it changes.** Adds `CompactionFilterSupplier::requires_full_input` and `complete_input_through`. A filter that requires its full input sees the rows a resumed compaction skips, replayed into a fresh filter before output resumes, and such jobs run as one subcompaction so a range boundary cannot split one key's versions. `complete_input_through` reports the sequence a job provably covers, so a filter drops a bottom tombstone only when every SST of its segment is in the job.

**Needed by.** Depot's version filter (`engine/packages/depot-worker/src/filesystem/filter.rs`), which decides each row from newer versions of the same key. Without this, a resumed compaction could keep an old version whose newer tombstone it already dropped and resurrect deleted data. Tests: `tests/unit/compaction_filter_resume.rs`, `tests/unit/compaction_input_coverage.rs`.

**Upstream.** Nothing equivalent; upstream's filter iterator still forwards `seek`. Candidate for an opt-in upstream API.

**Drop when.** Upstream offers the same opt-in.
