# 0000: packaging

**What it changes.** Adds the upstream `LICENSE` that the published 0.17.0 crate omitted (unmodified, from upstream commit `c1e36fc07fb9b759d31484dda18c1c1aad8a2853`), and registers the `leadership_lifecycle` integration test target in `Cargo.toml`.

**Needed by.** Licensing of the vendored copy, and the tests of `0001-leadership-epoch.patch`.

**Drop when.** Never needed upstream; regenerate it for each new release.
