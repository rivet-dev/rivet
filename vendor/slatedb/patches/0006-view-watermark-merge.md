# 0006: L0 watermark merge by view ID

**What it changes.** When the writer merges the compactor's L0 watermark, it matches by view ID, and by SST ID only for manifests without view IDs. Matching both dropped every newer view of the same SST from L0 when the compactor had compacted only the older one: data loss.

**Needed by.** Databases that hold several L0 views of one SST. Depot's earlier multi-range split clones (removed Oct 22) created them, and those children keep them until compaction rewrites their parent's SSTs, so this stays while such databases may exist. New single-range clones do not create them. Test: `test_merge_keeps_newer_views_of_a_partly_compacted_sst`.

**Upstream.** Fixed the same way in PR #2134 (merged after 0.17.0).

**Drop when.** Upgrading to 0.18.
