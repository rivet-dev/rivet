# 0005: projected clone verification

**What it changes.** Adds the read-only `VersionedManifest::matches_projected_data_from(source, range)`, which compares a clone's data trees with the projection of its retained source checkpoint.

**Needed by.** `clone_projected` in `engine/packages/env/src/backends/production/slatedb.rs`: a split retry must prove that an existing child is exactly the projection it meant to make before and after cloning. Upstream's retry check confirms lineage but not ranges, and the projection itself is private to SlateDB.

**Upstream.** Candidate for a clone verification API.

**Drop when.** Upstream exposes an equivalent check.
